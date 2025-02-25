use anyhow::{bail, Result};
use entity::{artist, genre, publisher, release, track, track_location};
use log::info;
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
            match publisher::Entity::find()
                .filter(publisher::Column::Name.eq(&publisher))
                .one(&self.database)
                .await?
            {
                Some(publisher) => publisher.id,
                None => match publisher::Entity::insert(publisher_data)
                    .exec(&self.database)
                    .await
                {
                    Ok(publisher_insert) => {
                        info!("Created publisher: {publisher}");
                        publisher_insert.last_insert_id
                    }
                    Err(err) => bail!("Could not insert publisher into database: {err}"),
                },
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
            match artist::Entity::find()
                .filter(artist::Column::Name.eq(&artist))
                .one(&self.database)
                .await
                .expect("could not fetch artist")
            {
                Some(artist) => artist.id,
                None => match artist::Entity::insert(artist_data)
                    .exec(&self.database)
                    .await
                {
                    Ok(artist_insert) => {
                        info!("Created artist: {}", artist);
                        artist_insert.last_insert_id
                    }
                    Err(err) => bail!("Could not insert artist into database: {err}"),
                },
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
        match track_location::Entity::find()
            .filter(track_location::Column::Path.eq(&path))
            .one(&self.database)
            .await?
        {
            Some(track_location) => {
                // Update the database if there is a different track_id
                if track_location.track_id != track_id {
                    let mut track_location = <track_location::ActiveModel>::from(track_location);
                    track_location.track_id = Set(track_id);
                    track_location.update(&self.database).await?;
                }
            }
            None => match track_location::Entity::insert(track_location_data)
                .exec(&self.database)
                .await
            {
                Ok(_) => info!("Created track_location: {}", path),
                Err(err) => bail!("Could not insert track location into database: {err}"),
            },
        };
        Ok(())
    }

    pub async fn insert_genres_if_not_exist(&self, name: String, track_id: i32) -> Result<()> {
        let genre_data = genre::ActiveModel {
            id: Default::default(),
            name: Set(name.clone()),
            track_id: Set(track_id),
        };
        match genre::Entity::find()
            .filter(genre::Column::Name.eq(&name))
            .filter(genre::Column::TrackId.eq(track_id))
            .one(&self.database)
            .await?
        {
            // If it is already there we don't do anything
            Some(_) => return Ok(()),
            None => match genre::Entity::insert(genre_data).exec(&self.database).await {
                Ok(_) => info!("Created genre: {}, for tack_id: {}", name, track_id),
                Err(err) => bail!("Could not insert track location into database: {err}"),
            },
        };
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
            match release::Entity::find()
                .filter(release::Column::Type.eq(r#type))
                .filter(release::Column::Name.eq(&name))
                .filter(release::Column::Date.eq(date))
                .filter(release::Column::ArtistId.eq(artist_id))
                .one(&self.database)
                .await?
            {
                Some(release) => release.id,
                None => match release::Entity::insert(release_data)
                    .exec(&self.database)
                    .await
                {
                    Ok(release_insert) => {
                        info!("Created release: {name}");
                        release_insert.last_insert_id
                    }
                    Err(err) => bail!("Could not insert release into database: {err}"),
                },
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
                None => match track::Entity::insert(track_data).exec(&self.database).await {
                    Ok(track_insert) => {
                        info!("Created track {}", name);
                        track_insert.last_insert_id
                    }
                    Err(err) => bail!("Could not insert track into database: {err}"),
                },
            },
        )
    }
}
