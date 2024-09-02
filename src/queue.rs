use rand::{
    distributions::{Distribution, WeightedIndex},
    random, thread_rng, Rng,
};
use std::{collections::VecDeque, path::Path};

/// Struct that will play things next
pub struct Queue<'a> {
    queu_items: VecDeque<QueueItem<'a>>,
    pub queu_options: QueueOptions,
    pub repeat_current: bool,
    pub current_track: Option<&'a Path>,
}

#[derive(Clone)]
pub enum QueueItem<'a> {
    Track(&'a Path),
    PlayList((VecDeque<QueueItem<'a>>, QueueOptions)),
    Album((VecDeque<&'a Path>, QueueOptions)),
}

#[derive(Clone)]
pub struct QueueOptions {
    pub shuffel_type: ShuffelType,
    pub repeat: bool,
}

#[derive(Clone)]
enum ShuffelType {
    None,
    TrueRandom,
    /// List of weights
    WeightedRandom(Vec<usize>),
}

impl<'a> Queue<'a> {
    pub fn new() -> Queue<'a> {
        let queu_items = VecDeque::new();
        let queu_options = QueueOptions {
            shuffel_type: ShuffelType::None,
            repeat: false,
        };
        let repeat_current = false;
        Queue {
            queu_items,
            queu_options,
            repeat_current,
            current_track: None,
        }
    }

    pub fn next_track(mut self) -> Option<&'a Path> {
        if self.repeat_current {
            return self.current_track;
        }
        self.current_track = Some(Self::get_track_from_list(
            self.queu_items,
            &self.queu_options,
        )?);
        self.current_track
    }

    fn get_track_from_list(
        track_list: VecDeque<QueueItem<'a>>,
        options: &QueueOptions,
    ) -> Option<&'a Path> {
        let chosen = Self::get_random(track_list, options)?;
        Self::get_track_from_item(chosen)
    }

    fn get_track_from_item(queu_item: QueueItem<'a>) -> Option<&'a Path> {
        match queu_item {
            QueueItem::Track(track) => Some(track),
            QueueItem::PlayList(playlist) => Self::get_track_from_list(playlist.0, &playlist.1),
            QueueItem::Album(album) => Self::get_random(album.0, &album.1),
        }
    }

    fn get_random<T>(mut list: VecDeque<T>, options: &QueueOptions) -> Option<T>
    where
        T: Clone,
    {
        if list.is_empty() {
            return None;
        }
        match &options.shuffel_type {
            ShuffelType::None => {
                if options.repeat {
                    list.rotate_left(1);
                    list.back().cloned()
                } else {
                    list.pop_front()
                }
            }
            ShuffelType::TrueRandom => {
                let chosen = thread_rng().gen_range(0..list.len());
                if options.repeat {
                    Some(list[chosen].clone())
                } else {
                    list.remove(chosen)
                }
            }
            ShuffelType::WeightedRandom(weights) => {
                let dist = WeightedIndex::new(weights).expect("List is Probably empty");
                let chosen = dist.sample(&mut thread_rng());
                if options.repeat {
                    let result = list.remove(chosen).expect("Weird logic");
                    list.push_back(result.clone());
                    Some(result)
                } else {
                    list.remove(chosen)
                }
            }
        }
    }
}
