use std::path::Path;

use anyhow::{bail, Result};
#[cfg(not(debug_assertions))]
use directories;
use lofty::read_from_path;
use lofty::ItemKey;
use lofty::Tag;
use lofty::TaggedFileExt;
use migration::MigratorTrait;
use sea_orm::{prelude::*, ConnectOptions, Database};

pub mod files;
pub mod insert;

pub struct Library {
    database: DatabaseConnection,
}

impl Library {
    pub async fn try_new() -> Result<Library> {
        #[cfg(debug_assertions)]
        let database_path = "sqlite:./main.sqlite?mode=rwc";
        #[cfg(not(debug_assertions))]
        {
            let dir = directories::ProjectDirs::from("", "Earthgame_s", "rmusic")
                .unwrap_or(bail!("Can't get directories"));
            let db_path = format!(
                "sqlite:{}/main.sqlite?mode=rwc",
                directories::ProjectDirs::data_dir(&dir).display()
            );
        }

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
    let multiple_string: Vec<String> = string_tag.map(|x| String::from(x)).collect();
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
