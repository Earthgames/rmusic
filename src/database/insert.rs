use chrono::NaiveDate;
use diesel::{prelude::*, result::Error};
use log::info;

use crate::schema::{artists, genres, publishers, releases, track_locations, tracks};

use super::Library;

/// Macro to make inserts if they don't exist easier
/// self: &mut Library,
/// table: schema module,
/// name: String(name of module/table),
/// name_value: String(name of thing being inserted),
/// values,+ : all things to select/filter on like: hair::color.eq("brown")
/// a ;
/// select => insert;* : for when the select and insert are different useful for Option
macro_rules! insert_if_not_exist {
    ($self:expr, $table:ident, $name:expr, $name_value:expr, $($value:expr),+ ; $($select:expr => $insert:expr;)*) => {
        Ok(match $table::table
            $(.filter($value))+
            $(.filter($select))*
            .select($table::id)
            .get_result(&mut $self.database)
        {
            Ok(id) => id,
            Err(err) => {
                if let Error::NotFound = err {
                    let id = diesel::insert_into($table::table)
                        .values(($($value),+ $(,$insert)* ))
                        .returning($table::id)
                        .get_result(&mut $self.database)?;
                    info!("Created {}: {}", $name, $name_value);
                    id
                } else {
                    return Err(err);
                }
            }
        })
    };
}

impl Library {
    pub fn insert_publisher_if_not_exist(
        &mut self,
        publisher: String,
        about: String,
    ) -> QueryResult<i32> {
        insert_if_not_exist!(
            self,
            publishers,
            "publisher",
            publisher,
            publishers::name.eq(&publisher),
            publishers::about.eq(&about);
        )
    }

    pub fn insert_artist_if_not_exist(
        &mut self,
        artist: String,
        about: String,
    ) -> QueryResult<i32> {
        insert_if_not_exist!(
            self,
            artists,
            "artist",
            artist,
            artists::name.eq(&artist),
            artists::about.eq(&about);
        )
    }

    pub fn insert_release_if_not_exist(
        &mut self,
        name: String,
        release_type: Option<String>,
        date: NaiveDate,
        artist_id: i32,
        publisher_id: Option<i32>,
    ) -> QueryResult<i32> {
        // Could be better but idk, don't want to write a whole macro for this
        match publisher_id {
            Some(publisher_id) => insert_if_not_exist!(
                self,
                releases,
                "release",
                name,
                releases::name.eq(&name),
                releases::release_type.eq(&release_type),
                releases::date.eq(date),
                releases::artist_id.eq(artist_id),
                releases::publisher_id.eq(publisher_id);
            ),
            None => insert_if_not_exist!(
                self,
                releases,
                "release",
                name,
                releases::name.eq(&name),
                releases::release_type.eq(&release_type),
                releases::date.eq(date),
                releases::artist_id.eq(artist_id);
                releases::publisher_id.is_null() => releases::publisher_id.eq(publisher_id);
            ),
        }
    }

    pub fn insert_track_if_not_exist(
        &mut self,
        name: String,
        date: NaiveDate,
        number: i32,
        duration: i32,
        artist_id: i32,
        release_id: i32,
    ) -> QueryResult<i32> {
        insert_if_not_exist!(
            self,
            tracks,
            "track",
            name,
            tracks::name.eq(&name),
            tracks::number.eq(number),
            tracks::duration.eq(duration),
            tracks::date.eq(date),
            tracks::artist_id.eq(artist_id),
            tracks::release_id.eq(release_id);
        )
    }

    pub fn insert_track_location_if_not_exist(
        &mut self,
        path: String,
        track_id: i32,
    ) -> QueryResult<()> {
        let opt = diesel::insert_into(track_locations::table)
            .values((
                track_locations::path.eq(&path),
                track_locations::track_id.eq(track_id),
            ))
            .on_conflict(track_locations::track_id)
            .do_update()
            .set(track_locations::track_id.eq(track_id))
            .execute(&mut self.database)
            .optional()?;
        if opt.is_some() {
            info!("Created or updated track_location: {path}");
        }
        Ok(())
    }

    pub fn insert_genres_if_not_exist(&mut self, name: String, track_id: i32) -> QueryResult<()> {
        let opt = diesel::insert_into(genres::table)
            .values((genres::name.eq(&name), genres::track_id.eq(track_id)))
            .on_conflict_do_nothing()
            .execute(&mut self.database)
            .optional()?;
        if opt.is_some() {
            info!("Created genre: {name}, for track_id {track_id}");
        }
        Ok(())
    }
}
