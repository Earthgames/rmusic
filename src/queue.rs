use std::{
    collections::VecDeque,
    fmt::{Debug, Display},
    path::PathBuf,
};

const DEPTH_LIMIT: usize = 10;

pub mod queue_items;
mod select_track;

use log::warn;
// use entity::{artist, release, track, track_location};
use queue_items::{QueueItem, QueueTrack};
use rand::thread_rng;
// pub use select_track::get_track_from_item;
// use select_track::get_track_from_list;

/// Struct that will play things next
#[derive(Debug)]
pub struct Queue {
    pub(crate) queue_items: VecDeque<QueueItem>,
    played_items: VecDeque<QueueItem>,
    max_history: usize,
    pub queue_options: QueueOptions,
    pub repeat_current: bool,
    pub(crate) current_track: Option<PathBuf>,
    next_up: VecDeque<QueueItem>,
    // nothing to do with next_up
    next_id: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub struct QueueOptions {
    pub shuffle_type: ShuffleType,
    pub stop_condition: StopCondition,
    pub selected: Option<usize>,
}

impl Default for QueueOptions {
    fn default() -> Self {
        Self {
            shuffle_type: ShuffleType::None,
            stop_condition: StopCondition::None,
            selected: None,
        }
    }
}

impl QueueOptions {
    /// This is the same as default
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub enum ShuffleType {
    #[default]
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

#[derive(Clone, PartialEq, Debug, Default)]
pub enum StopCondition {
    #[default]
    /// Stop at he end of the queue
    EndOfList,
    /// Loop at the end of the list
    None,
    /// Stop after playing this amount of tracks
    AmountTracks(usize),
    /// Stop playing after this the time in mil elapsed
    Time(u64),
}

impl ShuffleType {
    pub fn new_weighted_random(size: usize) -> Self {
        Self::WeightedRandom(vec![1; size])
    }
    pub fn new_weighted_random_default(default_weights: Vec<usize>) -> Self {
        let weights = vec![1; default_weights.len()];
        Self::WeightedRandomWithDefault(weights, default_weights)
    }
    pub fn display_small(&self) -> &str {
        match self {
            ShuffleType::None => "N",
            ShuffleType::TrueRandom => "TR",
            ShuffleType::WeightedRandom(_) => "WR",
            ShuffleType::WeightedDefault(_) => "WD",
            ShuffleType::WeightedRandomWithDefault(_, _) => "WRD",
        }
    }
}

impl Display for ShuffleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShuffleType::None => write!(f, "None"),
            ShuffleType::TrueRandom => write!(f, "TrueRandom"),
            ShuffleType::WeightedRandom(_) => write!(f, "WeightedRandom"),
            ShuffleType::WeightedDefault(_) => write!(f, "WeightedDefault"),
            ShuffleType::WeightedRandomWithDefault(_, _) => write!(f, "WeightedRandomWithDefault"),
        }
    }
}

#[derive(Debug)]
pub enum ShuffleError {
    /// The length doesn't match
    WrongLength,
}

fn switch_shuffle(
    shuffle: &mut ShuffleType,
    new_shuffle: ShuffleType,
    lenght: usize,
) -> Result<(), ShuffleError> {
    let length_check = |vec: &Vec<usize>| {
        if vec.len() == lenght {
            Ok(())
        } else {
            Err(ShuffleError::WrongLength)
        }
    };
    match new_shuffle {
        ShuffleType::WeightedRandom(ref vec) => length_check(vec)?,
        ShuffleType::WeightedDefault(ref vec) => length_check(vec)?,
        ShuffleType::WeightedRandomWithDefault(ref vec, ref vec1) => {
            length_check(vec)?;
            length_check(vec1)?;
        }
        _ => (),
    }
    *shuffle = new_shuffle;
    Ok(())
}

impl Queue {
    pub fn new() -> Queue {
        // DON'T use default in new if default in turn uses new
        // this causes a funny stack overflow :3
        Default::default()
    }

    pub(crate) fn play_queue_item(
        &mut self,
        queue_item: QueueItem,
        flatten: bool,
    ) -> Option<PathBuf> {
        self.clear_queue();
        self.append_queue_item(queue_item, flatten);
        self.current_track = self.next_track();
        self.current_track.clone()
    }

    // Remove everything that is in the queue
    // NOT the next items, or the played items
    pub fn clear_queue(&mut self) {
        self.queue_items.clear();
        self.queue_options = Default::default();
    }

    pub fn clear_next_items(&mut self) {
        self.next_up.clear();
    }

    pub fn clear_history(&mut self) {
        self.played_items.clear();
    }

    pub fn reset_queue(&mut self) {
        self.clear_queue();
        self.clear_next_items();
        self.clear_history();
    }

    pub fn switch_shuffle(&mut self, new_shuffle: ShuffleType) -> Result<(), ShuffleError> {
        switch_shuffle(
            &mut self.queue_options.shuffle_type,
            new_shuffle,
            self.queue_items.len(),
        )
    }

    pub fn cycle_shuffle(&mut self) {
        self.queue_options.shuffle_type = match &self.queue_options.shuffle_type {
            ShuffleType::None => ShuffleType::TrueRandom,
            ShuffleType::TrueRandom => ShuffleType::new_weighted_random(self.queue_items.len()),
            _ => ShuffleType::None,
        }
    }

    pub fn queue_items(&self) -> &VecDeque<QueueItem> {
        &self.queue_items
    }

    pub fn played_items(&self) -> &VecDeque<QueueItem> {
        &self.played_items
    }

    pub fn current_track(&self) -> &Option<PathBuf> {
        &self.current_track
    }

    pub fn append_queue_item<I>(&mut self, item: I, flatten: bool)
    where
        I: Into<QueueItem>,
    {
        let item: QueueItem = item.into();
        if flatten {
            let tracks = item.flatten();
            for track in tracks {
                self.queue_items.push_back(track.into());
            }
        } else {
            self.queue_items.push_back(item)
        }
    }

    /// Flattens the queue to only contain tracks
    pub fn flatten(mut self) {
        self.queue_items = self
            .queue_items
            .into_iter()
            .flat_map(|i| i.flatten())
            .map(QueueItem::Track)
            .collect();
        // INFO:
        // make this an option, or think if this should happen at all
        //
        // self.played_items = self
        //     .played_items
        //     .into_iter()
        //     .flat_map(|i| i.flatten())
        //     .map(QueueItem::Track)
        //     .collect()
    }
}

impl Default for Queue {
    fn default() -> Self {
        Queue {
            max_history: 128,
            repeat_current: false,
            current_track: None,
            next_id: 0,
            queue_items: VecDeque::new(),
            played_items: VecDeque::new(),
            queue_options: QueueOptions::default(),
            next_up: Default::default(),
        }
    }
}
