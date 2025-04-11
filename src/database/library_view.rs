use crate::queue::QueueItem;

use super::context::{GetContext, TrackError, TrackResult};
use super::Library;
use entity::{artist, release, track};
use entity::{genre, publisher};
use log::info;
use sea_orm::{EntityTrait, ModelTrait, Related};
use std::fmt::Debug;
use tokio::time::Instant;

/// Hate all the traits?
/// Me to, if you find a solution please let me know
///
/// Please read https://stackoverflow.com/questions/68728105/how-can-i-generate-trait-bounds-in-a-declarative-macro
/// or use
/// ```rust
/// where
///     A: L1<B, C>,
///     B: L2<C>,
///     C: L3,
/// ```
pub struct LibraryView<A, B, C>
where
    A: L1<B, C>,
    B: L2<C>,
    C: L3,
{
    list: Level1<A, B, C>,
}

pub type Level1<A, B, C> = Vec<(A, Option<Level2<B, C>>)>;
pub type Level2<B, C> = Vec<(B, Option<Vec<C>>)>;

pub trait IntoFR<T> {
    fn into(self) -> T;
}

macro_rules! impl_bs {
    ($($i:ident),+) => {$(
        impl IntoFR<<<$i::Model as ModelTrait>::Entity as EntityTrait>::Model> for $i::Model {
            fn into(self) -> Self {
                self
            }
        }
        )+
    };
}

impl_bs!(artist, release, track, publisher, genre);

pub trait L1<A, B>: Sized + Sync + Debug + GetContext + Clone
where
    A: L2<B>,
    B: L3,
{
    fn get_all(library: &Library) -> impl std::future::Future<Output = Result<Vec<Self>>> + Send;
    fn get_l2(&self, library: &Library)
        -> impl std::future::Future<Output = Result<Vec<A>>> + Send;
}

pub trait L2<A>: Debug + GetContext + Clone
where
    A: L3,
{
    fn get_l3(&self, library: &Library)
        -> impl std::future::Future<Output = Result<Vec<A>>> + Send;
}

pub trait L3: ModelTrait + Debug + GetContext + Clone {}

impl<A, B, C> L1<B, C> for A
where
    A: ModelTrait + Sync + GetContext,
    <<A as ModelTrait>::Entity as EntityTrait>::Model: IntoFR<A>,
    A::Entity: Related<B::Entity>,
    B: L2<C> + ModelTrait,
    <<B as ModelTrait>::Entity as EntityTrait>::Model: IntoFR<B>,
    C: L3,
{
    async fn get_all(library: &Library) -> Result<Vec<Self>> {
        library
            .find_all::<A::Entity>()
            .await
            .map(|x| x.into_iter().map(|x| IntoFR::into(x)).collect())
    }

    async fn get_l2(&self, library: &Library) -> Result<Vec<B>> {
        library
            .models_related(self)
            .await
            .map(|x| x.into_iter().map(|z| IntoFR::into(z)).collect())
    }
}

impl<A, B> L2<B> for A
where
    A: ModelTrait + std::marker::Sync + GetContext,
    B: L3 + GetContext,
    <<B as ModelTrait>::Entity as EntityTrait>::Model: IntoFR<B>,
    A::Entity: Related<B::Entity>,
{
    async fn get_l3(&self, library: &Library) -> Result<std::vec::Vec<B>, migration::DbErr> {
        library
            .models_related::<_, B::Entity>(self)
            .await
            .map(|x| x.into_iter().map(|z| IntoFR::into(z)).collect())
    }
}

impl<A> L3 for A where A: ModelTrait + GetContext {}

impl<A, B, C> Default for LibraryView<A, B, C>
where
    A: L1<B, C>,
    B: L2<C>,
    C: L3,
{
    fn default() -> Self {
        Self { list: vec![] }
    }
}

impl<A, B, C> LibraryView<A, B, C>
where
    A: L1<B, C>,
    B: L2<C>,
    C: L3,
{
    pub async fn new(library: &Library) -> Result<LibraryView<A, B, C>> {
        let mut view_thing = LibraryView { list: vec![] };
        view_thing.sync_with_database_l1(library).await?;
        Ok(view_thing)
    }
    /// Sync level 1 with database, A in type definition
    pub async fn sync_with_database_l1(&mut self, library: &Library) -> Result<()> {
        self.list = A::get_all(library)
            .await?
            .into_iter()
            .map(|x| (x, None))
            .collect();
        Ok(())
    }
    /// Sync all level 2 with database, B in type definition
    pub async fn sync_with_database_l2(&mut self, library: &Library) -> Result<()> {
        for item_l1 in self.list.iter_mut() {
            if item_l1.1.is_none() {
                let item_list = item_l1
                    .0
                    .get_l2(library)
                    .await?
                    .into_iter()
                    .map(|x| (x, None))
                    .collect();
                item_l1.1 = Some(item_list);
            }
        }
        Ok(())
    }
    /// Sync all level 3 with database, C in type definition
    /// Does nothing if lever 2 is not synced with database at least once
    pub async fn sync_with_database_l3(&mut self, library: &Library) -> Result<()> {
        for item_l1 in self.list.iter_mut() {
            if let Some(list) = &mut item_l1.1 {
                for item_l2 in list.iter_mut() {
                    if item_l2.1.is_none() {
                        let item_list = item_l2.0.get_l3(library).await?;
                        item_l2.1 = Some(item_list);
                    }
                }
            }
        }
        Ok(())
    }

    /// Sync a specific item from level 2, B in the type definition
    pub async fn sync_with_database_l2_item(
        &mut self,
        library: &Library,
        item: usize,
    ) -> Result<()> {
        let item_l1 = &mut self.list[item];
        if item_l1.1.is_none() {
            let itemlist = item_l1
                .0
                .get_l2(library)
                .await?
                .into_iter()
                .map(|x| (x, None))
                .collect();
            item_l1.1 = Some(itemlist);
        }
        Ok(())
    }

    /// Sync a specific item from level 3, C in the type definition
    /// item is (level 1 index, level 2 index)
    pub async fn sync_with_database_l3_item(
        &mut self,
        library: &Library,
        item: (usize, usize),
    ) -> Result<()> {
        let item_l1 = &mut self.list[item.0];
        if let Some(list) = &mut item_l1.1 {
            let item_l2 = &mut list[item.1];
            if item_l2.1.is_none() {
                let itemlist = item_l2.0.get_l3(library).await?;
                item_l2.1 = Some(itemlist);
            }
        }
        Ok(())
    }

    pub async fn sync_with_database_all(&mut self, library: &Library) -> Result<()> {
        let now = Instant::now();
        self.sync_with_database_l1(library).await?;
        self.sync_with_database_l2(library).await?;
        self.sync_with_database_l3(library).await?;
        let mut new_list = vec![];
        for item in &self.list {
            if let Some(list) = &item.1 {
                if !list.is_empty() {
                    new_list.push(item.clone());
                }
            }
        }
        self.list = new_list;
        info!(target: "rmusic::speed", "Sync with db took {} sec", now.elapsed().as_secs());
        Ok(())
    }

    /// Get a list of all items in level 1, A in the type definition
    pub fn get_l1(&self) -> Vec<&'_ A> {
        self.list.iter().map(|x| &x.0).collect()
    }

    /// Get the context(playable items) of an item in level 1
    /// Should be used when the user wants to play an item from level 1
    pub async fn get_context_list_l1(
        &self,
        library: &Library,
        index: usize,
    ) -> TrackResult<QueueItem> {
        A::get_context_list(self.get_l1(), index, library).await
    }
    /// Get the context(playable items) of an item in level 3
    /// Should be used when the user wants to play an item from level 2
    /// index is (index level 1, index level 2)
    pub async fn get_context_list_l2(
        &self,
        library: &Library,
        index: (usize, usize),
    ) -> TrackResult<QueueItem> {
        B::get_context_list(self.get_l2(index.0), index.1, library).await
    }
    /// Get the context(playable items) of an item in level 3
    /// Should be used when the user wants to play an item from level 3
    /// index is (index level 1, index level 2, index level 3)
    pub async fn get_context_list_l3(
        &self,
        library: &Library,
        index: (usize, usize, usize),
    ) -> TrackResult<QueueItem> {
        C::get_context_list(self.get_l3((index.0, index.1)), index.2, library).await
    }

    /// Get the context(playable items) of an item in level 1
    /// Should be used when the user wants to add an item from level 1 to a playlist or the queue
    pub async fn get_context_l1(&self, library: &Library, index: usize) -> TrackResult<QueueItem> {
        match self.get_l1().get(index) {
            Some(item) => A::get_context(item, library).await,
            None => Err(TrackError::WrongInput("index out of bounds".into())),
        }
    }

    /// Get the context(playable items) of an item in level 2
    /// Should be used when the user wants to add an item from level 2 to a playlist or the queue
    /// index is (index level 1, index level 2)
    pub async fn get_context_l2(
        &self,
        library: &Library,
        index: (usize, usize),
    ) -> TrackResult<QueueItem> {
        match self.get_l2(index.0).get(index.1) {
            Some(item) => B::get_context(item, library).await,
            None => Err(TrackError::WrongInput("index out of bounds".into())),
        }
    }

    /// Get the context(playable items) of an item in level 2
    /// Should be used when the user wants to add an item from level 2 to a playlist or the queue
    /// index is (index level 1, index level 2, index level 3)
    pub async fn get_context_l3(
        &self,
        library: &Library,
        index: (usize, usize, usize),
    ) -> TrackResult<QueueItem> {
        match self.get_l3((index.0, index.1)).get(index.2) {
            Some(item) => C::get_context(item, library).await,
            None => Err(TrackError::WrongInput("index out of bounds".into())),
        }
    }

    /// Get a list of items in level 2, B in the type definition
    pub fn get_l2(&self, item: usize) -> Vec<&'_ B> {
        let item_l1 = &self.list[item];
        match &item_l1.1 {
            Some(list) => list.iter().map(|x| &x.0).collect(),
            None => vec![],
        }
    }

    /// Get a list of items in level 3, C in the type definition
    /// item is (level 1 index, level 2 index)
    pub fn get_l3(&self, item: (usize, usize)) -> Vec<&'_ C> {
        let item_l1 = &self.list[item.0];
        match &item_l1.1 {
            Some(list_l2) => {
                let item_l2 = &list_l2[item.1];
                match &item_l2.1 {
                    Some(list_l3) => list_l3.iter().collect(),
                    None => vec![],
                }
            }
            None => vec![],
        }
    }
}

pub type Result<T, E = migration::DbErr> = std::result::Result<T, E>;
