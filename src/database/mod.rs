use anyhow::{bail, Result};
#[cfg(not(debug_assertions))]
use directories::{self, ProjectDirs};
use lofty::read_from_path;
use lofty::ItemKey;
use lofty::Tag;
use lofty::TaggedFileExt;
use migration::MigratorTrait;
use sea_orm::{prelude::*, ConnectOptions, Database};
#[cfg(not(debug_assertions))]
use std::fs;
use std::path::Path;

pub use entity::*;

pub use entity::artist::Model as Artist;
pub use entity::genre::Model as Genre;
pub use entity::playlist::Model as Playlist;
pub use entity::playlist_item::Model as PlaylistItem;
pub use entity::publisher::Model as Publisher;
pub use entity::release::Model as Release;
pub use entity::track::Model as Track;
pub use entity::track_location::Model as TrackLocation;

pub mod context;
pub mod files;
pub mod insert;
pub mod library_view;
pub mod select;

/// Main db struct
#[derive(Clone, Debug)]
pub struct Library {
    database: DatabaseConnection,
}

impl Library {
    pub async fn try_new() -> Result<Library> {
        #[cfg(debug_assertions)]
        let database_path = "sqlite:./main.sqlite?mode=rwc";
        #[cfg(not(debug_assertions))]
        let dir = match ProjectDirs::from("", "Earthgame_s", "rmusic") {
            Some(dir) => dir,
            None => bail!("Can't get directories"),
        };
        #[cfg(not(debug_assertions))]
        let database_path = format!(
            "sqlite:{}/main.sqlite?mode=rwc",
            directories::ProjectDirs::data_dir(&dir).display()
        );
        #[cfg(not(debug_assertions))]
        if let Err(err) = fs::create_dir_all(directories::ProjectDirs::data_dir(&dir)) {
            match err.kind() {
                std::io::ErrorKind::AlreadyExists => (),
                _ => bail!("Could not create path"),
            }
        }
        println!("{}", database_path);
        let mut database_options = ConnectOptions::new(database_path);
        database_options
            .sqlx_logging(true)
            .sqlx_logging_level(log::LevelFilter::Info);

        let database = Database::connect(database_options).await?;
        migration::Migrator::up(&database, None).await?;
        Ok(Library { database })
    }
}

fn get_tag(music_file: &Path) -> Result<Tag> {
    let tagged_file = read_from_path(music_file)?;
    let tag = match tagged_file.primary_tag() {
        Some(tag) => tag,
        None => match tagged_file.first_tag() {
            Some(tag) => tag,
            None => bail!("No tag found"),
        },
    };
    Ok(tag.clone())
}

fn parse_date(date: &str) -> Option<Date> {
    if let Ok(date_result) = Date::parse_from_str(date, "%Y-%m-%d") {
        return Some(date_result);
    }
    if let Ok(date_result) = Date::parse_from_str(date, "%Y/%m/%d") {
        return Some(date_result);
    }
    // if there is only a year
    if date.trim().len() == 4 {
        if let Ok(date_result) = Date::parse_from_str(&format!("{}0101", date), "%Y%m%d") {
            return Some(date_result);
        }
    }
    None
}

fn string_from_tag(tag: &Tag, item_key: &ItemKey) -> Option<String> {
    let string_tag = tag.get(item_key)?;
    let string = string_tag.clone().into_value().into_string()?;
    Some(string)
}

/// Returns strings from all references of the item_key
fn multiple_string_from_tag(tag: &Tag, item_key: &ItemKey) -> Vec<String> {
    let string_tag = tag.get_strings(item_key);
    let multiple_string: Vec<String> = string_tag.map(String::from).collect();
    multiple_string
}

fn number_from_tag(tag: &Tag, item_key: &ItemKey) -> Result<i32> {
    let Some(number_tag) = string_from_tag(tag, item_key) else {
        bail!("Could not find number tag: {:?}", item_key);
    };
    let Ok(number) = number_tag.parse::<i32>() else {
        bail!("Could not parse number tag: {:?}", item_key);
    };
    Ok(number)
}

fn date_from_tag(tag: &Tag, item_key: &ItemKey) -> Result<Date> {
    let Some(date_tag) = string_from_tag(tag, item_key) else {
        bail!("Could not find date tag: {:?}", item_key);
    };
    let Some(date) = parse_date(&date_tag) else {
        bail!("Could not parse date tag: {:?}", item_key);
    };
    Ok(date)
}
