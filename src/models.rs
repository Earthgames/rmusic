use crate::schema::*;
use chrono::NaiveDate;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Identifiable, Clone, PartialEq)]
pub struct Artist {
    pub id: i32,
    pub name: String,
    pub about: String,
}

#[derive(Queryable, Selectable, Debug, Identifiable, Associations, Clone, PartialEq)]
#[diesel(belongs_to(Track))]
pub struct Genre {
    pub id: i32,
    pub name: String,
    pub track_id: i32,
}

#[derive(Queryable, Selectable, Debug, Identifiable, Clone, PartialEq)]
pub struct Playlist {
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Queryable, Selectable, Debug, Identifiable, Associations, Clone, PartialEq)]
#[diesel(belongs_to(Playlist))]
#[diesel(belongs_to(Release, foreign_key = item_release_id))]
#[diesel(belongs_to(Track, foreign_key = item_track_id))]
pub struct PlaylistItem {
    pub id: i32,
    pub date: NaiveDate,
    pub number: i32,
    pub item_type: i32,
    pub deleted: bool,
    pub playlist_id: i32,
    pub item_playlist_id: Option<i32>,
    pub item_release_id: Option<i32>,
    pub item_track_id: Option<i32>,
}

#[derive(Queryable, Selectable, Debug, Identifiable, Clone, PartialEq)]
pub struct Publisher {
    pub id: i32,
    pub name: String,
    pub about: String,
}

#[derive(Queryable, Selectable, Debug, Identifiable, Associations, Clone, PartialEq)]
#[diesel(belongs_to(Publisher))]
#[diesel(belongs_to(Artist))]
pub struct Release {
    pub id: i32,
    pub name: String,
    pub release_type: Option<String>,
    pub date: NaiveDate,
    pub publisher_id: Option<i32>,
    pub artist_id: i32,
}

#[derive(Queryable, Selectable, Debug, Identifiable, Associations, Clone, PartialEq)]
#[diesel(belongs_to(Artist))]
#[diesel(belongs_to(Release))]
pub struct Track {
    pub id: i32,
    pub name: String,
    pub date: NaiveDate,
    pub number: i32,
    pub duration: i32,
    pub artist_id: i32,
    pub release_id: i32,
}

#[derive(
    Queryable, Insertable, Selectable, Debug, Identifiable, Associations, Clone, PartialEq,
)]
#[diesel(belongs_to(Track))]
#[diesel(primary_key(path))]
pub struct TrackLocation {
    pub path: String,
    pub track_id: i32,
}
