use std::{collections::VecDeque, path::PathBuf};

use entity::{artist, release, track, track_location};
use log::{error, warn};
use sea_orm::prelude::*;

use crate::queue::{QueueItem, QueueOptions};

use super::Library;

pub trait GetContext {
    // get the context when this item is play on its own
    fn get_context(
        &self,
        library: &Library,
    ) -> impl std::future::Future<Output = TrackResult<QueueItem>> + Send;
    // get the context when this item is played from a list of items
    fn get_context_from_list(
        self_list: Vec<&Self>,
        index: usize,
        library: &Library,
    ) -> impl std::future::Future<Output = TrackResult<QueueItem>> + Send;
}

impl GetContext for track::Model {
    async fn get_context(&self, library: &Library) -> TrackResult<QueueItem> {
        match library
            .model_related::<_, track_location::Entity>(self)
            .await?
        {
            Some(location) => Ok(QueueItem::Track(PathBuf::from(location.path))),
            None => {
                warn!("Could not find path for {}", self.name);
                Err(TrackError::NoTrackLocation)
            }
        }
    }

    async fn get_context_from_list(
        self_list: Vec<&Self>,
        index: usize,
        library: &Library,
    ) -> TrackResult<QueueItem> {
        let mut result = VecDeque::new();
        for track in self_list.into_iter() {
            match track.get_context(library).await {
                Ok(location) => result.push_back(location),
                Err(err) => match err {
                    // a track_location missing is not fatal
                    TrackError::NoTrackLocation => (),
                    TrackError::DbErr(db_err) => return Err(db_err.into()),
                    // can't happen
                    TrackError::WrongInput(err) => error!("WrongInput: {err}"),
                },
            }
        }
        if index > result.len() {
            error!("index out of bounds while getting context");
            return Err(TrackError::WrongInput("index out of bounds".into()));
        }
        result.rotate_left(index);
        Ok(QueueItem::PlayList(result, QueueOptions::default()))
    }
}

macro_rules! standerd_list_context {
    () => {
        async fn get_context_from_list(
            self_list: Vec<&Self>,
            index: usize,
            library: &Library,
        ) -> TrackResult<QueueItem> {
            if let Some(item) = self_list.get(index) {
                item.get_context(library).await
            } else {
                error!("index out of bounds while getting context");
                Err(TrackError::WrongInput("index out of bounds".into()))
            }
        }
    };
}

impl GetContext for release::Model {
    async fn get_context(&self, library: &Library) -> TrackResult<QueueItem> {
        let tracks = self
            .find_related(track::Entity)
            .find_also_related(track_location::Entity)
            .all(&library.database)
            .await?;
        let mut result = VecDeque::new();
        for (track, option) in tracks {
            match option {
                Some(location) => result.push_back(PathBuf::from(location.path)),
                None => warn!("Could not find path for {}", track.name),
            }
        }
        Ok(QueueItem::Album(result, QueueOptions::default()))
    }
    standerd_list_context!();
}

impl GetContext for artist::Model {
    async fn get_context(&self, library: &Library) -> TrackResult<QueueItem> {
        let discography = self
            .find_related(release::Entity)
            .all(&library.database)
            .await?;
        let mut result = vec![];
        for release in discography.into_iter() {
            result.push(release.get_context(library).await);
        }
        result
            .into_iter()
            .collect::<TrackResult<VecDeque<_>>>()
            .map(|vec| QueueItem::PlayList(vec, QueueOptions::default()))
    }
    standerd_list_context!();
}

pub type TrackResult<T, E = TrackError> = std::result::Result<T, E>;

pub enum TrackError {
    /// There was no track_location for this track in the database
    NoTrackLocation,
    /// The input was faulty
    WrongInput(String),
    /// The database gave an error
    DbErr(migration::DbErr),
}

impl From<migration::DbErr> for TrackError {
    fn from(value: migration::DbErr) -> Self {
        Self::DbErr(value)
    }
}
