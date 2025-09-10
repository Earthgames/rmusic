use anyhow::{bail, Result};
use chrono::NaiveDate;
use diesel::{
    connection::Instrumentation,
    prelude::*,
    r2d2::{ConnectionManager, Pool, PooledConnection},
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
#[cfg(not(debug_assertions))]
use directories::{self, ProjectDirs};
use lofty::{read_from_path, ItemKey, Tag, TaggedFileExt};
use log::debug;
#[cfg(not(debug_assertions))]
use std::fs;
use std::{fmt::Display, path::Path};

pub mod context;
pub mod files;
pub mod insert;
pub mod library_view;
pub mod select;

type DB = diesel::sqlite::Sqlite;
type Conn = diesel::sqlite::SqliteConnection;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Main db struct
pub struct Library {
    connection_pool: Pool<ConnectionManager<Conn>>,
    database: PooledConnection<ConnectionManager<Conn>>,
}

pub struct DBLogger {}

impl Library {
    pub fn try_new() -> Result<Library> {
        #[cfg(debug_assertions)]
        let database_path = "./main.sqlite";
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
        let manager = ConnectionManager::<Conn>::new(database_path);
        let connection_pool = Pool::builder().test_on_check_out(true).build(manager)?;

        let mut database = connection_pool.get()?;
        database.set_instrumentation(DBLogger {});
        if let Err(err) = database.run_pending_migrations(MIGRATIONS) {
            bail!("Error while migrating: {err}")
        }
        Ok(Library {
            connection_pool,
            database,
        })
    }
    pub fn try_clone(&self) -> Result<Library> {
        let connection_pool = self.connection_pool.clone();
        let mut database = connection_pool.get()?;
        database.set_instrumentation(DBLogger {});
        Ok(Library {
            connection_pool,
            database,
        })
    }
}

impl Instrumentation for DBLogger {
    fn on_connection_event(&mut self, event: diesel::connection::InstrumentationEvent<'_>) {
        match event {
            diesel::connection::InstrumentationEvent::StartQuery { query, .. } => {
                debug!("query start: {}", query)
            }
            diesel::connection::InstrumentationEvent::FinishQuery { query, error, .. } => {
                debug!("query end: {query} error?: {:?}", error)
            }
            _ => debug!("Something in DB happend: {:?}", event),
        }
    }
}

#[derive(Debug)]
pub enum MusicFileError {
    /// A tag was missing that was necessary
    MissingTag(String),
    /// No tag could be found on the file
    NoTag,
    /// A tag cannot be parsed
    CannotParse(String),
    /// Cannot find a decoder to get the length of the track
    NoDecoder,
    /// Tag reader gave an error
    TagReaderError(lofty::error::LoftyError),
    /// A check failed while trying to open the file
    FileCheck(String),
    IOError(std::io::Error),
}

impl Display for MusicFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MusicFileError::MissingTag(tag) => write!(f, "Could not find tag: \"{}\"", tag),
            MusicFileError::CannotParse(text) => write!(f, "Could not parse tag {}", text),
            MusicFileError::NoDecoder => write!(f, "Could not find decoder for file"),
            MusicFileError::TagReaderError(lofty_error) => {
                write!(f, "The tag reader gave an error: \"{}\"", lofty_error)
            }
            MusicFileError::NoTag => write!(f, "No tag found on file"),
            MusicFileError::FileCheck(text) => write!(f, "Error while getting the file: {text}"),
            MusicFileError::IOError(error) => write!(f, "IOError: {error}"),
        }
    }
}
impl std::error::Error for MusicFileError {}
impl From<lofty::LoftyError> for MusicFileError {
    fn from(value: lofty::LoftyError) -> Self {
        MusicFileError::TagReaderError(value)
    }
}
impl From<std::io::Error> for MusicFileError {
    fn from(value: std::io::Error) -> Self {
        MusicFileError::IOError(value)
    }
}

fn get_tag(music_file: &Path) -> Result<Tag, MusicFileError> {
    let tagged_file = read_from_path(music_file)?;
    let tag = match tagged_file.primary_tag() {
        Some(tag) => tag,
        None => match tagged_file.first_tag() {
            Some(tag) => tag,
            None => return Err(MusicFileError::NoTag),
        },
    };
    Ok(tag.clone())
}

fn parse_date(date: &str) -> Option<NaiveDate> {
    if let Ok(date_result) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        return Some(date_result);
    }
    if let Ok(date_result) = NaiveDate::parse_from_str(date, "%Y/%m/%d") {
        return Some(date_result);
    }
    // if there is only a year
    if date.trim().len() == 4 {
        if let Ok(date_result) = NaiveDate::parse_from_str(&format!("{}0101", date), "%Y%m%d") {
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

fn number_from_tag(tag: &Tag, item_key: &ItemKey) -> Result<i32, MusicFileError> {
    let Some(number_tag) = string_from_tag(tag, item_key) else {
        return Err(MusicFileError::MissingTag(format!("{:?}", item_key)));
    };
    let Ok(number) = number_tag.parse::<i32>() else {
        return Err(MusicFileError::CannotParse(format!(
            "\"{:?}\" as number",
            item_key
        )));
    };
    Ok(number)
}

fn date_from_tag(tag: &Tag, item_key: &ItemKey) -> Result<NaiveDate, MusicFileError> {
    let Some(date_tag) = string_from_tag(tag, item_key) else {
        return Err(MusicFileError::MissingTag(format!("{:?}", item_key)));
    };
    let Some(date) = parse_date(&date_tag) else {
        return Err(MusicFileError::CannotParse(format!(
            "\"{:?}\" as date",
            item_key
        )));
    };
    Ok(date)
}
