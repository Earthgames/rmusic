use std::{
    collections::VecDeque,
    fmt::Display,
};

use log::{debug, error, info};

use crate::{
    models::{Artist, Playlist, Release, Track},
    queue::queue_items::{FromDB, QueueAlbum, QueueItem, QueuePlaylist, QueueTrack},
};

use super::Library;

pub trait GetContext {
    // Get the context when this item is play on its own
    fn get_context(&self, library: &mut Library) -> TrackResult<QueueItem>;
}

macro_rules! impl_get_context {
    ($($from:ident -> $to:ident),*) => {$(
        impl GetContext for $from
        {
            fn get_context(&self, library: &mut Library) -> TrackResult<QueueItem> {
                $to::from_db(self.clone(), library).map(|m| m.into())
            }
        }
    )*};
}

impl_get_context!(Track -> QueueTrack, Release -> QueueAlbum, Playlist -> QueuePlaylist);

pub trait GetContextList {
    // get the context when this item is played from a list of items
    fn get_context_list(
        self_list: Vec<&Self>,
        index: usize,
        library: &mut Library,
    ) -> TrackResult<QueueItem>;
}

impl GetContextList for Track {
    fn get_context_list(
        self_list: Vec<&Self>,
        index: usize,
        library: &mut Library,
    ) -> TrackResult<QueueItem> {
        debug!("list: {:?}", self_list);
        let mut result = VecDeque::new();
        for track in self_list.into_iter() {
            match track.get_context(library) {
                Ok(location) => result.push_back(location),
                Err(err) => match err {
                    // A track_location missing is not fatal
                    ContextError::NoResult => info!("No track location for: {:?}", track.name),
                    ContextError::DbErr(db_err) => return Err(db_err.into()),
                    // can't happen
                    ContextError::IndexOutOfBounds(_) => {
                        error!("Index out of bounds while getting track")
                    }
                    ContextError::MaxDepthReached => error!("Max depth reached"),
                },
            }
        }
        if index > result.len() {
            return Err(ContextError::IndexOutOfBounds(result.len()));
        }
        result.rotate_left(index);
        Ok(QueuePlaylist::from_items(result).into())
    }
}

impl GetContext for Artist {
    fn get_context(&self, library: &mut Library) -> TrackResult<QueueItem> {
        let albums = library.models_related::<_, Release>(self)?;
        let mut result = VecDeque::new();
        for album in albums {
            match album.get_context(library) {
                Ok(album) => result.push_back(album),
                Err(err) => match err {
                    ContextError::DbErr(db_err) => return Err(db_err.into()),
                    ContextError::IndexOutOfBounds(_) => {
                        error!("Index out of bounds while getting album")
                    }
                    _ => (),
                },
            }
        }
        Ok(QueuePlaylist::from_items(result).into())
    }
}

trait DefaultListContext {}
impl DefaultListContext for Artist {}
impl DefaultListContext for Release {}
impl DefaultListContext for Playlist {}

impl<T> GetContextList for T
where
    T: GetContext + DefaultListContext,
{
    fn get_context_list(
        self_list: Vec<&Self>,
        index: usize,
        library: &mut Library,
    ) -> TrackResult<QueueItem> {
        if let Some(item) = self_list.get(index) {
            item.get_context(library)
        } else {
            Err(ContextError::IndexOutOfBounds(self_list.len()))
        }
    }
}

pub type TrackResult<T, E = ContextError> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum ContextError {
    /// The Depth limit was reached when traversing a playlist
    MaxDepthReached,
    /// There was no result from the database for something that was needed to get the context
    NoResult,
    /// The index that was given is out of bounds,
    /// number is the length of the list that was wrongly indexed
    /// happen
    IndexOutOfBounds(usize),
    /// The database gave an error
    DbErr(diesel::result::Error),
}

impl Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextError::MaxDepthReached => write!(f, "Max Depth Reached"),
            ContextError::NoResult => write!(f, "No context found"),
            ContextError::IndexOutOfBounds(size) => write!(
                f,
                "The index was out of bounds: actual size of list: \"{size}\""
            ),
            ContextError::DbErr(db_err) => write!(f, "DataBase Error: \"{db_err}\""),
        }
    }
}

impl std::error::Error for ContextError {}

impl From<diesel::result::Error> for ContextError {
    fn from(value: diesel::result::Error) -> Self {
        Self::DbErr(value)
    }
}
