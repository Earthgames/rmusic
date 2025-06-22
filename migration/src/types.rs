use sea_orm_migration::prelude::*;

#[derive(DeriveIden)]
pub enum Artist {
    Table,
    Id,
    Name,
    About,
}

#[derive(DeriveIden)]
pub enum Publisher {
    Table,
    Id,
    Name,
    About,
}

#[derive(DeriveIden)]
pub enum Release {
    Table,
    Id,
    Name,
    Type,
    Date,
    PublisherId,
    ArtistId,
}

#[derive(DeriveIden)]
pub enum Playlist {
    Table,
    Id,
    Name,
    Description,
}

#[derive(DeriveIden)]
pub enum Track {
    Table,
    Id,
    Name,
    Number,
    Date,
    Duration,
    ArtistId,
    ReleaseId,
}

#[derive(DeriveIden)]
pub enum TrackLocation {
    Table,
    Path,
    TrackId,
}

#[derive(DeriveIden)]
pub enum PlaylistItem {
    Table,
    PlaylistId,
    Id,
    Date,
    Number,
    Deleted,
    Type,
    ItemTrackId,
    ItemReleaseId,
    ItemPlaylistId,
}

#[derive(DeriveIden)]
pub enum Genre {
    Table,
    Id,
    Name,
    TrackId,
}
