use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng,
};

use std::{collections::VecDeque, path::PathBuf};

use super::{QueueItem, QueueOptions, ShuffelType};

pub fn get_track_from_list(
    track_list: &mut VecDeque<QueueItem>,
    options: &mut QueueOptions,
) -> Option<PathBuf> {
    // if let Some(index) = options.selected {
    //     if let Some(track) = get_track_from_item(track_list.get_mut(index)?) {
    //         return Some(track);
    //     }
    // }
    // // if we can't get a new index we fail
    // let index = options.get_next(track_list.len())?;
    // get_track_from_item(track_list.get_mut(index)?)
    let mut chosen = get_random(track_list, options, thread_rng())?;
    get_track_from_item(&mut chosen)
}

pub fn get_track_from_item(queue_item: &mut QueueItem) -> Option<PathBuf> {
    match queue_item {
        QueueItem::Track(track, _) => Some(track.to_path_buf()),
        QueueItem::PlayList(playlist, options) => get_track_from_list(playlist, options),
        QueueItem::Album(album, options) => get_random(album, options, thread_rng()),
    }
}

// Logic for the QueueOptions and thus ShuffelType
fn get_random<T, R>(list: &mut VecDeque<T>, options: &mut QueueOptions, mut rng: R) -> Option<T>
where
    T: Clone,
    R: rand::Rng,
{
    if list.is_empty() {
        return None;
    }
    match &mut options.shuffel_type {
        ShuffelType::None => {
            if options.repeat {
                list.rotate_left(1);
                list.back().cloned()
            } else {
                list.pop_front()
            }
        }
        ShuffelType::TrueRandom => {
            let chosen = rng.gen_range(0..list.len());
            if options.repeat {
                list.get(chosen).cloned()
            } else {
                list.remove(chosen)
            }
        }
        ShuffelType::CustomList(ref mut indexes) => {
            let index = if options.repeat {
                indexes.rotate_left(1);
                indexes.back().cloned()?
            } else {
                indexes.pop_front()?
            };
            list.get(index).cloned()
        }
        ShuffelType::WeightedRandom(ref mut weights) => {
            let dist = (WeightedIndex::new(weights.iter())).expect("List is probably empty");
            let chosen = dist.sample(&mut rng);
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
#[cfg(test)]
mod tests {
    use rand::rngs::mock::StepRng;

    use super::*;

    fn test_queue() -> VecDeque<String> {
        [
            "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn simple_rand() -> StepRng {
        StepRng::new(0, 1)
    }

    #[test]
    fn test_none_one() {
        let mut list = test_queue();
        let mut options = QueueOptions {
            shuffel_type: ShuffelType::None,
            repeat: false,
            selected: None,
            played: 0,
        };
        let result = get_random(&mut list, &mut options, simple_rand());
        assert_eq!(result, Some("Zero".to_string()));
        let result = get_random(&mut list, &mut options, simple_rand());
        assert_eq!(result, Some("One".to_string()));
        let result = get_random(&mut list, &mut options, simple_rand());
        assert_eq!(result, Some("Two".to_string()));
    }

    #[test]
    fn test_none_two() {
        let mut list = test_queue();
        let mut options = QueueOptions {
            shuffel_type: ShuffelType::None,
            repeat: false,
            selected: None,
            played: 0,
        };
        get_random(&mut list, &mut options, simple_rand());
        get_random(&mut list, &mut options, simple_rand());
        get_random(&mut list, &mut options, simple_rand());
        let mut left = test_queue();
        left.drain(..3);
        assert_eq!(list, left)
    }

    #[test]
    fn test_none_three() {
        let mut list = test_queue();
        let mut options = QueueOptions {
            shuffel_type: ShuffelType::None,
            repeat: true,
            selected: None,
            played: 0,
        };
        get_random(&mut list, &mut options, simple_rand());
        get_random(&mut list, &mut options, simple_rand());
        get_random(&mut list, &mut options, simple_rand());
        let mut left = test_queue();
        left.rotate_left(3);
        assert_eq!(list, left)
    }
}
