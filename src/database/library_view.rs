use crate::queue::queue_items::QueueItem;

use super::context::{ContextError, GetContext, GetContextList, TrackResult};
use super::select::{GetAll, RelatedMany};
use super::Library;
use log::info;
use tokio::time::Instant;

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

pub trait L1<A, B>: GetContext + GetContextList + Sized
where
    A: L2<B>,
    B: L3,
{
    fn get_all(library: &mut Library) -> TrackResult<Vec<Self>>;
    fn get_l2(&self, library: &mut Library) -> TrackResult<Vec<A>>;
}

pub trait L2<A>: GetContext + GetContextList
where
    A: L3,
{
    fn get_l3(&self, library: &mut Library) -> TrackResult<Vec<A>>;
}

pub trait L3: GetContext + GetContextList {}

impl<A, B, C> L1<B, C> for A
where
    A: GetContext + GetAll + GetContextList,
    A: RelatedMany<B>,
    B: L2<C>,
    C: L3,
{
    fn get_all(library: &mut Library) -> TrackResult<Vec<Self>> {
        Ok(library.find_all::<A>().map(|x| x.into_iter().collect())?)
    }

    fn get_l2(&self, library: &mut Library) -> TrackResult<Vec<B>> {
        Ok(library
            .models_related(self)
            .map(|x| x.into_iter().collect())?)
    }
}

impl<A, B> L2<B> for A
where
    A: GetContext + GetContextList,
    B: L3 + GetContext,
    A: RelatedMany<B>,
{
    fn get_l3(&self, library: &mut Library) -> TrackResult<std::vec::Vec<B>> {
        Ok(library.models_related::<_, B>(self)?)
    }
}

impl<A> L3 for A where A: GetContext + GetContextList {}

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
    A: L1<B, C> + Clone,
    B: L2<C> + Clone,
    C: L3 + Clone,
{
    pub fn new(library: &mut Library) -> TrackResult<LibraryView<A, B, C>> {
        let mut view_thing = LibraryView { list: vec![] };
        view_thing.sync_with_database_l1(library)?;
        Ok(view_thing)
    }
    /// Sync level 1 with database, A in type definition
    pub fn sync_with_database_l1(&mut self, library: &mut Library) -> TrackResult<()> {
        self.list = A::get_all(library)?
            .into_iter()
            .map(|x| (x, None))
            .collect();
        Ok(())
    }
    /// Sync all level 2 with database, B in type definition
    pub fn sync_with_database_l2(&mut self, library: &mut Library) -> TrackResult<()> {
        for item_l1 in self.list.iter_mut() {
            if item_l1.1.is_none() {
                let item_list = item_l1
                    .0
                    .get_l2(library)?
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
    pub fn sync_with_database_l3(&mut self, library: &mut Library) -> TrackResult<()> {
        for item_l1 in self.list.iter_mut() {
            if let Some(list) = &mut item_l1.1 {
                for item_l2 in list.iter_mut() {
                    if item_l2.1.is_none() {
                        let item_list = item_l2.0.get_l3(library)?;
                        item_l2.1 = Some(item_list);
                    }
                }
            }
        }
        Ok(())
    }

    /// Sync a specific item from level 2, B in the type definition
    pub fn sync_with_database_l2_item(
        &mut self,
        library: &mut Library,
        item: usize,
    ) -> TrackResult<()> {
        let item_l1 = &mut self.list.get_mut(item);
        if let Some(item_l1) = item_l1 {
            if item_l1.1.is_none() {
                let itemlist = item_l1
                    .0
                    .get_l2(library)?
                    .into_iter()
                    .map(|x| (x, None))
                    .collect();
                item_l1.1 = Some(itemlist);
            }
        }
        Ok(())
    }

    /// Sync a specific item from level 3, C in the type definition
    /// item is (level 1 index, level 2 index)
    pub fn sync_with_database_l3_item(
        &mut self,
        library: &mut Library,
        item: (usize, usize),
    ) -> TrackResult<()> {
        let mut item_l1 = &mut self.list.get_mut(item.0);
        if let Some((_, Some(list))) = &mut item_l1 {
            let item_l2 = &mut list.get_mut(item.1);
            if let Some(item_l2) = item_l2 {
                if item_l2.1.is_none() {
                    let itemlist = item_l2.0.get_l3(library)?;
                    item_l2.1 = Some(itemlist);
                }
            }
        }
        Ok(())
    }

    pub fn sync_with_database_all(&mut self, library: &mut Library) -> TrackResult<()> {
        let now = Instant::now();
        self.sync_with_database_l1(library)?;
        self.sync_with_database_l2(library)?;
        self.sync_with_database_l3(library)?;
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
    pub fn get_context_list_l1(
        &self,
        library: &mut Library,
        index: usize,
    ) -> TrackResult<QueueItem> {
        A::get_context_list(self.get_l1(), index, library)
    }
    /// Get the context(playable items) of an item in level 3
    /// Should be used when the user wants to play an item from level 2
    /// index is (index level 1, index level 2)
    pub fn get_context_list_l2(
        &self,
        library: &mut Library,
        index: (usize, usize),
    ) -> TrackResult<QueueItem> {
        B::get_context_list(self.get_l2(index.0), index.1, library)
    }
    /// Get the context(playable items) of an item in level 3
    /// Should be used when the user wants to play an item from level 3
    /// index is (index level 1, index level 2, index level 3)
    pub fn get_context_list_l3(
        &self,
        library: &mut Library,
        index: (usize, usize, usize),
    ) -> TrackResult<QueueItem> {
        C::get_context_list(self.get_l3((index.0, index.1)), index.2, library)
    }

    /// Get the context(playable items) of an item in level 1
    /// Should be used when the user wants to add an item from level 1 to a playlist or the queue
    pub fn get_context_l1(&self, library: &mut Library, index: usize) -> TrackResult<QueueItem> {
        let l1 = self.get_l1();
        match l1.get(index) {
            Some(item) => A::get_context(item, library),
            None => Err(ContextError::IndexOutOfBounds(l1.len())),
        }
    }

    /// Get the context(playable items) of an item in level 2
    /// Should be used when the user wants to add an item from level 2 to a playlist or the queue
    /// index is (index level 1, index level 2)
    pub fn get_context_l2(
        &self,
        library: &mut Library,
        index: (usize, usize),
    ) -> TrackResult<QueueItem> {
        let l2 = self.get_l2(index.0);
        match l2.get(index.1) {
            Some(item) => B::get_context(item, library),
            None => Err(ContextError::IndexOutOfBounds(l2.len())),
        }
    }

    /// Get the context(playable items) of an item in level 2
    /// Should be used when the user wants to add an item from level 2 to a playlist or the queue
    /// index is (index level 1, index level 2, index level 3)
    pub fn get_context_l3(
        &self,
        library: &mut Library,
        index: (usize, usize, usize),
    ) -> TrackResult<QueueItem> {
        let l3 = self.get_l3((index.0, index.1));
        match l3.get(index.2) {
            Some(item) => C::get_context(item, library),
            None => Err(ContextError::IndexOutOfBounds(l3.len())),
        }
    }

    /// Get a list of items in level 2, B in the type definition
    pub fn get_l2(&self, item: usize) -> Vec<&'_ B> {
        let item_l1 = &self.list.get(item);
        match &item_l1 {
            Some((_, Some(list))) => list.iter().map(|x| &x.0).collect(),
            _ => vec![],
        }
    }

    /// Get a list of items in level 3, C in the type definition
    /// item is (level 1 index, level 2 index)
    pub fn get_l3(&self, item: (usize, usize)) -> Vec<&'_ C> {
        let item_l1 = &self.list.get(item.0);
        match &item_l1 {
            Some((_, Some(list_l2))) => {
                let item_l2 = &list_l2.get(item.1);
                match &item_l2 {
                    Some((_, Some(list_l3))) => list_l3.iter().collect(),
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    }
}
