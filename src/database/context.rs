use std::{collections::VecDeque, path::PathBuf};

use entity::{artist, release, track, track_location};
use log::{error, warn};
use sea_orm::{prelude::*, QuerySelect, SelectColumns};

use crate::queue::{QueueItem, QueueOptions};

use super::Library;

pub trait GetContext {
    // get the context when this item is play on its own
    fn get_context(
        &self,
        library: &Library,
    ) -> impl std::future::Future<Output = Result<Option<QueueItem>>> + Send;
    // get the context when this item is played from a list of items
    fn get_context_from_list(
        self_list: Vec<&Self>,
        index: usize,
        library: &Library,
    ) -> impl std::future::Future<Output = Result<Option<QueueItem>>> + Send;
}

impl GetContext for track::Model {
    async fn get_context(&self, library: &Library) -> Result<Option<QueueItem>> {
        library
            .model_related::<_, track_location::Entity>(self)
            .await
            .map(|option| match option {
                Some(location) => Some(QueueItem::Track(PathBuf::from(location.path))),
                None => {
                    warn!("Could not find path for {}", self.name);
                    None
                }
            })
    }

    async fn get_context_from_list(
        self_list: Vec<&Self>,
        index: usize,
        library: &Library,
    ) -> Result<Option<QueueItem>> {
        let mut result = VecDeque::new();
        for track in self_list.into_iter() {
            match track.get_context(library).await? {
                Some(location) => result.push_back(location),
                None => warn!("Could not find path for {}", track.name),
            }
        }
        if index > result.len() {
            error!("index out of bounds while getting context");
            return Err(DbErr::Custom("index out of bounds".into()));
        }
        result.rotate_left(index);
        Ok(Some(QueueItem::PlayList(result, QueueOptions::default())))
    }
}

macro_rules! standerd_list_context {
    () => {
        async fn get_context_from_list(
            self_list: Vec<&Self>,
            index: usize,
            library: &Library,
        ) -> Result<Option<QueueItem>> {
            if let Some(item) = self_list.get(index) {
                item.get_context(library).await
            } else {
                error!("index out of bounds while getting context");
                Err(DbErr::Custom("index out of bounds".into()))
            }
        }
    };
}

impl GetContext for release::Model {
    async fn get_context(&self, library: &Library) -> Result<Option<QueueItem>> {
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
        Ok(Some(QueueItem::Album(result, QueueOptions::default())))
    }
    standerd_list_context!();
}

impl GetContext for artist::Model {
    async fn get_context(&self, library: &Library) -> Result<Option<QueueItem>> {
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
            .collect::<Result<Option<VecDeque<_>>>>()
            .map(|option| option.map(|vec| QueueItem::PlayList(vec, QueueOptions::default())))
    }
    standerd_list_context!();
}

pub type Result<T, E = migration::DbErr> = std::result::Result<T, E>;
