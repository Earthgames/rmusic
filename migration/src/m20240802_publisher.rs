use crate::types::Publisher;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Publisher::Table)
                    .if_not_exists()
                    .col(pk_auto(Publisher::Id))
                    .col(string(Publisher::Name).unique_key())
                    .col(string(Publisher::About))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Publisher::Table).to_owned())
            .await
    }
}
