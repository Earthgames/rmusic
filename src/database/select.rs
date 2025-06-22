use std::path::Path;

use super::Library;
use entity::{artist, genre, playlist, playlist_item, publisher, release, track, track_location};
use log::{error, warn};
use sea_orm::prelude::*;
use sea_orm::EntityTrait;

pub type Result<T, E = migration::DbErr> = std::result::Result<T, E>;

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

    pub async fn get_track(&self, path: &Path) -> Result<Option<track::Model>> {
        let path = match path.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                error!(
                    "Error canonicalizing path: {}\nerr: {}",
                    path.display(),
                    err
                );
                return Err(DbErr::Custom("Error canonicalizing path".into()));
            }
        };
        track_location::Entity::find_related()
            .filter(track_location::Column::Path.eq(path.to_str()))
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

pub enum PlaylistItem {
    Track(track::Model),
    Release(release::Model),
    Playlist(playlist::Model),
}

impl Library {
    /// Gets the playlist items from the database in a convenient format.
    ///
    /// A playlist is a recursive format and the places you would use it (UI)
    /// require that you transform it to your own types.
    /// Because of this you need to implement the recursive part yourself
    pub async fn playlist(&self, playlist: playlist::Model) -> Result<Vec<PlaylistItem>> {
        let pl_items = playlist_item::Entity::find()
            .filter(playlist_item::Column::PlaylistId.eq(playlist.id))
            .all(&self.database)
            .await?;

        let mut items = vec![];

        for item in pl_items {
            macro_rules! get_model {
                ($id:expr, $entity:ident, $name:literal) => {
                    match self
                        .check_id::<$entity::Entity>($id, $name, item.id)
                        .await?
                    {
                        Some(model) => model.into(),
                        None => continue,
                    }
                };
            }

            let item = match item.r#type {
                // Track
                0 => get_model!(item.item_track_id, track, "track"),
                // Release
                1 => get_model!(item.item_release_id, release, "release"),
                // Playlist
                2 => get_model!(item.item_playlist_id, playlist, "playlist"),
                _ => {
                    error!(
                        "Wrong type playlist item in database, type:{}, id:{}",
                        item.r#type, item.id
                    );
                    continue;
                }
            };
            // add playlist item to result
            items.push(item)
        }
        Ok(items)
    }

    /// Check id and log problems with it
    async fn check_id<E>(
        &self,
        id: Option<i32>,
        name: &str,
        item_id: i32,
    ) -> Result<Option<E::Model>>
    where
        E: EntityTrait + NormalId,
    {
        let id = match id {
            Some(id) => id,
            None => {
                error!(
                    "PlaylistItem is a {0}, but no {0} id found, id:{1}",
                    name, item_id
                );
                return Ok(None);
            }
        };
        match self.from_id::<E>(id).await? {
            Some(model) => Ok(Some(model)),
            None => {
                warn!(
                    "Couldn't find track {} linked to PlaylistItem {}",
                    item_id, id
                );
                Ok(None)
            }
        }
    }

    pub(crate) async fn from_id<E>(&self, id: i32) -> Result<Option<E::Model>>
    where
        E: EntityTrait + NormalId,
    {
        E::from_id(id).one(&self.database).await
    }
}

macro_rules! impl_into_pli {
    ($($i:ident, $enum:ident), +) => {$(
        impl Into<PlaylistItem> for $i::Model {
            fn into(self) -> PlaylistItem {
                PlaylistItem::$enum(self)
            }
        }
    )+
    };
}
impl_into_pli!(track, Track, release, Release, playlist, Playlist);

trait NormalId: EntityTrait {
    fn from_id(id: i32) -> Select<Self>;
}

macro_rules! impl_normal_id {
    ($($i:ident),+) => {$(
        impl NormalId for $i::Entity {
            fn from_id(id: i32) -> Select<Self> {
                Self::find().filter($i::Column::Id.eq(id))
            }
        }
        )+
    };
}

impl_normal_id!(
    artist,
    genre,
    playlist,
    playlist_item,
    publisher,
    release,
    track
);
