use std::{collections::VecDeque, path::PathBuf};

mod select_track;

/// Struct that will play things next
pub struct Queue {
    pub queue_items: VecDeque<QueueItem>,
    pub queue_options: QueueOptions,
    pub repeat_current: bool,
    pub current_track: Option<PathBuf>,
}

#[derive(Clone)]
pub enum QueueItem {
    Track(PathBuf),
    PlayList(VecDeque<QueueItem>, QueueOptions),
    Album(VecDeque<PathBuf>, QueueOptions),
}

impl QueueItem {
    fn flatten(self) -> VecDeque<PathBuf> {
        match self {
            QueueItem::Track(track) => [track].into(),
            QueueItem::PlayList(playlist, _) => {
                playlist.into_iter().flat_map(|i| i.flatten()).collect()
            }
            QueueItem::Album(album, _) => album,
        }
    }
}

#[derive(Clone)]
pub struct QueueOptions {
    pub shuffel_type: ShuffelType,
    pub repeat: bool,
}

impl Default for QueueOptions {
    fn default() -> Self {
        Self {
            shuffel_type: ShuffelType::None,
            repeat: false,
        }
    }
}

#[derive(Clone)]
pub enum ShuffelType {
    None,
    TrueRandom,
    /// List of weights
    WeightedRandom(Vec<usize>),
    /// List of indexes to use
    CustomList(VecDeque<usize>),
}

impl Queue {
    pub fn new() -> Queue {
        let queue_items = VecDeque::new();
        let queue_options = QueueOptions {
            shuffel_type: ShuffelType::None,
            repeat: false,
        };
        let repeat_current = false;
        Queue {
            queue_items,
            queue_options,
            repeat_current,
            current_track: None,
        }
    }

    pub fn next_track(mut self) -> Option<PathBuf> {
        if self.repeat_current {
            return self.current_track;
        }
        self.current_track = Some(select_track::get_track_from_list(
            self.queue_items,
            self.queue_options,
        )?);
        self.current_track
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
