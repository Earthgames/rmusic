use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng, Rng,
};

use std::{collections::VecDeque, path::PathBuf};

use super::{QueueItem, QueueOptions, ShuffelType};

pub fn get_track_from_list(
    track_list: VecDeque<QueueItem>,
    options: QueueOptions,
) -> Option<PathBuf> {
    let chosen = get_random(track_list, options)?;
    get_track_from_item(chosen)
}

fn get_track_from_item(queue_item: QueueItem) -> Option<PathBuf> {
    match queue_item {
        QueueItem::Track(track) => Some(track),
        QueueItem::PlayList((playlist, options)) => get_track_from_list(playlist, options),
        QueueItem::Album((album, options)) => get_random(album, options),
    }
}

// Logic for the QueueOptions and thus ShuffelType
fn get_random<T>(mut list: VecDeque<T>, options: QueueOptions) -> Option<T>
where
    T: Clone,
{
    if list.is_empty() {
        return None;
    }
    match options.shuffel_type {
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
                list.get(chosen).cloned()
            } else {
                list.remove(chosen)
            }
        }
        ShuffelType::CustomList(mut indexes) => {
            let index = if options.repeat {
                indexes.rotate_left(1);
                indexes.back().cloned()?
            } else {
                indexes.pop_front()?
            };
            list.get(index).cloned()
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
