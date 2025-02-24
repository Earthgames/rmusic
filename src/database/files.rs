use anyhow::{anyhow, bail, Result};
use lofty::{Accessor, ItemKey};
use log::{info, warn};
use std::path::Path;

use crate::playback::match_decoder;

use super::{
    date_from_tag, get_tag, multiple_string_from_tag, number_from_tag, parse_date, string_from_tag,
    Library,
};

pub const MEDIAEXTENSIONS: [&str; 4] = ["opus", "mp3", "flac", "wav"];

impl Library {
    pub async fn add_file(&self, file: &Path) -> Result<()> {
        if !file.is_file() {
            bail!("Not a file")
        }
        let Ok(file) = file.canonicalize() else {
            bail!("Could not canonicalize path to file")
        };
        // we don't use `display`. If we can't get the file name we return error
        info!(
            "Adding file: \"{}\"",
            file.file_name()
                .ok_or(anyhow!("Not a file_name"))?
                .to_string_lossy()
        );
        let tag = get_tag(&file)?;

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
            warn!("Could not find release type tag");
        }
        // Track Number
        let track_number = number_from_tag(&tag, &ItemKey::TrackNumber).unwrap_or_else(|err| {
            warn!("Could not get track number from tag: {}", err);
            1
        });
        // Track Date
        let Some(date_tag) = tag.get(&ItemKey::RecordingDate) else {
            bail!("Could not find date tag")
        };
        let Some(date_tag) = date_tag.clone().into_value().into_string() else {
            bail!("Could not find date tag")
        };
        let Some(date) = parse_date(&date_tag) else {
            bail!("Could not find date tag")
        };
        // Album Date
        let album_date = date_from_tag(&tag, &ItemKey::ReleaseDate).unwrap_or_else(|err| {
            warn!("Could not get album date from tag: {}", err);
            date
        });
        // Genres
        let genres = multiple_string_from_tag(&tag, &ItemKey::Genre);
        // Album Artist
        let album_artist_tag = string_from_tag(&tag, &ItemKey::AlbumArtist);
        if album_artist_tag.is_none() {
            warn!("Could not find album artist tag");
        }

        let Some(decoder) = match_decoder(&file) else {
            bail!("Could not get decoder")
        };
        let duration = (decoder.length() / decoder.sample_rate() as u64) as i32;

        // Create artist if needed
        let artist_id = self
            .insert_artist_if_not_exist(artist_tag.to_string(), "".to_string())
            .await?;
        // If album artist does not exist or is the same, use the track artist
        let artist_id_album = match album_artist_tag {
            Some(artist) if artist == artist_tag => artist_id,
            None => artist_id,
            Some(artist) => {
                self.insert_artist_if_not_exist(artist, "".to_string())
                    .await?
            }
        };
        // Publisher
        let publisher_id = if let Some(publisher) = string_from_tag(&tag, &ItemKey::Publisher) {
            self.insert_publisher_if_not_exist(publisher, "".to_string())
                .await
                .ok()
        } else {
            warn!("Could not find publisher tag");
            None
        };
        // Create release if needed
        let release_id = self
            .insert_release_if_not_exist(
                album_tag.to_string(),
                album_type_tag,
                album_date,
                artist_id_album,
                publisher_id,
            )
            .await?;
        // Create track if needed
        let track_id = self
            .insert_track_if_not_exist(
                title_tag.to_string(),
                date,
                track_number,
                duration,
                artist_id,
                release_id,
            )
            .await?;
        self.insert_track_location_if_not_exist(file.display().to_string(), track_id)
            .await?;
        for genre in genres {
            if self
                .insert_genres_if_not_exist(genre.clone(), track_id)
                .await
                .is_err()
            {
                warn!("Could not add genre \"{}\"", genre)
            }
        }
        info!("Successfully added file: \"{}\"", file.display());
        Ok(())
    }

    pub async fn add_folder_rec(&self, folder: &Path) -> Result<()> {
        // check if it is a directory
        if !folder.is_dir() {
            bail!("\"{}\" is not a folder", folder.display());
        }
        info!("Adding folder: \"{}\"", folder.display());

        for item in folder.read_dir()? {
            let item = item?.path();
            if item.is_dir() {
                Box::pin(self.add_folder_rec(&item)).await?;
            } else if item.extension().is_some_and(|x| {
                MEDIAEXTENSIONS.contains(&x.to_string_lossy().into_owned().as_str())
            }) {
                if let Err(err) = self.add_file(&item).await {
                    warn!(
                        "Error while adding file: \"{}\"\nError:{err}",
                        item.display()
                    )
                }
            }
        }
        info!("Successfully added folder: \"{}\"", folder.display());
        Ok(())
    }
}
