use crate::types::{Playlist, PlaylistItem, Track};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlaylistItem::Table)
                    .if_not_exists()
                    .col(pk_auto(PlaylistItem::Id))
                    .col(date(PlaylistItem::Date))
                    .col(integer(PlaylistItem::Number))
                    .col(integer(PlaylistItem::Type))
                    .col(boolean(PlaylistItem::Deleted))
                    .col(integer(PlaylistItem::PlaylistId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-playlist_item-playlist_id")
                            .from(PlaylistItem::Table, PlaylistItem::PlaylistId)
                            .to(Playlist::Table, Playlist::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(integer_null(PlaylistItem::ItemPlaylistId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-playlist_item_playlist-playlist_id")
                            .from(PlaylistItem::Table, PlaylistItem::ItemPlaylistId)
                            .to(Playlist::Table, Playlist::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(integer_null(PlaylistItem::ItemTrackId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-playlist_item_track-track_id")
                            .from(PlaylistItem::Table, PlaylistItem::ItemTrackId)
                            .to(Track::Table, Track::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlaylistItem::Table).to_owned())
            .await
    }
}
