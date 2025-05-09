use std::{collections::VecDeque, path::PathBuf};

mod select_track;

use rand::thread_rng;
pub use select_track::get_track_from_item;
use select_track::get_track_from_list;

/// Struct that will play things next
#[derive(Clone, PartialEq, Debug)]
pub struct Queue {
    pub(crate) queue_items: VecDeque<QueueItem>,
    //TODO: add history size
    played_items: VecDeque<PathBuf>,
    pub queue_options: QueueOptions,
    pub repeat_current: bool,
    pub(crate) current_track: Option<PathBuf>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum QueueItem {
    Track(PathBuf),
    PlayList(VecDeque<QueueItem>, QueueOptions),
    Album(VecDeque<PathBuf>, QueueOptions),
}

impl QueueItem {
    pub fn flatten(self) -> VecDeque<PathBuf> {
        match self {
            QueueItem::Track(track) => [track].into(),
            QueueItem::PlayList(playlist, _) => {
                playlist.into_iter().flat_map(|i| i.flatten()).collect()
            }
            QueueItem::Album(album, _) => album,
        }
    }

    pub fn count(&self) -> u32 {
        match self {
            QueueItem::Track(_) => 1,
            QueueItem::PlayList(playlist, _) => playlist.iter().map(|x| x.count()).sum(),
            QueueItem::Album(album, _) => album.len() as u32,
        }
    }

    /// Should return if a item is fully played
    pub fn is_empty(&self) -> bool {
        match self {
            // if a track is selected than it has been played
            QueueItem::Track(_) => true,
            QueueItem::PlayList(playlist, options) => {
                if playlist.is_empty() {
                    return true;
                }
                // we always repeat so we are never empty
                if options.repeat {
                    return false;
                }
                // We have 1 item left, if that is empty we are also empty
                if playlist.len() == 1 {
                    return playlist[0].is_empty();
                }
                false
            }
            QueueItem::Album(album, options) => {
                if album.is_empty() {
                    return true;
                }
                // we always repeat so we are never empty
                if options.repeat {
                    return false;
                }
                if album.len() == 1 {
                    return true;
                }
                false
            }
        }
    }

    pub fn get_selected(&self) -> Option<PathBuf> {
        match self {
            QueueItem::Track(track) => Some(track.clone()),
            QueueItem::PlayList(playlist, options) => {
                let item = playlist.get(options.selected?)?;
                item.get_selected()
            }
            QueueItem::Album(album, options) => album.get(options.selected?).cloned(),
        }
    }

    /// Set QueueOptions only for this item, not the children
    pub fn set_queue_options(self, options: QueueOptions) -> Self {
        match self {
            QueueItem::PlayList(vec_deque, _) => QueueItem::PlayList(vec_deque, options),
            QueueItem::Album(vec_deque, _) => QueueItem::Album(vec_deque, options),
            track => track,
        }
    }

    /// Set QueueOptions for this item, and the children
    pub fn set_queue_options_rec(self, options: QueueOptions) -> Self {
        match self {
            QueueItem::PlayList(vec_deque, _) => QueueItem::PlayList(
                vec_deque
                    .into_iter()
                    .map(|x| x.set_queue_options_rec(options.clone()))
                    .collect(),
                options,
            ),
            QueueItem::Album(vec_deque, _) => QueueItem::Album(vec_deque, options),
            track => track,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct QueueOptions {
    pub shuffel_type: ShuffelType,
    pub repeat: bool,
    pub selected: Option<usize>,
}

impl Default for QueueOptions {
    fn default() -> Self {
        Self {
            shuffel_type: ShuffelType::None,
            repeat: false,
            selected: None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ShuffelType {
    None,
    TrueRandom,
    /// List of weights to avoid duplicates
    WeightedRandom(Vec<usize>),
    /// List of default weights
    WeightedDefault(Vec<usize>),
    /// List of weights with defaults that don't change
    /// and weights to avoid duplicates
    WeightedRandomWithDefault(Vec<usize>, Vec<usize>),
}

impl ShuffelType {
    pub fn new_weighted_random(size: usize) -> Self {
        Self::WeightedRandom(vec![1; size])
    }
    pub fn new_weighted_random_default(default_weights: Vec<usize>) -> Self {
        let weights = vec![1; default_weights.len()];
        Self::WeightedRandomWithDefault(weights, default_weights)
    }
}

impl Queue {
    pub fn new() -> Queue {
        let queue_items = VecDeque::new();
        let played_items = VecDeque::new();
        let queue_options = Default::default();
        let repeat_current = false;
        Queue {
            queue_items,
            played_items,
            queue_options,
            repeat_current,
            current_track: None,
        }
    }

    ///TODO: make sure all shuffel types work
    pub fn next_track(&mut self) -> Option<PathBuf> {
        if self.repeat_current && self.current_track.is_some() {
            return self.current_track.clone();
        }
        let options = &mut self.queue_options;

        if let Some(track) = self.current_track.clone() {
            self.played_items.push_front(track);
        }
        self.current_track = get_track_from_list(&mut self.queue_items, options, &mut thread_rng());
        self.current_track.clone()
    }

    pub fn queue_items(&self) -> &VecDeque<QueueItem> {
        &self.queue_items
    }

    pub fn played_items(&self) -> &VecDeque<PathBuf> {
        &self.played_items
    }

    pub fn append_track(mut self, track: PathBuf) {
        self.queue_items.push_back(QueueItem::Track(track));
    }

    pub fn append_playlist(mut self, playlist: Vec<QueueItem>, options: QueueOptions) {
        self.queue_items
            .push_back(QueueItem::PlayList(playlist.into(), options))
    }

    pub fn append_album(mut self, album: Vec<PathBuf>, options: QueueOptions) {
        self.queue_items
            .push_back(QueueItem::Album(album.into(), options))
    }

    /// Flattens the queue to only contain tracks
    pub fn flatten(mut self) {
        self.queue_items = self
            .queue_items
            .into_iter()
            .flat_map(|i| i.flatten())
            .map(QueueItem::Track)
            .collect()
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}
