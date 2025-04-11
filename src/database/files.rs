use anyhow::{anyhow, bail, Result};
use entity::track_location;
use lofty::{Accessor, ItemKey};
use log::{debug, error, info};
use sea_orm::{entity::ColumnTrait, prelude::Date, ActiveValue, EntityTrait, QueryFilter};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{atomic::AtomicU8, Arc},
    time::Instant,
};

use crate::playback::match_decoder;

use super::{
    date_from_tag, get_tag, multiple_string_from_tag, number_from_tag, parse_date, string_from_tag,
    Library,
};

pub const MEDIAEXTENSIONS: [&str; 4] = ["opus", "mp3", "flac", "wav"];

struct CheckedFile {
    path: PathBuf,
    full_path: String,
}

impl CheckedFile {
    fn new(file: &Path) -> Result<CheckedFile> {
        if !file.is_file() {
            bail!("Not a file")
        }
        let Ok(file) = file.canonicalize() else {
            bail!("Could not canonicalize path to file")
        };
        // we don't use `display`. If we can't get the file name we return error
        let full_path = file.to_str().ok_or(anyhow!("Not a file_name"))?.to_string();
        Ok(CheckedFile {
            path: file,
            full_path,
        })
    }
}
struct MusicFileInsert {
    artist_tag: String,
    title_tag: String,
    album_tag: String,
    album_type_tag: Option<String>,
    album_artist_tag: Option<String>,
    album_date: Date,
    track_number: i32,
    publisher_tag: Option<String>,
    date_tag: Date,
    genres: Vec<String>,
    duration: i32,
    file_location: String,
}

impl MusicFileInsert {
    fn new(file: CheckedFile) -> Result<MusicFileInsert> {
        let tag = get_tag(&file.path)?;

        // Artist Name
        let Some(artist_tag) = tag.artist() else {
            bail!("Could not find artist tag");
        };
        // Song title
        let Some(title_tag) = tag.title() else {
            bail!("Could not find title tag");
        };
        // Album Title
        let Some(album_tag) = tag.album() else {
            bail!("Could not find album tag");
        };
        // Release Type
        let album_type_tag = string_from_tag(&tag, &ItemKey::Unknown("RELEASETYPE".to_string()));
        if album_type_tag.is_none() {
            info!("Could not find release type tag");
        }
        // Track Number
        let track_number = number_from_tag(&tag, &ItemKey::TrackNumber).unwrap_or_else(|err| {
            info!("Could not get track number from tag: {}", err);
            1
        });
        // Track Date
        let date_tag = if let Some(date_tag) = tag.get(&ItemKey::RecordingDate) {
            date_tag
        } else if let Some(year) = tag.get(&ItemKey::Year) {
            year
        } else {
            bail!("Could not find date tag")
        };
        let Some(date_tag) = date_tag.value().text() else {
            bail!("Could not find date tag")
        };
        let Some(date) = parse_date(date_tag) else {
            bail!("Could not find date tag")
        };
        // Album Date
        let album_date = date_from_tag(&tag, &ItemKey::ReleaseDate).unwrap_or_else(|err| {
            info!("Could not get album date from tag: {}", err);
            date
        });
        // Genres
        let genres = multiple_string_from_tag(&tag, &ItemKey::Genre);
        // Album Artist
        let album_artist_tag = string_from_tag(&tag, &ItemKey::AlbumArtist);
        if album_artist_tag.is_none() {
            info!("Could not find album artist tag");
        }

        let Some(decoder) = match_decoder(&file.path) else {
            bail!("Could not get decoder")
        };
        #[allow(clippy::cast_possible_truncation)]
        let duration = (decoder.length() / decoder.sample_rate() as u64) as i32;

        let publisher_tag = string_from_tag(&tag, &ItemKey::Publisher);

        Ok(MusicFileInsert {
            artist_tag: artist_tag.to_string(),
            title_tag: title_tag.to_string(),
            album_tag: album_tag.to_string(),
            album_type_tag,
            track_number,
            date_tag: date,
            album_date,
            genres,
            album_artist_tag,
            duration,
            publisher_tag,
            file_location: file.full_path,
        })
    }
}

impl Library {
    pub async fn add_file(&self, file: &Path) -> Result<()> {
        info!("Adding file: \"{}\"", file.display());
        let checked_file = CheckedFile::new(file)?;
        self.insert_music_file(MusicFileInsert::new(checked_file)?)
            .await?;
        info!("Successfully added file: \"{}\"", file.display());
        Ok(())
    }

    async fn insert_music_file(&self, insert: MusicFileInsert) -> Result<()> {
        // Create artist if needed
        let artist_id = self
            .insert_artist_if_not_exist(insert.artist_tag.clone(), String::new())
            .await?;
        // If album artist does not exist or is the same, use the track artist
        let artist_id_album = match insert.album_artist_tag {
            Some(artist) if artist == insert.artist_tag => artist_id,
            None => artist_id,
            Some(artist) => {
                self.insert_artist_if_not_exist(artist, String::new())
                    .await?
            }
        };
        // Publisher
        let publisher_id = if let Some(publisher) = insert.publisher_tag {
            self.insert_publisher_if_not_exist(publisher, String::new())
                .await
                .ok()
        } else {
            info!("Could not find publisher tag");
            None
        };
        // Create release if needed
        let release_id = self
            .insert_release_if_not_exist(
                insert.album_tag.to_string(),
                insert.album_type_tag,
                insert.album_date,
                artist_id_album,
                publisher_id,
            )
            .await?;
        // Create track if needed
        let track_id = self
            .insert_track_if_not_exist(
                insert.title_tag.to_string(),
                insert.date_tag,
                insert.track_number,
                insert.duration,
                artist_id,
                release_id,
            )
            .await?;
        self.insert_track_location_if_not_exist(insert.file_location.clone(), track_id)
            .await?;
        for genre in insert.genres {
            if self
                .insert_genres_if_not_exist(genre.clone(), track_id)
                .await
                .is_err()
            {
                error!("Could not add genre \"{}\"", genre);
            }
        }
        Ok(())
    }

    pub async fn add_folder_rec(&self, folder: &Path, per_done: &Arc<AtomicU8>) -> Result<()> {
        // check if it is a directory
        if !folder.is_dir() {
            bail!("\"{}\" is not a folder", folder.display());
        }
        info!("Adding folder: \"{}\"", folder.display());
        per_done.store(0, std::sync::atomic::Ordering::Relaxed);
        let now = Instant::now();
        // takes no time at all
        let files = Self::find_files(folder)?;
        let total = files.len();
        let mut progress = 0;

        let mut filter_files = vec![];
        // should also take no time
        for file in files {
            let checked_file = match CheckedFile::new(&file) {
                Ok(file) => file,
                Err(_) => continue,
            };
            match track_location::Entity::find()
                .filter(track_location::Column::Path.eq(&checked_file.full_path))
                .one(&self.database)
                .await
            {
                Ok(Some(_)) => debug!(
                    "file was already in database: {}",
                    checked_file.path.display()
                ),
                Ok(None) => filter_files.push(checked_file),
                Err(err) => bail!("Err while selecting from database {}", err),
            };
        }

        // about 10% of the total time
        let mut inserts = vec![];
        for (i, file) in filter_files.into_iter().enumerate() {
            match MusicFileInsert::new(file) {
                Err(err) => {
                    error!("Error getting tags of file: {}", err);
                }
                Ok(insert) => inserts.push(insert),
            }
            let progress_amount = ((i + 1) / total) as u8 / 10;
            if progress_amount > progress {
                progress = progress_amount;
                per_done.store(progress_amount, std::sync::atomic::Ordering::Relaxed);
            }
        }

        // Make sure everything exist without pinging the db to many times
        let mut artist_map: HashMap<String, i32> = HashMap::new();
        let mut publisher_map: HashMap<String, i32> = HashMap::new();
        let mut release_map: HashMap<(String, i32, Date), i32> = HashMap::new();

        let mut track_locations = vec![];
        for (i, insert) in inserts.into_iter().enumerate() {
            let artist_id = if let Some(id) = artist_map.get(&insert.artist_tag) {
                *id
            } else {
                let id = self
                    .insert_artist_if_not_exist(insert.artist_tag.clone(), String::new())
                    .await?;
                artist_map.insert(insert.artist_tag.clone(), id);
                id
            };
            let artist_id_album = match &insert.album_artist_tag {
                Some(artist) if artist == &insert.artist_tag => artist_id,
                None => artist_id,
                Some(artist) => {
                    if let Some(id) = artist_map.get(artist) {
                        *id
                    } else {
                        let id = self
                            .insert_artist_if_not_exist(artist.clone(), String::new())
                            .await?;
                        artist_map.insert(insert.artist_tag.clone(), id);
                        id
                    }
                }
            };
            let publisher_id = match &insert.publisher_tag {
                Some(publisher) => {
                    if let Some(id) = publisher_map.get(publisher) {
                        Some(*id)
                    } else {
                        let id = self
                            .insert_publisher_if_not_exist(publisher.clone(), String::new())
                            .await?;
                        publisher_map.insert(publisher.clone(), id);
                        Some(id)
                    }
                }
                None => None,
            };

            let release_id = if let Some(id) =
                release_map.get(&(insert.album_tag.clone(), artist_id_album, insert.album_date))
            {
                *id
            } else {
                let id = self
                    .insert_release_if_not_exist(
                        insert.album_tag.clone(),
                        insert.album_artist_tag.clone(),
                        insert.album_date,
                        artist_id_album,
                        publisher_id,
                    )
                    .await?;
                release_map.insert(
                    (insert.album_tag.clone(), artist_id_album, insert.album_date),
                    id,
                );
                id
            };
            let track_id = self
                .insert_track_if_not_exist(
                    insert.title_tag.to_string(),
                    insert.date_tag,
                    insert.track_number,
                    insert.duration,
                    artist_id,
                    release_id,
                )
                .await?;
            track_locations.push(track_location::ActiveModel {
                path: ActiveValue::Set(insert.file_location),
                track_id: ActiveValue::Set(track_id),
            });
            let progress_amount = ((i + 1) / total) as u8 / 89 + 10;
            if progress_amount > progress {
                progress = progress_amount;
                per_done.store(progress_amount, std::sync::atomic::Ordering::Relaxed);
            }
        }
        // insert tracks_locations
        track_location::Entity::insert_many(track_locations)
            .exec(&self.database)
            .await?;
        per_done.store(100, std::sync::atomic::Ordering::Relaxed);

        info!(target: "rmusic::speed", "Addin files took {} sec", now.elapsed().as_secs());
        info!("Successfully added folder: \"{}\"", folder.display());
        Ok(())
    }

    fn find_files(folder: &Path) -> Result<Vec<PathBuf>> {
        if !folder.is_dir() {
            bail!("\"{}\" is not a folder", folder.display());
        }
        let mut result = vec![];

        for item in folder.read_dir()? {
            let item = item?.path();
            if item.is_dir() {
                result.append(&mut Self::find_files(&item.clone())?);
            } else if item.extension().is_some_and(|x| {
                MEDIAEXTENSIONS.contains(&x.to_string_lossy().into_owned().as_str())
            }) {
                result.push(item.clone());
            }
        }
        Ok(result)
    }
}
