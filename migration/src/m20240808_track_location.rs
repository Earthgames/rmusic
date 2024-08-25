use crate::types::{Track, TrackLocation};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TrackLocation::Table)
                    .if_not_exists()
                    .col(string(TrackLocation::Path).primary_key())
                    .col(integer(TrackLocation::TrackId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-track_location-track_id")
                            .from(TrackLocation::Table, TrackLocation::TrackId)
                            .to(Track::Table, Track::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TrackLocation::Table).to_owned())
            .await
    }
}
