use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use atomic_float::AtomicF32;
use log::error;

use crate::queue::Queue;

use super::BuF;

pub type ArcPlaybackContext = Arc<PlaybackContext>;

pub struct PlaybackContext {
    pub queue: Mutex<Queue>,
    left: AtomicU64,
    length: AtomicU64,
    sample_rate: AtomicUsize,
    volume_level: AtomicF32,
}

impl PlaybackContext {
    pub(crate) fn new() -> ArcPlaybackContext {
        let queue = Mutex::new(Queue::new());
        let left = AtomicU64::new(0);
        let length = AtomicU64::new(0);
        let sample_rate = AtomicUsize::new(0);
        let volume_level = AtomicF32::new(100.0);
        Arc::new(PlaybackContext {
            queue,
            left,
            length,
            sample_rate,
            volume_level,
        })
    }

    pub(crate) fn new_from(
        length: u64,
        track: PathBuf,
        sample_rate: usize,
        volume_level: BuF,
    ) -> ArcPlaybackContext {
        let mut queue = Queue::new();
        queue.current_track = Some(track);
        let queue = Mutex::new(queue);
        let left = AtomicU64::new(length);
        let length = AtomicU64::new(length);
        let sample_rate = AtomicUsize::new(sample_rate);
        let volume_level = AtomicF32::new(volume_level);
        Arc::new(PlaybackContext {
            queue,
            left,
            length,
            sample_rate,
            volume_level,
        })
    }

    pub(crate) fn update_left(&self, left: u64) {
        self.left.store(left, Ordering::Relaxed)
    }

    pub fn update_volume_level(&self, volume_level: BuF) {
        self.volume_level
            .store(volume_level.max(0.0), Ordering::Relaxed);
    }

    pub fn change_volume_level(&self, volume_change: BuF) {
        let new = self
            .volume_level
            .fetch_add(volume_change, Ordering::Relaxed)
            + volume_change;
        self.volume_level.store(new.max(0.0), Ordering::Relaxed);
    }

    pub fn lock_queue(&self) -> std::sync::MutexGuard<'_, Queue> {
        match self.queue.lock() {
            Ok(queue) => queue,
            Err(err) => {
                error!("Queue Poisoned: {err}, creating new queue");
                let mut lock = err.into_inner();
                *lock = Queue::new();
                lock
            }
        }
    }

    pub(crate) fn set_track(&self, track: PathBuf, length: u64, sample_rate: usize) {
        self.length.store(length, Ordering::Relaxed);
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
        let mut queue = self.lock_queue();
        queue.current_track = Some(track);
    }

    pub fn current_track(&self) -> Option<PathBuf> {
        self.lock_queue().current_track.clone()
    }
    pub fn left(&self) -> u64 {
        self.left.load(Ordering::Relaxed)
    }
    pub fn length(&self) -> u64 {
        self.length.load(Ordering::Relaxed)
    }
    pub fn sample_rate(&self) -> usize {
        self.sample_rate.load(Ordering::Relaxed)
    }
    pub fn volume_level(&self) -> BuF {
        self.volume_level.load(Ordering::Relaxed)
    }
    pub fn played(&self) -> u64 {
        self.length().saturating_sub(self.left())
    }
    pub fn played_sec(&self) -> u64 {
        self.some_sec(self.played())
    }
    pub fn length_sec(&self) -> u64 {
        self.some_sec(self.length())
    }
    pub fn left_sec(&self) -> u64 {
        self.some_sec(self.left())
    }
    fn some_sec(&self, sample_count: u64) -> u64 {
        let sample_rate = self.sample_rate();
        if sample_rate == 0 {
            0
        } else {
            sample_count / sample_rate as u64
        }
    }
}
