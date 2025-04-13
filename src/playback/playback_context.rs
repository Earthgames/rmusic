use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use log::error;

use crate::queue::Queue;

pub type ArcPlaybackContext = Arc<PlaybackContext>;

pub struct PlaybackContext {
    pub queue: Mutex<Queue>,
    left: AtomicU64,
    length: AtomicU64,
    sample_rate: AtomicUsize,
}

impl PlaybackContext {
    pub(crate) fn new() -> ArcPlaybackContext {
        let queue = Mutex::new(Queue::new());
        let left = AtomicU64::new(0);
        let length = AtomicU64::new(0);
        let sample_rate = AtomicUsize::new(0);
        Arc::new(PlaybackContext {
            queue,
            left,
            length,
            sample_rate,
        })
    }
    pub(crate) fn new_from(length: u64, track: PathBuf, sample_rate: usize) -> ArcPlaybackContext {
        let mut queue = Queue::new();
        queue.current_track = Some(track);
        let queue = Mutex::new(queue);
        let left = AtomicU64::new(length);
        let length = AtomicU64::new(length);
        let sample_rate = AtomicUsize::new(sample_rate);
        Arc::new(PlaybackContext {
            queue,
            left,
            length,
            sample_rate,
        })
    }
    pub(crate) fn update_left(&self, left: u64) {
        self.left.store(left, Ordering::Relaxed)
    }
    pub(crate) fn lock_queue(&self) -> std::sync::MutexGuard<'_, Queue> {
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
}
