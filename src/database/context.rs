use std::{collections::VecDeque, future::Future, path::PathBuf, process::Output};

use entity::{artist, release, track, track_location};
use sea_orm::{prelude::*, QuerySelect, SelectColumns};

use crate::queue::{QueueItem, QueueOptions};

use super::Library;

pub trait GetContext {
    fn get_context(
        &self,
        library: &Library,
    ) -> impl std::future::Future<Output = Result<Option<QueueItem>>> + Send;
}

impl GetContext for track::Model {
    async fn get_context(&self, library: &Library) -> Result<Option<QueueItem>> {
        library
            .model_related::<_, track_location::Entity>(self)
            .await
            .map(|option| option.map(|location| QueueItem::Track(PathBuf::from(location.path))))
    }
}

impl GetContext for release::Model {
    async fn get_context(&self, library: &Library) -> Result<Option<QueueItem>> {
        self.find_related(track::Entity)
            .find_also_related(track_location::Entity)
            .select_only()
            .select_column(track_location::Column::Path)
            .all(&library.database)
            .await
            .map(|db_result| {
                db_result
                    .into_iter()
                    .map(|(_, option)| option.map(|x| PathBuf::from(x.path)))
                    .collect::<Option<VecDeque<_>>>()
                    .map(|vec| QueueItem::Album(vec, QueueOptions::default()))
            })
    }
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
}

pub type Result<T, E = migration::DbErr> = std::result::Result<T, E>;
