use anyhow::{bail, Context, Result};
use entity::{artist, genre, publisher, release, track, track_location};
use log::{debug, info};
use migration::OnConflict;
use sea_orm::prelude::*;
use sea_orm::{EntityTrait, QueryFilter, Set};

use super::Library;

impl Library {
    pub async fn insert_publisher_if_not_exist(
        &self,
        publisher: String,
        about: String,
    ) -> Result<i32> {
        let publisher_data = publisher::ActiveModel {
            id: Default::default(),
            name: Set(publisher.clone()),
            about: Set(about),
        };
        Ok(
            match publisher::Entity::insert(publisher_data)
                .on_conflict_do_nothing()
                .exec(&self.database)
                .await
            {
                Ok(sea_orm::TryInsertResult::Inserted(publisher_insert)) => {
                    info!("Created publisher: {publisher}");
                    publisher_insert.last_insert_id
                }
                Ok(_) | Err(DbErr::Exec(_)) => {
                    match publisher::Entity::find()
                        .filter(publisher::Column::Name.eq(&publisher))
                        .one(&self.database)
                        .await
                        .context("While finding publisher")?
                    {
                        Some(publisher) => publisher.id,
                        None => bail!(
                            "Could not find publisher ({publisher}) after trying to insert it"
                        ),
                    }
                }
                Err(err) => return Err(err).context("While inserting publisher"),
            },
        )
    }

    pub async fn insert_artist_if_not_exist(&self, artist: String, about: String) -> Result<i32> {
        let artist_data = artist::ActiveModel {
            id: Default::default(),
            name: Set(artist.clone()),
            about: Set(about),
        };
        Ok(
            match artist::Entity::insert(artist_data)
                .on_conflict_do_nothing()
                .exec(&self.database)
                .await
            {
                Ok(sea_orm::TryInsertResult::Inserted(artist_insert)) => {
                    info!("Created artist: {}", artist);
                    artist_insert.last_insert_id
                }
                Ok(_) | Err(DbErr::Exec(_)) => {
                    match artist::Entity::find()
                        .filter(artist::Column::Name.eq(&artist))
                        .one(&self.database)
                        .await
                        .context("While finding artist")?
                    {
                        Some(artist) => artist.id,
                        None => bail!("Could not find artist ({artist}) after trying to insert it"),
                    }
                }
                Err(err) => return Err(err).context("While inserting artist"),
            },
        )
    }

    pub async fn insert_track_location_if_not_exist(
        &self,
        path: String,
        track_id: i32,
    ) -> Result<()> {
        let track_location_data = track_location::ActiveModel {
            path: Set(path.clone()),
            track_id: Set(track_id),
        };
        track_location::Entity::insert(track_location_data)
            .on_conflict(
                OnConflict::column(track_location::Column::TrackId)
                    .update_column(track_location::Column::TrackId)
                    .to_owned(),
            )
            .do_nothing()
            .exec(&self.database)
            .await
            .context("While inserting track_location")?;
        info!("Created or updated track_location: {}", path);
        Ok(())
    }

    pub async fn insert_genres_if_not_exist(&self, name: String, track_id: i32) -> Result<()> {
        let genre_data = genre::ActiveModel {
            id: Default::default(),
            name: Set(name.clone()),
            track_id: Set(track_id),
        };
        match genre::Entity::insert(genre_data)
            .do_nothing()
            .exec(&self.database)
            .await
            .context("While inserting genre")?
        {
            sea_orm::TryInsertResult::Empty => (),
            sea_orm::TryInsertResult::Conflicted => debug!("Genre already existed for this track"),
            sea_orm::TryInsertResult::Inserted(_) => {
                info!("Created genre: {}, for tack_id: {}", name, track_id)
            }
        }
        Ok(())
    }

    pub async fn insert_release_if_not_exist(
        &self,
        name: String,
        r#type: Option<String>,
        date: Date,
        artist_id: i32,
        publisher_id: Option<i32>,
    ) -> Result<i32> {
        let release_data = release::ActiveModel {
            id: Default::default(),
            name: Set(name.clone()),
            r#type: Set(r#type.clone()),
            date: Set(date),
            artist_id: Set(artist_id),
            publisher_id: Set(publisher_id),
        };
        Ok(
            match release::Entity::insert(release_data)
                .on_conflict_do_nothing()
                .exec(&self.database)
                .await
            {
                Ok(sea_orm::TryInsertResult::Inserted(release_insert)) => {
                    info!("Created release: {name}");
                    release_insert.last_insert_id
                }
                Ok(_) | Err(DbErr::Exec(_)) => {
                    match release::Entity::find()
                        .filter(release::Column::Name.eq(&name))
                        .filter(match r#type {
                            Some(release_type) => release::Column::Type.eq(release_type),
                            None => release::Column::Type.is_null(),
                        })
                        .filter(release::Column::Date.eq(date))
                        .filter(release::Column::ArtistId.eq(artist_id))
                        .one(&self.database)
                        .await?
                    {
                        Some(release) => release.id,
                        None => {
                            bail!("Could not find release ({name}) after trying to insert it")
                        }
                    }
                }
                Err(err) => bail!("Could not insert release into database: {err}"),
            },
        )
    }

    pub async fn insert_track_if_not_exist(
        &self,
        name: String,
        date: Date,
        number: i32,
        duration: i32,
        artist_id: i32,
        release_id: i32,
    ) -> Result<i32> {
        let track_data = track::ActiveModel {
            name: Set(name.clone()),
            date: Set(date),
            number: Set(number),
            duration: Set(duration),
            artist_id: Set(artist_id),
            release_id: Set(release_id),
            ..Default::default()
        };
        Ok(
            match track::Entity::insert(track_data)
                .on_conflict_do_nothing()
                .exec(&self.database)
                .await
            {
                Ok(sea_orm::TryInsertResult::Inserted(track_insert)) => {
                    info!("Created track {}", name);
                    track_insert.last_insert_id
                }
                Ok(_) | Err(DbErr::Exec(_)) => {
                    match track::Entity::find()
                        .filter(track::Column::Name.eq(&name))
                        .filter(track::Column::Date.eq(date))
                        .filter(track::Column::Number.eq(number))
                        .filter(track::Column::Duration.eq(duration))
                        .filter(track::Column::ArtistId.eq(artist_id))
                        .filter(track::Column::ReleaseId.eq(release_id))
                        .one(&self.database)
                        .await?
                    {
                        Some(track) => track.id,
                        None => bail!("Could not find track ({name}) after trying to insert it"),
                    }
                }
                Err(err) => return Err(err).context("While inserting track"),
            },
        )
    }
}
