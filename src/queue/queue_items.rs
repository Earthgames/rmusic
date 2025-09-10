use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

use log::{error, warn};

use crate::{
    database::{context::ContextError, select::PlaylistItemType, Library},
    models::{Playlist, Release, Track, TrackLocation},
    struct_in_enum,
};

use super::{switch_shuffle, QueueOptions, ShuffleError, ShuffleType, DEPTH_LIMIT};

#[derive(Clone, PartialEq, Debug)]
pub struct QueueTrack {
    track: Track,
    location: PathBuf,
}

pub(crate) trait FromDB<T>: Sized + Into<QueueItem> {
    fn from_db(model: T, library: &mut Library) -> Result<Self, ContextError>;
}

impl QueueTrack {
    pub fn new(track: Track, location: PathBuf) -> QueueTrack {
        QueueTrack { track, location }
    }

    pub fn track(&self) -> &Track {
        &self.track
    }
    pub fn location(&self) -> &Path {
        &self.location
    }
}

impl FromDB<Track> for QueueTrack {
    fn from_db(track: Track, library: &mut Library) -> Result<QueueTrack, ContextError> {
        let locations = library.models_related::<Track, TrackLocation>(&track)?;
        for location in locations {
            let location = PathBuf::from(location.path);
            if location.is_file() {
                return Ok(QueueTrack::new(track, location));
            }
        }
        Err(ContextError::NoResult)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct QueueAlbum {
    album: Release,
    pub(crate) tracks: VecDeque<QueueTrack>,
    pub(crate) queue_option: QueueOptions,
    // unique id for playlist to sync history with upcoming items
    id: usize,
}

impl QueueAlbum {
    pub fn new(
        album: Release,
        tracks: VecDeque<QueueTrack>,
        queue_option: QueueOptions,
    ) -> QueueAlbum {
        QueueAlbum {
            album,
            tracks,
            queue_option,
            id: 0,
        }
    }

    pub fn from_db(release: Release, library: &mut Library) -> Result<QueueAlbum, ContextError> {
        let tracks_models = library.models_related::<_, Track>(&release)?;
        let mut tracks = VecDeque::new();

        for track in tracks_models {
            match QueueTrack::from_db(track, library) {
                Ok(track) => tracks.push_back(track),
                Err(err) => error!("Error while getting track: {err}"),
            }
        }

        if tracks.is_empty() {
            return Err(ContextError::NoResult);
        }

        Ok(QueueAlbum::new(release, tracks, Default::default()))
    }

    pub(crate) fn empty_clone(&self) -> QueueAlbum {
        QueueAlbum {
            album: self.album.clone(),
            tracks: VecDeque::new(),
            // queue options sometimes depends on the items, so we can only use the default
            queue_option: QueueOptions::default(),
            id: self.id,
        }
    }

    pub fn album(&self) -> &Release {
        &self.album
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn tracks(&self) -> &VecDeque<QueueTrack> {
        &self.tracks
    }

    pub fn queue_option(&self) -> &QueueOptions {
        &self.queue_option
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn change_queue_options(&mut self, queue_option: QueueOptions) {
        self.queue_option = queue_option;
    }
}

impl FromDB<Release> for QueueAlbum {
    fn from_db(release: Release, library: &mut Library) -> Result<QueueAlbum, ContextError> {
        let tracks_models = library.models_related::<_, Track>(&release)?;
        let mut tracks = VecDeque::new();

        for track in tracks_models {
            match QueueTrack::from_db(track, library) {
                Ok(track) => tracks.push_back(track),
                Err(err) => error!("Error while getting track: {err}"),
            }
        }

        if tracks.is_empty() {
            return Err(ContextError::NoResult);
        }

        Ok(QueueAlbum::new(release, tracks, Default::default()))
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct QueuePlaylist {
    playlist: Option<Playlist>,
    pub(crate) playlist_items: VecDeque<QueueItem>,
    pub(crate) queue_option: QueueOptions,
    id: usize,
}

impl QueuePlaylist {
    pub fn new(
        playlist: Option<Playlist>,
        playlist_items: VecDeque<QueueItem>,
        queue_option: QueueOptions,
    ) -> QueuePlaylist {
        QueuePlaylist {
            playlist,
            playlist_items,
            queue_option,
            id: 0,
        }
    }

    pub fn from_items(playlist_items: VecDeque<QueueItem>) -> QueuePlaylist {
        QueuePlaylist {
            playlist: None,
            playlist_items,
            queue_option: QueueOptions::default(),
            id: 0,
        }
    }

    pub(crate) fn empty_clone(&self) -> QueuePlaylist {
        QueuePlaylist {
            playlist: self.playlist.clone(),
            playlist_items: VecDeque::new(),
            queue_option: QueueOptions::default(),
            id: self.id,
        }
    }

    pub fn playlist(&self) -> &Option<Playlist> {
        &self.playlist
    }

    pub fn i(&self) -> &VecDeque<QueueItem> {
        &self.playlist_items
    }

    pub fn queue_option(&self) -> &QueueOptions {
        &self.queue_option
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn change_queue_options(&mut self, queue_option: QueueOptions) {
        self.queue_option = queue_option;
    }
}

impl FromDB<Playlist> for QueuePlaylist {
    fn from_db(playlist: Playlist, library: &mut Library) -> Result<QueuePlaylist, ContextError> {
        recursive_playlist_from_db(playlist, library, 0)
    }
}

fn recursive_playlist_from_db(
    playlist: Playlist,
    library: &mut Library,
    depth: usize,
) -> Result<QueuePlaylist, ContextError> {
    if depth > DEPTH_LIMIT {
        return Err(ContextError::MaxDepthReached);
    }

    let pl_items = match library.playlist(&playlist) {
        Ok(pl_items) => pl_items,
        Err(err) => {
            error!("Couldn't get playlist items: {:?}", err);
            return Err(ContextError::NoResult);
        }
    };
    let mut items = VecDeque::with_capacity(pl_items.len());
    for playlist_item in pl_items {
        macro_rules! get_model {
            ($model:expr, $result:expr) => {{
                let string = format!("{:?}", $model);
                match $result {
                    Ok(model) => model.into(),
                    Err(err) => match err {
                        ContextError::NoResult => {
                            warn!("No result for {:?}", string);
                            continue;
                        }
                        _ => return Err(err),
                    },
                }
            }};
        }

        let item: QueueItem = match playlist_item {
            PlaylistItemType::Track(model) => {
                get_model!(model, QueueTrack::from_db(model, library))
            }
            PlaylistItemType::Release(model) => {
                get_model!(model, QueueAlbum::from_db(model, library))
            }
            PlaylistItemType::Playlist(model) => {
                get_model!(model, recursive_playlist_from_db(model, library, depth + 1))
            }
        };
        items.push_back(item)
    }
    Ok(QueuePlaylist::new(
        Some(playlist),
        items,
        Default::default(),
    ))
}

#[derive(Clone, PartialEq, Debug)]
pub enum QueueItem {
    Track(QueueTrack),
    Playlist(QueuePlaylist),
    Album(QueueAlbum),
}

struct_in_enum!(QueueItem, impl_into_quei);
impl_into_quei!(Track : QueueTrack, Album: QueueAlbum, Playlist: QueuePlaylist);

impl QueueItem {
    pub fn flatten(self) -> VecDeque<QueueTrack> {
        match self {
            QueueItem::Track(track) => [track].into(),
            QueueItem::Playlist(playlist) => playlist
                .playlist_items
                .into_iter()
                .flat_map(|i| i.flatten())
                .collect(),
            QueueItem::Album(album) => album.tracks,
        }
    }

    pub fn count(&self) -> u32 {
        match self {
            QueueItem::Track(_) => 1,
            QueueItem::Playlist(playlist) => {
                playlist.playlist_items.iter().map(|x| x.count()).sum()
            }
            QueueItem::Album(album) => album.tracks.len() as u32,
        }
    }

    /// If the current queue-item contains a track somewhere in itself
    pub fn is_empty(&self) -> bool {
        match self {
            // if a track is selected than it has been played
            QueueItem::Track(_) => false,
            QueueItem::Playlist(playlist) => {
                if playlist.playlist_items.is_empty() {
                    return true;
                }
                //WARN: Could infinitely recurs
                playlist.playlist_items.iter().all(|x| x.is_empty())
            }
            QueueItem::Album(album) => album.is_empty(),
        }
    }

    pub fn get_selected(&self) -> Option<QueueTrack> {
        match self {
            QueueItem::Track(track) => Some(track.clone()),
            QueueItem::Playlist(playlist) => {
                let item = playlist
                    .playlist_items
                    .get(playlist.queue_option.selected?)?;
                item.get_selected()
            }
            QueueItem::Album(album) => album.tracks.get(album.queue_option.selected?).cloned(),
        }
    }

    /// Set QueueOptions only for this item
    pub fn set_queue_options(&mut self, options: QueueOptions) {
        match self {
            QueueItem::Playlist(ref mut playlist) => {
                playlist.change_queue_options(options);
            }
            QueueItem::Album(ref mut album) => {
                album.change_queue_options(options);
            }
            QueueItem::Track(_) => (),
        }
    }

    /// Set QueueOptions for this item and the children
    pub fn set_queue_options_rec(&mut self, options: QueueOptions) {
        self.set_queue_options(options.clone());
        if let QueueItem::Playlist(ref mut playlist) = self {
            for item in playlist.playlist_items.iter_mut() {
                item.set_queue_options_rec(options.clone());
            }
        }
    }

    /// Switch the shuffle type of this QueueItem
    pub fn switch_shuffle(&mut self, new_shuffle: ShuffleType) -> Result<(), ShuffleError> {
        match self {
            QueueItem::Track(_) => Ok(()),
            QueueItem::Playlist(playlist) => switch_shuffle(
                &mut playlist.queue_option.shuffle_type,
                new_shuffle,
                playlist.playlist_items.len(),
            ),
            QueueItem::Album(album) => switch_shuffle(
                &mut album.queue_option.shuffle_type,
                new_shuffle,
                album.tracks.len(),
            ),
        }
    }

    pub fn has_id(&self, id: usize) -> bool {
        match self {
            QueueItem::Track(_) => false,
            QueueItem::Playlist(queue_playlist) => queue_playlist.id() == id,
            QueueItem::Album(queue_album) => queue_album.id() == id,
        }
    }

    /// Returns last used id + 1
    /// When no id is used it returns the same id
    /// Wraps when it would overflow
    pub(crate) fn set_id(&mut self, id: usize) -> usize {
        match self {
            QueueItem::Track(_) => id,
            QueueItem::Playlist(queue_playlist) => {
                queue_playlist.id = id;
                id.wrapping_add(1)
            }
            QueueItem::Album(queue_album) => {
                queue_album.id = id;
                id.wrapping_add(1)
            }
        }
    }

    /// See [`set_id()`]
    /// Does the same but recursively
    pub(crate) fn set_id_rec(&mut self, id: usize) -> usize {
        let mut id = self.set_id(id);
        if let QueueItem::Playlist(queue_playlist) = self {
            for item in queue_playlist.playlist_items.iter_mut() {
                id = item.set_id_rec(id)
            }
        }
        id
    }
}

// Boring test just to be sure
// written by AI, but I checked them
#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::models::{Playlist, Release, Track};
    use std::collections::VecDeque;

    // Helper function to create a dummy QueueTrack
    fn dummy_queue_track(id: i32) -> QueueTrack {
        QueueTrack::new(
            Track {
                id,
                name: format!("Track {}", id),
                date: NaiveDate::default(),
                number: 0,
                duration: 2000,
                artist_id: 0,
                release_id: 0,
            },
            PathBuf::from(format!("/path/to/track_{}.mp3", id)),
        )
    }

    // Helper function to create a dummy QueueAlbum
    fn dummy_queue_album(id: i32, num_tracks: usize) -> QueueAlbum {
        let mut tracks = VecDeque::new();
        for i in 0..num_tracks {
            tracks.push_back(dummy_queue_track(id * 100 + i as i32));
        }
        QueueAlbum::new(
            Release {
                id,
                name: format!("Album {}", id),
                release_type: None,
                date: NaiveDate::default(),
                publisher_id: None,
                artist_id: 0,
            },
            tracks,
            Default::default(),
        )
    }

    // Helper function to create a dummy QueuePlaylist
    fn dummy_queue_playlist(id: i32, items: VecDeque<QueueItem>) -> QueuePlaylist {
        QueuePlaylist::new(
            Some(Playlist {
                id,
                name: format!("Playlist {}", id),
                description: "".into(),
            }),
            items,
            Default::default(),
        )
    }

    #[test]
    fn test_set_id_for_album() {
        let mut album_item = QueueItem::Album(dummy_queue_album(1, 3));
        let initial_id = 10;
        let next_id = album_item.set_id(initial_id);

        // For a QueueItem::Album, its ID should be set, and the next ID should be initial_id + 1
        if let QueueItem::Album(album) = &album_item {
            assert_eq!(*album.id(), initial_id);
        } else {
            panic!("Expected QueueItem::Album");
        }
        assert_eq!(next_id, initial_id.wrapping_add(1));
    }

    #[test]
    fn test_set_id_for_playlist() {
        let mut playlist_item = QueueItem::Playlist(dummy_queue_playlist(
            1,
            VecDeque::from([QueueItem::Track(dummy_queue_track(1))]),
        ));
        let initial_id = 20;
        let next_id = playlist_item.set_id(initial_id);

        // For a QueueItem::Playlist, its ID should be set, and the next ID should be initial_id + 1
        if let QueueItem::Playlist(playlist) = &playlist_item {
            assert_eq!(*playlist.id(), initial_id);
        } else {
            panic!("Expected QueueItem::Playlist");
        }
        assert_eq!(next_id, initial_id.wrapping_add(1));
    }

    #[test]
    fn test_set_id_wrapping() {
        let mut album_item = QueueItem::Album(dummy_queue_album(1, 1));
        let initial_id = usize::MAX; // Test wrapping
        let next_id = album_item.set_id(initial_id);

        if let QueueItem::Album(album) = &album_item {
            assert_eq!(*album.id(), initial_id);
        } else {
            panic!("Expected QueueItem::Album");
        }
        assert_eq!(next_id, 0); // MAX.wrapping_add(1) should be 0
    }

    #[test]
    fn test_set_id_rec_simple_playlist() {
        // Playlist containing two tracks
        let mut playlist_item = QueueItem::Playlist(dummy_queue_playlist(
            1,
            VecDeque::from([
                QueueItem::Track(dummy_queue_track(1)),
                QueueItem::Track(dummy_queue_track(2)),
            ]),
        ));

        let initial_id = 100;
        let next_id = playlist_item.set_id_rec(initial_id);

        // The playlist itself should get the initial ID
        if let QueueItem::Playlist(playlist) = &playlist_item {
            assert_eq!(*playlist.id(), initial_id);
        } else {
            panic!("Expected QueueItem::Playlist");
        }
        // The next ID should be initial_id + 1 (only the playlist gets an ID)
        assert_eq!(next_id, initial_id.wrapping_add(1));
    }

    #[test]
    fn test_set_id_rec_complex_playlist() {
        // Playlist structure:
        // - Playlist 1 (ID 100)
        //   - Track 1
        //   - Album 2 (ID 101)
        //     - Track 201
        //     - Track 202
        //   - Playlist 3 (ID 102)
        //     - Track 301
        //     - Album 4 (ID 103)
        //       - Track 401
        let mut inner_album_tracks = VecDeque::new();
        inner_album_tracks.push_back(dummy_queue_track(401));
        let inner_album = QueueItem::Album(dummy_queue_album(4, 1)); // Album 4, 1 track

        let mut inner_playlist_items = VecDeque::new();
        inner_playlist_items.push_back(QueueItem::Track(dummy_queue_track(301)));
        inner_playlist_items.push_back(inner_album);
        let inner_playlist = QueueItem::Playlist(dummy_queue_playlist(3, inner_playlist_items)); // Playlist 3

        let mut mid_album_tracks = VecDeque::new();
        mid_album_tracks.push_back(dummy_queue_track(201));
        mid_album_tracks.push_back(dummy_queue_track(202));
        let mid_album = QueueItem::Album(dummy_queue_album(2, 2)); // Album 2, 2 tracks

        let mut root_playlist_items = VecDeque::new();
        root_playlist_items.push_back(QueueItem::Track(dummy_queue_track(1)));
        root_playlist_items.push_back(mid_album);
        root_playlist_items.push_back(inner_playlist);

        let mut root_playlist_item =
            QueueItem::Playlist(dummy_queue_playlist(1, root_playlist_items));

        let initial_id = 100;
        let final_id = root_playlist_item.set_id_rec(initial_id);

        // Verify IDs
        if let QueueItem::Playlist(root_pl) = &root_playlist_item {
            assert_eq!(*root_pl.id(), 100);

            // Item 1: Album (should be 101)
            if let QueueItem::Album(album) = &root_pl.playlist_items[1] {
                assert_eq!(*album.id(), 101);
            } else {
                panic!("Expected Album at index 1");
            }

            // Item 2: Inner Playlist (should be 102)
            if let QueueItem::Playlist(inner_pl) = &root_pl.playlist_items[2] {
                assert_eq!(*inner_pl.id(), 102);

                // Inner Playlist Item 1: Album (should be 103)
                if let QueueItem::Album(album) = &inner_pl.playlist_items[1] {
                    assert_eq!(*album.id(), 103);
                    // Check album tracks remain original
                } else {
                    panic!("Expected Album at index 1 of inner playlist");
                }
            } else {
                panic!("Expected Playlist at index 2");
            }
        } else {
            panic!("Expected root QueueItem::Playlist");
        }

        // Final ID should be 104 (initial + 4 items that take an ID: root playlist, album 2, playlist 3, album 4)
        assert_eq!(final_id, initial_id.wrapping_add(4));
    }

    #[test]
    fn test_set_id_rec_empty_playlist() {
        let mut empty_playlist_item = QueueItem::Playlist(dummy_queue_playlist(1, VecDeque::new()));
        let initial_id = 500;
        let next_id = empty_playlist_item.set_id_rec(initial_id);

        if let QueueItem::Playlist(pl) = &empty_playlist_item {
            assert_eq!(*pl.id(), initial_id);
        } else {
            panic!("Expected QueueItem::Playlist");
        }
        // Only the playlist itself takes an ID
        assert_eq!(next_id, initial_id.wrapping_add(1));
    }

    #[test]
    fn test_set_id_rec_track_item() {
        let mut track_item = QueueItem::Track(dummy_queue_track(99));
        let initial_id = 700;
        let next_id = track_item.set_id_rec(initial_id);
        assert_eq!(next_id, initial_id);
    }

    #[test]
    fn test_set_id_rec_album_item() {
        let mut album_item = QueueItem::Album(dummy_queue_album(5, 2));
        let initial_id = 800;
        let next_id = album_item.set_id_rec(initial_id);

        // For an Album, set_id_rec behaves like set_id (sets its ID, returns incremented ID)
        if let QueueItem::Album(album) = &album_item {
            assert_eq!(*album.id(), initial_id);
        } else {
            panic!("Expected QueueItem::Album");
        }
        assert_eq!(next_id, initial_id.wrapping_add(1));
    }
}
