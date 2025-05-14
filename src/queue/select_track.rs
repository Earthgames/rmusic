use log::error;
use rand::distributions::{Distribution, WeightedIndex};

use std::{collections::VecDeque, path::PathBuf};

use super::{QueueItem, QueueOptions, ShuffleType};

pub fn get_track_from_list<R>(
    track_list: &mut VecDeque<QueueItem>,
    options: &mut QueueOptions,
    rng: &mut R,
) -> Option<PathBuf>
where
    R: RandTrack,
{
    let new_index = match options.selected {
        Some(index) if !track_list.get(index)?.is_empty() => index,
        _ => {
            let new_index = get_random(track_list, options, rng);
            options.selected = new_index;
            new_index?
        }
    };
    let new_track = get_track_from_item(&mut track_list[new_index], rng);
    if track_list.get(new_index)?.is_empty() && !options.repeat {
        options.selected = None;
        remove_item(track_list, options, new_index);
    }
    new_track
}

fn remove_item<T>(list: &mut VecDeque<T>, options: &mut QueueOptions, index: usize) -> Option<T> {
    match options.shuffle_type {
        ShuffleType::WeightedRandom(ref mut vec) => {
            vec.remove(index);
        }
        ShuffleType::WeightedDefault(ref mut vec) => {
            vec.remove(index);
        }
        ShuffleType::WeightedRandomWithDefault(ref mut vec, ref mut vec1) => {
            vec.remove(index);
            vec1.remove(index);
        }
        _ => (),
    }
    list.remove(index)
}

pub fn get_track_from_item<R>(queue_item: &mut QueueItem, rng: &mut R) -> Option<PathBuf>
where
    R: RandTrack,
{
    match queue_item {
        QueueItem::Track(track) => Some(track.to_path_buf()),
        QueueItem::PlayList(playlist, options) => get_track_from_list(playlist, options, rng),
        QueueItem::Album(album, options) => {
            let index = get_random(album, options, rng)?;
            if options.repeat {
                options.selected = Some(index);
                album.get(index).cloned()
            } else {
                options.selected = None;
                remove_item(album, options, index)
            }
        }
    }
}

// Logic for the QueueOptions and thus ShuffelType
fn get_random<T, R>(
    list: &mut VecDeque<T>,
    options: &mut QueueOptions,
    rng: &mut R,
) -> Option<usize>
where
    T: Clone,
    R: RandTrack,
{
    if list.is_empty() {
        return None;
    }
    match &mut options.shuffle_type {
        ShuffleType::None => match options.selected {
            Some(index) => {
                let next = index + 1;
                if next < list.len() {
                    Some(next)
                } else if options.repeat {
                    Some(0)
                } else {
                    None
                }
            }
            None => Some(0),
        },
        ShuffleType::TrueRandom => {
            let chosen = rng.gen_range(list.len());
            Some(chosen)
        }
        ShuffleType::WeightedRandom(weights) => {
            let chosen = rng.select_weights(weights);
            increase_weights(weights, chosen)
        }
        ShuffleType::WeightedDefault(weights) => rng.select_weights(weights),
        ShuffleType::WeightedRandomWithDefault(changing_weights, default_weights) => {
            if changing_weights.len() != default_weights.len() {
                error!("Weights are not the same lenght");
                return None;
            }
            let weights = changing_weights
                .iter()
                .zip(default_weights.iter())
                .map(|(a, b)| *a + *b)
                .collect::<Vec<_>>();
            let chosen = rng.select_weights(&weights);
            increase_weights(changing_weights, chosen)
        }
    }
}

fn increase_weights(weights: &mut [usize], chosen: Option<usize>) -> Option<usize> {
    weights.iter_mut().for_each(|w| *w += 1);
    if let Some(x) = weights.get_mut(chosen?) {
        *x = 0;
    }
    chosen
}

fn select_weights<R>(weights: &[usize], mut rng: R) -> Option<usize>
where
    R: rand::Rng,
{
    let dist = match WeightedIndex::new(weights) {
        Ok(d) => d,
        Err(err) => {
            error!("Error while making weights: {err}");
            return None;
        }
    };
    let chosen = dist.sample(&mut rng);
    Some(chosen)
}

pub trait RandTrack {
    fn gen_range(&mut self, end: usize) -> usize;
    fn select_weights(&mut self, weights: &[usize]) -> Option<usize>;
}

impl<R> RandTrack for R
where
    R: rand::Rng,
{
    fn gen_range(&mut self, end: usize) -> usize {
        self.gen_range(0..end)
    }

    fn select_weights(&mut self, weights: &[usize]) -> Option<usize> {
        select_weights(weights, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRand {
        current: usize,
        increment: usize,
    }
    impl RandTrack for MockRand {
        fn gen_range(&mut self, end: usize) -> usize {
            let chosen = self.current % end;
            self.current += self.increment;
            chosen
        }

        fn select_weights(&mut self, weights: &[usize]) -> Option<usize> {
            let max = weights.iter().max()?;
            for (i, x) in weights.iter().enumerate() {
                if x == max {
                    return Some(i);
                }
            }
            None
        }
    }

    fn test_queue() -> VecDeque<QueueItem> {
        [
            "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
        ]
        .iter()
        .map(|s| QueueItem::Track(PathBuf::from(s)))
        .collect()
    }

    fn simple_rand() -> MockRand {
        MockRand {
            current: 1,
            increment: 1,
        }
    }

    fn path(str: &str) -> PathBuf {
        PathBuf::from(str)
    }

    #[test]
    fn test_none_one() {
        let mut list = test_queue();
        let mut options = QueueOptions {
            shuffle_type: ShuffleType::None,
            repeat: false,
            selected: None,
        };
        let mut rng = simple_rand();
        let result = get_track_from_list(&mut list, &mut options, &mut rng);
        assert_eq!(result, Some(path("Zero")));
        let result = get_track_from_list(&mut list, &mut options, &mut rng);
        assert_eq!(result, Some(path("One")));
        let result = get_track_from_list(&mut list, &mut options, &mut rng);
        assert_eq!(result, Some(path("Two")));
    }

    #[test]
    fn test_none_two() {
        let mut list = test_queue();
        let mut options = QueueOptions {
            shuffle_type: ShuffleType::None,
            repeat: false,
            selected: None,
        };
        let mut rng = simple_rand();
        get_track_from_list(&mut list, &mut options, &mut rng);
        get_track_from_list(&mut list, &mut options, &mut rng);
        get_track_from_list(&mut list, &mut options, &mut rng);
        let mut left = test_queue();
        left.drain(..3);
        assert_eq!(list, left)
    }

    #[test]
    fn test_none_three() {
        let mut list = test_queue();
        let mut options = QueueOptions {
            shuffle_type: ShuffleType::None,
            repeat: true,
            selected: None,
        };
        let mut rng = simple_rand();
        get_track_from_list(&mut list, &mut options, &mut rng);
        get_track_from_list(&mut list, &mut options, &mut rng);
        get_track_from_list(&mut list, &mut options, &mut rng);
        let left = test_queue();
        assert_eq!(list, left);
        assert_eq!(options.selected, Some(2))
    }

    #[test]
    fn test_true_random_one() {
        let mut list = test_queue();
        let mut options = QueueOptions {
            shuffle_type: ShuffleType::TrueRandom,
            repeat: true,
            selected: None,
        };
        let mut rng = simple_rand();
        get_track_from_list(&mut list, &mut options, &mut rng);
        get_track_from_list(&mut list, &mut options, &mut rng);
        get_track_from_list(&mut list, &mut options, &mut rng);
        let left = test_queue();
        assert_eq!(list, left);
        assert_eq!(options.selected, Some(3))
    }
}
