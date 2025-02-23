use std::{collections::VecDeque, path::PathBuf};

mod select_track;

pub use select_track::get_track_from_item;

/// Struct that will play things next
#[derive(Clone, PartialEq, Debug)]
pub struct Queue {
    pub queue_items: VecDeque<QueueItem>,
    pub queue_options: QueueOptions,
    pub repeat_current: bool,
    pub current_track: Option<PathBuf>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum QueueItem {
    Track(PathBuf, bool),
    PlayList(VecDeque<QueueItem>, QueueOptions),
    Album(VecDeque<PathBuf>, QueueOptions),
}

impl QueueItem {
    pub fn flatten(self) -> VecDeque<PathBuf> {
        match self {
            QueueItem::Track(track, played) => {
                if played {
                    [].into()
                } else {
                    [track].into()
                }
            }
            QueueItem::PlayList(mut playlist, op) => {
                let i = if op.repeat {
                    0
                } else {
                    op.selected.unwrap_or(0)
                };
                playlist.drain(i..).flat_map(|i| i.flatten()).collect()
            }
            QueueItem::Album(mut album, op) => {
                let i = if op.repeat {
                    0
                } else {
                    op.selected.unwrap_or(0)
                };
                album.drain(i..).collect()
            }
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
    pub played: usize,
}

impl Default for QueueOptions {
    fn default() -> Self {
        Self {
            shuffel_type: ShuffelType::None,
            repeat: false,
            selected: None,
            played: 0,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
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
        let queue_options = Default::default();
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
            &mut self.queue_items,
            &mut self.queue_options,
        )?);
        self.current_track
    }

    pub fn append_track(mut self, track: PathBuf) {
        self.queue_items.push_back(QueueItem::Track(track, false));
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
            .map(|path| QueueItem::Track(path, false))
            .collect()
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}
