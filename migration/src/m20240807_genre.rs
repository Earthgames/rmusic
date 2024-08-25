use crate::types::{Genre, Track};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Genre::Table)
                    .if_not_exists()
                    .col(pk_auto(Genre::Id))
                    .col(string(Genre::Name))
                    .col(integer(Genre::TrackId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-genre-track_id")
                            .from(Genre::Table, Genre::TrackId)
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
            .drop_table(Table::drop().table(Genre::Table).to_owned())
            .await
    }
}
