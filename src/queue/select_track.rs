use log::error;
use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng, Rng,
};

use std::{collections::VecDeque, path::PathBuf};

use super::{QueueItem, QueueOptions, ShuffelType};

pub fn get_track_from_list(
    track_list: &mut VecDeque<QueueItem>,
    options: &mut QueueOptions,
) -> Option<PathBuf> {
    if let Some(index) = options.selected {
        if let Some(track) = get_track_from_item(track_list.get_mut(index)?) {
            return Some(track);
        }
    }
    // if we can't get a new index we fail
    let index = options.get_next(track_list.len())?;
    get_track_from_item(track_list.get_mut(index)?)
}

pub fn get_track_from_item(queue_item: &mut QueueItem) -> Option<PathBuf> {
    match queue_item {
        QueueItem::Track(track, played) => {
            if *played {
                None
            } else {
                Some(track.to_path_buf())
            }
        }
        QueueItem::PlayList(playlist, options) => get_track_from_list(playlist, options),
        QueueItem::Album(album, options) => {
            let selected = options.get_next(album.len());
            selected.and_then(|index| album.get(index).cloned())
        }
    }
}

impl QueueOptions {
    // Logic for the QueueOptions and thus ShuffelType
    fn get_next(&mut self, list_len: usize) -> Option<usize> {
        if list_len == 0 {
            return None;
        }

        let mut next_played = |chosen: Option<usize>| {
            if (0..list_len - 1).contains(&self.played) {
                self.played += 1;
                chosen
            } else {
                None
            }
        };

        let selected = match self.shuffel_type {
            ShuffelType::None => {
                let next = match self.selected {
                    Some(i) => i + 1,
                    None => {
                        self.selected = Some(0);
                        return Some(0);
                    }
                };
                if self.repeat {
                    Some((next) % (list_len - 1))
                } else if (0..list_len - 1).contains(&next) {
                    Some(next)
                } else {
                    None
                }
            }
            ShuffelType::TrueRandom => {
                let chosen = thread_rng().gen_range(0..list_len);
                if self.repeat {
                    Some(chosen)
                } else {
                    // We play until we have played the amount of songs that are in the list
                    // this means we could have skipped a few, this is intentional
                    next_played(Some(chosen))
                }
            }
            ShuffelType::CustomList(ref mut indexes) => {
                if indexes.len() != list_len {
                    error!("List & indexes missmatch");
                    return None;
                }
                if self.repeat {
                    indexes.rotate_left(1);
                    indexes.back().cloned()
                } else if (0..list_len - 1).contains(&self.played) {
                    self.played += 1;
                    indexes.get(self.played).copied()
                } else {
                    None
                }
            }
            ShuffelType::WeightedRandom(ref mut weights) => {
                if weights.len() != list_len {
                    error!("List & weights missmatch");
                    return None;
                }
                let start = if self.repeat {
                    0
                } else if (0..list_len - 1).contains(&self.played) {
                    self.played
                } else {
                    return None;
                };
                let dist = match WeightedIndex::new(weights[start..].iter()) {
                    Ok(ok) => ok,
                    Err(err) => {
                        error!("Error while creating weights: {err}");
                        return None;
                    }
                };
                let chosen = dist.sample(&mut thread_rng());
                if self.repeat {
                    weights.iter_mut().for_each(|x| *x += 1);
                } else {
                    // we already checked if played was out of bounds
                    weights.iter_mut().for_each(|x| {
                        if *x != 0 {
                            *x += 1;
                        }
                    });
                    self.played += 1;
                }
                *(weights.get_mut(chosen)?) = 0;
                Some(chosen)
            }
        };
        self.selected = selected;
        selected
    }

    pub fn change_type(&mut self, shuffel_type: ShuffelType) {
        self.shuffel_type = shuffel_type;
        self.played = 0;
    }
}
