pub use sea_orm_migration::prelude::*;

mod m20240801_artist;
mod m20240802_publisher;
mod m20240803_release;
mod m20240804_playlist;
mod m20240805_track;
mod m20240806_playlist_item;
mod m20240807_genre;
mod m20240808_track_location;
pub mod types;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240801_artist::Migration),
            Box::new(m20240802_publisher::Migration),
            Box::new(m20240803_release::Migration),
            Box::new(m20240804_playlist::Migration),
            Box::new(m20240805_track::Migration),
            Box::new(m20240806_playlist_item::Migration),
            Box::new(m20240807_genre::Migration),
            Box::new(m20240808_track_location::Migration),
        ]
    }
}
