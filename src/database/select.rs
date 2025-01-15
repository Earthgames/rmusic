use super::Library;
use entity::{artist, release, track};
use sea_orm::prelude::*;
use sea_orm::EntityTrait;

impl Library {
    pub async fn model_related<M, R>(&self, model: &M) -> Result<Vec<R::Model>>
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
        let releases = self.model_related::<_, release::Entity>(artist).await?;
        let mut release_tracks = vec![];
        for release in &releases {
            release_tracks.push(self.model_related::<_, track::Entity>(release).await?);
        }
        Ok(releases.into_iter().zip(release_tracks).collect())
    }
}

pub struct LibraryView<A, B, C>
where
    A: EntityTrait,
    B: EntityTrait,
    A: Related<B>,
    C: EntityTrait,
    B: Related<C>,
{
    list: Level1<A, B, C>,
}

pub type Level1<A, B, C> = Vec<(<A as EntityTrait>::Model, Option<Level2<B, C>>)>;
pub type Level2<B, C> = Vec<(
    <B as EntityTrait>::Model,
    Option<Vec<<C as EntityTrait>::Model>>,
)>;

impl<A, B, C> Default for LibraryView<A, B, C>
where
    A: EntityTrait,
    B: EntityTrait,
    A: Related<B>,
    C: EntityTrait,
    B: Related<C>,
{
    fn default() -> Self {
        Self { list: vec![] }
    }
}

impl<A, B, C> LibraryView<A, B, C>
where
    A: EntityTrait,
    B: EntityTrait,
    A: Related<B>,
    C: EntityTrait,
    B: Related<C>,
{
    pub async fn new(library: &Library) -> Result<LibraryView<A, B, C>> {
        let mut view_thing = LibraryView { list: vec![] };
        view_thing.sync_with_database_l1(library).await?;
        Ok(view_thing)
    }
    /// Sync level 1 with database, A in type definition
    pub async fn sync_with_database_l1(&mut self, library: &Library) -> Result<()> {
        self.list = library
            .find_all::<A>()
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
                let item_list = library
                    .model_related::<A::Model, B>(&item_l1.0)
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
                        let item_list = library.model_related::<B::Model, C>(&item_l2.0).await?;
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
            let itemlist = library
                .model_related::<_, B>(&item_l1.0)
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
                let itemlist = library.model_related::<_, C>(&item_l2.0).await?;
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
    pub fn get_l1(&self) -> Vec<&'_ A::Model> {
        self.list.iter().map(|x| &x.0).collect()
    }

    // Get a list of items in level 2, B in the type definition
    pub fn get_l2(&self, item: usize) -> Vec<&'_ B::Model> {
        let item_l1 = &self.list[item];
        match &item_l1.1 {
            Some(list) => list.iter().map(|x| &x.0).collect(),
            None => vec![],
        }
    }

    // Get a list of items in level 3, C in the type definition
    /// item is (level 1 index, level 2 index)
    pub fn get_l3(&self, item: (usize, usize)) -> Vec<&'_ C::Model> {
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
