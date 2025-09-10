use log::{error, warn};
use rand::distributions::{Distribution, WeightedError, WeightedIndex};

use std::{collections::VecDeque, path::PathBuf};

use crate::queue::DEPTH_LIMIT;

use super::{queue_items::QueueTrack, Queue, QueueItem, QueueOptions, ShuffleType};

impl Queue {
    //TODO: make sure all shuffle types work
    pub(crate) fn next_track(&mut self) -> Option<PathBuf> {
        // Case: Repeat Current
        if self.repeat_current && self.current_track.is_some() {
            return self.current_track.clone();
        }
        // Case: Tracks in self.next_up
        if !self.next_up.iter().all(|x| x.is_empty()) {
            fn zero(_: usize, _: &mut QueueOptions) -> usize {
                0
            }
            let track = double_recurse(
                zero,
                &mut self.next_up,
                &mut self.played_items,
                &mut self.queue_options,
            )?;
            return Some(track.location().into());
        }
        // Case: Tracks in self.queue_items
        // Get them with the right randomization, don't remove them

        if !self.queue_items.iter().all(|x| x.is_empty()) {
            fn decide(
                length: usize,
                options: &mut QueueOptions,
            ) -> Result<Option<usize>, SelectError> {
                get_random(length, options, &mut rand::thread_rng())
            }
            match recurse(decide, &mut self.queue_items, &mut self.queue_options) {
                Ok(Some(track)) => {
                    return Some(track.location().into())
                },
                Ok(None) => return None,
                Err(err) => {
                    error!("{:?}", err);
                    return None
                }
            }
        }
        None
    }
}

fn double_recurse(
    decide: fn(usize, &mut QueueOptions) -> usize,
    source: &mut VecDeque<QueueItem>,
    history: &mut VecDeque<QueueItem>,
    options: &mut QueueOptions,
) -> Option<QueueTrack> {
    double_recurse_internal(decide, source, history, options, 0)
}

fn double_recurse_internal(
    decide: fn(usize, &mut QueueOptions) -> usize,
    source: &mut VecDeque<QueueItem>,
    history: &mut VecDeque<QueueItem>,
    options: &mut QueueOptions,
    depth: usize,
) -> Option<QueueTrack> {
    if depth > DEPTH_LIMIT {
        warn!("Reached Depth limit in next_up");
        return None;
    }
    let chosen = decide(source.len(), options);
    match source.get(chosen)? {
        QueueItem::Track(track) => Some(move_track(source, history, track.clone(), chosen)),
        QueueItem::Playlist(playlist) => {
            // Check if it exists in history, else add it to history, recurs into it
            let id = playlist.id();

            let create_index = |history: &mut VecDeque<QueueItem>| {
                // create the playlist in the history
                let id = history.len();
                history.push_back(playlist.empty_clone().into());
                id
            };
            let history_id = find_id(history, id, chosen, create_index);

            let (source_list, options) = get_playlist_item(source, chosen)?;
            let (history_list, _) = get_playlist_item(history, history_id)?;

            double_recurse_internal(decide, source_list, history_list, options, depth + 1)
        }
        QueueItem::Album(album) => {
            // Check if it exists in history, else add it to history, recurs into it
            let id = album.id();

            let create_index = |history: &mut VecDeque<QueueItem>| {
                // create the playlist in the history
                let id = history.len();
                history.push_back(album.empty_clone().into());
                id
            };
            let history_id = find_id(history, id, chosen, create_index);

            let (source_list, options) = get_album_item(source, chosen)?;
            let (history_list, _) = get_album_item(history, history_id)?;

            let chosen = decide(source_list.len(), options);
            let track = source_list.get(chosen)?.clone();
            Some(move_track(source_list, history_list, track, chosen))
        }
    }
}

/// Remove queue_track from source, and add it to history, also return it
fn move_track<T>(
    source: &mut VecDeque<T>,
    history: &mut VecDeque<T>,
    track: QueueTrack,
    index: usize,
) -> QueueTrack
where
    QueueTrack: Into<T>,
{
    source.remove(index);
    let track = track.clone();
    history.push_back(track.clone().into());
    track
}

fn recurse(
    decide: fn(usize, &mut QueueOptions) -> Result<Option<usize>, SelectError>,
    source: &mut VecDeque<QueueItem>,
    options: &mut QueueOptions,
) -> Result<Option<QueueTrack>, SelectError> {
    recurse_internal(decide, source, options, 0)
}

fn recurse_internal(
    decide: fn(usize, &mut QueueOptions) -> Result<Option<usize>, SelectError>,
    source: &mut VecDeque<QueueItem>,
    options: &mut QueueOptions,
    depth: usize,
) -> Result<Option<QueueTrack>, SelectError> {
    if depth > DEPTH_LIMIT {
        return Err(SelectError::MaxDepthReached);
    }
    let chosen = match decide(source.len(), options)? {
        Some(index) => index,
        // can't do anything with this
        None => return Ok(None),
    };
    let item = match source.get_mut(chosen) {
        Some(item) => item,
        //WARN: Error?
        None => return Ok(None),
    };
    match item {
        QueueItem::Track(queue_track) => Ok(Some(queue_track.clone())),
        QueueItem::Playlist(queue_playlist) => {
            // recurse
            let source = &mut queue_playlist.playlist_items;
            if source.is_empty() {
                return Ok(None);
            }
            let options = &mut queue_playlist.queue_option;
            for _ in 0..DEPTH_LIMIT {
                let result = recurse_internal(decide, source, options, depth + 1)?;
                // On None we retry
                if let Some(track) = result {
                    return Ok(Some(track));
                }
            }
            Ok(None)
        }
        QueueItem::Album(queue_album) => {
            let source = &mut queue_album.tracks;
            let options = &mut queue_album.queue_option;
            if source.is_empty() {
                return Ok(None);
            }
            let chosen = decide(source.len(), options)?;
            match chosen {
                Some(index) => Ok(source.get(index).cloned()),
                None => Ok(None),
            }
        }
    }
}

fn get_album_item(
    list: &mut VecDeque<QueueItem>,
    index: usize,
) -> Option<(&mut VecDeque<QueueTrack>, &mut QueueOptions)> {
    match list.get_mut(index) {
        Some(QueueItem::Album(album)) => Some((&mut album.tracks, &mut album.queue_option)),
        _ => None,
    }
}

fn get_playlist_item(
    list: &mut VecDeque<QueueItem>,
    index: usize,
) -> Option<(&mut VecDeque<QueueItem>, &mut QueueOptions)> {
    match list.get_mut(index) {
        Some(QueueItem::Playlist(playlist)) => {
            Some((&mut playlist.playlist_items, &mut playlist.queue_option))
        }
        _ => None,
    }
}

fn find_id<F>(
    list: &mut VecDeque<QueueItem>,
    id: usize,
    fast_index: usize,
    create_index: F,
) -> usize
where
    F: FnOnce(&mut VecDeque<QueueItem>) -> usize,
{
    match list.get(fast_index) {
        Some(item) if item.has_id(id) => return fast_index,
        _ => (),
    }
    let mut found = None;
    // we go backwards because there is a bigger chance it is in the back
    for (i, e) in list.iter().enumerate().rev() {
        if e.has_id(id) {
            found = Some(i);
            break;
        }
    }
    match found {
        Some(i) => i,
        None => create_index(list),
    }
}

#[derive(Debug)]
enum SelectError {
    /// The underlying weight library errored
    Weight(WeightedError),
    /// The lenght of a weight list was wrong
    /// (inside the QueueOptions)
    SizeError,
    /// The Depth limit was reached when traversing a playlist
    MaxDepthReached,
}

impl From<WeightedError> for SelectError {
    fn from(value: WeightedError) -> Self {
        SelectError::Weight(value)
    }
}

/// Logic for the QueueOptions and thus ShuffelType
fn get_random<R>(
    list_len: usize,
    options: &mut QueueOptions,
    rng: &mut R,
) -> Result<Option<usize>, SelectError>
where
    R: RandTrack,
{
    check_weight_length(&options.shuffle_type, list_len)?;
    let chosen = match &mut options.shuffle_type {
        ShuffleType::None => match options.selected {
            Some(index) => {
                // Get next item
                let next = index + 1;
                if next < list_len {
                    Some(next)
                } else {
                    None
                }
            }
            // First time in this list, so select first item
            None => Some(0),
        },
        ShuffleType::TrueRandom => {
            let chosen = rng.gen_range(list_len);
            Some(chosen)
        }
        ShuffleType::WeightedRandom(weights) => {
            let chosen = rng.select_weights(weights)?;
            Some(increase_weights(weights, chosen))
        }
        ShuffleType::WeightedDefault(weights) => Some(rng.select_weights(weights)?),
        ShuffleType::WeightedRandomWithDefault(changing_weights, default_weights) => {
            if changing_weights.len() != default_weights.len() {
                return Err(SelectError::SizeError);
            } else {
                // add weights
                let weights = changing_weights
                    .iter()
                    .zip(default_weights.iter())
                    .map(|(a, b)| *a + *b)
                    .collect::<Vec<_>>();
                let chosen = rng.select_weights(&weights)?;
                Some(increase_weights(changing_weights, chosen))
            }
        }
    };
    options.selected = chosen;
    Ok(chosen)
}

/// Increase all weights, and reset the chosen to 0
fn increase_weights(weights: &mut [usize], chosen: usize) -> usize {
    weights.iter_mut().for_each(|w| *w += 1);
    if let Some(x) = weights.get_mut(chosen) {
        *x = 0;
    }
    chosen
}

fn check_weight_length(shuffle: &ShuffleType, list_len: usize) -> Result<(), SelectError> {
    fn check(list: &Vec<usize>, list_len: usize) -> Result<(), SelectError> {
        if list.len() != list_len {
            Err(SelectError::SizeError)
        } else {
            Ok(())
        }
    }
    match shuffle {
        ShuffleType::None => Ok(()),
        ShuffleType::TrueRandom => Ok(()),
        ShuffleType::WeightedRandom(vec) => check(vec, list_len),
        ShuffleType::WeightedDefault(vec) => check(vec, list_len),
        ShuffleType::WeightedRandomWithDefault(vec, vec1) => {
            check(vec, list_len)?;
            check(vec1, list_len)
        }
    }
}

pub trait RandTrack {
    fn gen_range(&mut self, end: usize) -> usize;
    fn select_weights(&mut self, weights: &[usize]) -> Result<usize, WeightedError>;
}

impl<R> RandTrack for R
where
    R: rand::Rng,
{
    fn gen_range(&mut self, end: usize) -> usize {
        self.gen_range(0..end)
    }

    fn select_weights(&mut self, weights: &[usize]) -> Result<usize, WeightedError> {
        // select_weights(weights, self)
        let dist = WeightedIndex::new(weights)?;
        let chosen = dist.sample(self);
        Ok(chosen)
    }
}
