use super::Library;
use entity::{artist, release, track};
use sea_orm::prelude::*;
use sea_orm::EntityTrait;

impl Library {
    pub async fn models_related<M, R>(&self, model: &M) -> Result<Vec<R::Model>>
    where
        M: ModelTrait,
        R: EntityTrait,
        M::Entity: Related<R>,
    {
        model
            .find_related::<R>(R::default())
            .all(&self.database)
            .await
    }

    pub async fn model_related<M, R>(&self, model: &M) -> Result<Option<R::Model>>
    where
        M: ModelTrait,
        R: EntityTrait,
        M::Entity: Related<R>,
    {
        model
            .find_related::<R>(R::default())
            .one(&self.database)
            .await
    }

    pub async fn find_all<E>(&self) -> Result<Vec<E::Model>>
    where
        E: EntityTrait,
    {
        E::find().all(&self.database).await
    }

    pub async fn artist_discography(
        &self,
        artist: &artist::Model,
    ) -> Result<Vec<(release::Model, Vec<track::Model>)>> {
        artist
            .find_related(release::Entity)
            .find_with_related(track::Entity)
            .all(&self.database)
            .await
    }
}

pub type Result<T, E = migration::DbErr> = std::result::Result<T, E>;
