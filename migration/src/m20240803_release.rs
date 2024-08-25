use crate::types::{Artist, Publisher, Release};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Release::Table)
                    .if_not_exists()
                    .col(pk_auto(Release::Id))
                    .col(string(Release::Name))
                    .col(string_null(Release::Type))
                    .col(date(Release::Date))
                    .col(integer_null(Release::PublisherId))
                    .col(integer(Release::ArtistId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-release-publisher_id")
                            .from(Release::Table, Release::PublisherId)
                            .to(Publisher::Table, Publisher::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-release-artist_id")
                            .from(Release::Table, Release::ArtistId)
                            .to(Artist::Table, Artist::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Release::Table).to_owned())
            .await
    }
}
