use super::Library;
use entity::{artist, release, track};
use sea_orm::prelude::*;
use sea_orm::EntityTrait;

impl Library {
    pub async fn artists(&self) -> Result<Vec<artist::Model>> {
        artist::Entity::find().all(&self.database).await
    }

    async fn artist_tracks(&self, artist: &artist::Model) -> Result<Vec<track::Model>> {
        artist.find_related(track::Entity).all(&self.database).await
    }

    pub async fn artist_releases(&self, artist: &artist::Model) -> Result<Vec<release::Model>> {
        artist
            .find_related(release::Entity)
            .all(&self.database)
            .await
    }

    pub async fn release_tracks(&self, release: &release::Model) -> Result<Vec<track::Model>> {
        release
            .find_related(track::Entity)
            .all(&self.database)
            .await
    }

    pub async fn artist_discography(
        &self,
        artist: &artist::Model,
    ) -> Result<Vec<(release::Model, Vec<track::Model>)>> {
        let releases = self.artist_releases(artist).await?;
        let mut release_tracks = vec![];
        for release in &releases {
            release_tracks.push(self.release_tracks(release).await?);
        }
        Ok(releases.into_iter().zip(release_tracks).collect())
    }
}

pub type Result<T, E = migration::DbErr> = std::result::Result<T, E>;
