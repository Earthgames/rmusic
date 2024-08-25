use crate::types::{Artist, Release, Track};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Track::Table)
                    .if_not_exists()
                    .col(pk_auto(Track::Id))
                    .col(string(Track::Name))
                    .col(date(Track::Date))
                    .col(integer(Track::Number))
                    .col(integer(Track::Duration))
                    .col(integer(Track::ArtistId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-track-artist_id")
                            .from(Track::Table, Track::ArtistId)
                            .to(Artist::Table, Artist::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(integer(Track::ReleaseId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-track-release_id")
                            .from(Track::Table, Track::ReleaseId)
                            .to(Release::Table, Release::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Track::Table).to_owned())
            .await
    }
}
