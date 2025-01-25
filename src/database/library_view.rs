use std::marker::PhantomData;

use super::Library;
use entity::{artist, release, track};
use entity::{genre, publisher};
use sea_orm::prelude::*;
use sea_orm::EntityTrait;

/// Hate all the traits?
/// Me to, if you find a solution please let me know
///
/// Please read https://stackoverflow.com/questions/68728105/how-can-i-generate-trait-bounds-in-a-declarative-macro
/// or use
/// ```rust
/// where
///     A: Related<B> + EntityTrait,
///     A::Model: Viewable,
///     B: Related<C> + EntityTrait,
///     B::Model: Viewable,
///     C: EntityTrait,
///     C::Model: Viewable,
/// ```
pub struct LibraryView<A, B, C, V>
where
    A: L1<B, C, V>,
    B: L2<C, V>,
    C: L3<V>,
{
    list: Level1<A, B, C>,
    _view: PhantomData<V>,
}

pub trait Viewable<V> {
    fn to_view(&self) -> V;
}

macro_rules! impl_viewable {
    ($($i:ident),+) => {$(
        impl Viewable<String> for $i::Model {
            fn to_view(&self) -> String {
                self.name.clone()
            }
        })+
    };
}

impl_viewable!(artist, release, track, publisher, genre);

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

pub type Level1<A, B, C> = Vec<(A, Option<Level2<B, C>>)>;
pub type Level2<B, C> = Vec<(B, Option<Vec<C>>)>;

pub trait L1<A, B, V>: Sized + Viewable<V> + Sync
where
    A: L2<B, V>,
    B: L3<V>,
{
    fn get_all(library: &Library) -> impl std::future::Future<Output = Result<Vec<Self>>> + Send;
    fn get_l2(&self, library: &Library)
        -> impl std::future::Future<Output = Result<Vec<A>>> + Send;
}

impl<A, B, C, V> L1<B, C, V> for A
where
    A: Viewable<V> + ModelTrait + Sync,
    <<A as ModelTrait>::Entity as EntityTrait>::Model: IntoFR<A>,
    A::Entity: Related<B::Entity>,
    B: L2<C, V> + ModelTrait,
    <<B as ModelTrait>::Entity as EntityTrait>::Model: IntoFR<B>,
    C: L3<V>,
{
    async fn get_all(library: &Library) -> Result<Vec<Self>> {
        library
            .find_all::<A::Entity>()
            .await
            .map(|x| x.into_iter().map(|x| IntoFR::into(x)).collect())
    }

    async fn get_l2(&self, library: &Library) -> Result<Vec<B>> {
        library
            .model_related(self)
            .await
            .map(|x| x.into_iter().map(|z| IntoFR::into(z)).collect())
    }
}

impl<A, B, V> L2<B, V> for A
where
    A: ModelTrait + std::marker::Sync + Viewable<V>,
    B: L3<V>,
    <<B as ModelTrait>::Entity as EntityTrait>::Model: IntoFR<B>,
    A::Entity: Related<B::Entity>,
{
    async fn get_l3(&self, library: &Library) -> Result<std::vec::Vec<B>, migration::DbErr> {
        library
            .model_related::<_, B::Entity>(self)
            .await
            .map(|x| x.into_iter().map(|z| IntoFR::into(z)).collect())
    }
}

pub trait L2<A, V>: Viewable<V>
where
    A: L3<V>,
{
    fn get_l3(&self, library: &Library)
        -> impl std::future::Future<Output = Result<Vec<A>>> + Send;
}

pub trait L3<V>: ModelTrait + Viewable<V> {}

impl<A, B, C, V> Default for LibraryView<A, B, C, V>
where
    A: L1<B, C, V>,
    B: L2<C, V>,
    C: L3<V>,
{
    fn default() -> Self {
        Self {
            list: vec![],
            _view: PhantomData,
        }
    }
}

impl<A, B, C, V> LibraryView<A, B, C, V>
where
    A: L1<B, C, V>,
    B: L2<C, V>,
    C: L3<V>,
{
    pub async fn new(library: &Library) -> Result<LibraryView<A, B, C, V>> {
        let mut view_thing = LibraryView {
            list: vec![],
            _view: PhantomData,
        };
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
        self.sync_with_database_l1(library).await?;
        self.sync_with_database_l2(library).await?;
        self.sync_with_database_l3(library).await
    }

    // Get a list of all items in level 1, A in the type definition
    pub fn get_l1(&self) -> Vec<&'_ A> {
        self.list.iter().map(|x| &x.0).collect()
    }

    // Get a list of items in level 2, B in the type definition
    pub fn get_l2(&self, item: usize) -> Vec<&'_ B> {
        let item_l1 = &self.list[item];
        match &item_l1.1 {
            Some(list) => list.iter().map(|x| &x.0).collect(),
            None => vec![],
        }
    }

    // Get a list of items in level 3, C in the type definition
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
