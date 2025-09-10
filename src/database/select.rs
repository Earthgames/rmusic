use std::path::Path;

use crate::{
    models::{Artist, Genre, Playlist, PlaylistItem, Publisher, Release, Track, TrackLocation},
    schema::{artists, playlists, publishers, releases, track_locations, tracks},
    struct_in_enum,
};

use anyhow::{Context, Result};
use log::error;
use log::warn;

use super::{Conn, Library};
use diesel::{associations::HasTable, prelude::*};

pub trait RelatedMany<C> {
    fn models_related(&self, conn: &mut Conn) -> QueryResult<Vec<C>>;
}
pub trait Related<P> {
    fn models_related(&self, conn: &mut Conn) -> QueryResult<Option<P>>;
}
// Implement relations because doing it with generics is pain
macro_rules! impl_related {
    ($($P:ident, $p:ident -> $C:ident),*) => {$(
        // Release -> Vec<Track>
        impl RelatedMany<$C> for $P {
            fn models_related(&self, conn: &mut Conn) -> QueryResult<Vec<$C>> {
                $C::belonging_to(self)
                    .select($C::as_select())
                    .load(conn)
            }
        }
        // Track -> Option<Release>
        impl Related<$P> for $C {
            fn models_related(&self, conn: &mut Conn) -> QueryResult<Option<$P>> {
                // inner join Track, Release. Only Select Release
                $P::table()
                    .inner_join($C::table())
                    .filter(
                        $p::id.eq(
                            diesel::associations::BelongsTo::<$P>::foreign_key(self)
                                .expect("No foreign key found"),
                        ),
                    )
                    .select($P::as_select())
                    // Get first result, or return None
                    .first(conn).optional()
            }
        }
    )*
    };
}

impl_related!(
    Release, releases -> Track,
    Artist, artists -> Track,
    Track, tracks -> Genre,
    Track, tracks -> TrackLocation,
    Artist, artists -> Release,
    Publisher, publishers -> Release,
    Release, releases -> PlaylistItem,
    Track, tracks -> PlaylistItem,
    Playlist, playlists -> PlaylistItem
);

pub trait GetAll: Sized {
    fn get_all(conn: &mut Conn) -> QueryResult<Vec<Self>>;
}

macro_rules! impl_get_all {
    ($($M:ident),*) => {$(
        impl GetAll for $M {
            fn get_all(conn: &mut Conn) -> QueryResult<Vec<Self>> {
                $M::table().select($M::as_select()).load(conn)
            }
        }
    )*
    };
}

impl_get_all!(
    Artist,
    Genre,
    Playlist,
    PlaylistItem,
    Publisher,
    Release,
    Track,
    TrackLocation
);

impl Library {
    pub fn models_related<P, C>(&mut self, model: &P) -> QueryResult<Vec<C>>
    where
        P: RelatedMany<C>,
    {
        P::models_related(model, &mut self.database)
    }

    pub fn model_related<P, C>(&mut self, model: &C) -> QueryResult<Option<P>>
    where
        C: Related<P>,
    {
        C::models_related(model, &mut self.database)
    }

    pub fn get_track(&mut self, path: &Path) -> Result<Option<Track>> {
        let path = match path.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                return Err(err).context("Error canonicalizing path");
            }
        };
        let path = path.to_string_lossy().into_owned();

        Ok(Track::table()
            .inner_join(TrackLocation::table())
            .filter(track_locations::path.eq(&path))
            .select(Track::as_select())
            .first(&mut self.database)
            .optional()?)
    }

    pub fn find_all<E>(&mut self) -> QueryResult<Vec<E>>
    where
        E: GetAll,
    {
        E::get_all(&mut self.database)
    }
}

pub enum PlaylistItemType {
    Track(Track),
    Release(Release),
    Playlist(Playlist),
}
struct_in_enum!(PlaylistItemType, impl_into_pli);
impl_into_pli!(Track: Track, Release: Release, Playlist: Playlist);

impl Library {
    /// Gets the playlist items from the database in a convenient format.
    ///
    /// A playlist is a recursive format and the places you would use it (UI)
    /// require that you transform it to your own types.
    /// Because of this you need to implement the recursive part yourself
    pub fn playlist(&mut self, playlist: &Playlist) -> Result<Vec<PlaylistItemType>> {
        let pl_items = self.models_related::<_, PlaylistItem>(playlist)?;

        let mut items = vec![];

        for item in pl_items {
            // Get model by id and check if it exists, else we continue
            macro_rules! get_model {
                ($id:expr, $entity:ident, $name:literal) => {
                    match self.check_id::<$entity>($id, $name, item.id)? {
                        Some(model) => model.into(),
                        None => continue,
                    }
                };
            }

            let item: PlaylistItemType = match item.item_type {
                // Track
                0 => get_model!(item.item_track_id, Track, "track"),
                // Release
                1 => get_model!(item.item_release_id, Release, "release"),
                // Playlist
                2 => get_model!(item.item_playlist_id, Playlist, "playlist"),
                _ => {
                    error!(
                        "Wrong type playlist item in database, type:{}, id:{}",
                        item.item_type, item.id
                    );
                    continue;
                }
            };
            items.push(item)
        }
        Ok(items)
    }

    /// Check id and log problems with it
    fn check_id<E>(&mut self, id: Option<i32>, name: &str, item_id: i32) -> Result<Option<E>>
    where
        E: NormalId,
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
        match self.model_from_id::<E>(id)? {
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

    #[allow(private_bounds)]
    pub(crate) fn model_from_id<E>(&mut self, id: i32) -> QueryResult<Option<E>>
    where
        E: NormalId,
    {
        E::from_id(id, self)
    }
}

trait NormalId: Sized {
    fn from_id(id: i32, library: &mut Library) -> QueryResult<Option<Self>>;
}

macro_rules! impl_normal_id {
    ($($i:ident),+) => {$(
        impl NormalId for $i {
            fn from_id(id: i32, library: &mut Library) -> QueryResult<Option<Self>> {
                $i::table()
                    .find(id)
                    .first(&mut library.database)
                    .optional()
            }
        }
    )+};
}

impl_normal_id!(
    Artist,
    Genre,
    Playlist,
    PlaylistItem,
    Publisher,
    Release,
    Track
);
