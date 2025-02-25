use std::sync::mpsc::Receiver;

use cpal::Sample;
use log::{error, info};

use crate::{playback::PlaybackDaemon, queue::QueueItem};

#[derive(Debug)]
pub enum PlaybackAction {
    Playing,
    Paused,
    /// Toggle between playing and paused
    PlayPause,
    /// Number of samples to go back
    Rewind(u64),
    /// Number of samples to skip
    FastForward(u64),
    /// Number of samples to go to in a song
    GoTo(u64),
    Que(QueueItem),
    /// Play the first item of the QueueItem and set the rest as the queue
    Play(QueueItem),
    /// Set the volume, 1.0 is default
    SetVolume(f32),
    /// Change the volume
    ChangeVolume(f32),
}

pub fn playback_loop(
    data: &mut [f32],
    _callback: &cpal::OutputCallbackInfo,
    playback_daemon: &mut PlaybackDaemon,
    rx: &Receiver<PlaybackAction>,
) {
    if let Ok(status) = rx.try_recv() {
        info!(target: "playback_loop", "Received: {:?}", status);
        match status {
            PlaybackAction::Playing => playback_daemon.playing = true,
            PlaybackAction::Paused => playback_daemon.playing = false,
            PlaybackAction::PlayPause => playback_daemon.playing = !playback_daemon.playing,
            PlaybackAction::GoTo(target) => playback_daemon
                .goto(target * playback_daemon.sample_rate_input() as u64)
                .unwrap_or_else(|err| error!("Error in Stream: {}", err)),
            PlaybackAction::FastForward(amount) => {
                let current = playback_daemon.current_length() - playback_daemon.left();
                let target = current + amount * playback_daemon.sample_rate_input() as u64;
                if target <= playback_daemon.current_length() {
                    playback_daemon
                        .goto(target)
                        .unwrap_or_else(|err| error!("Error in Stream: {}", err))
                }
            }
            PlaybackAction::Rewind(amount) => {
                let current = playback_daemon.current_length() - playback_daemon.left();
                if amount <= current {
                    playback_daemon
                        .goto(current - amount * playback_daemon.sample_rate_input() as u64)
                        .unwrap_or_else(|err| error!("Error in Stream: {}", err))
                }
            }
            PlaybackAction::Play(item) => playback_daemon
                .play(item)
                .unwrap_or_else(|err| error!("Error in Stream: {}", err)),
            PlaybackAction::SetVolume(vol) => playback_daemon.volume_level = vol,
            PlaybackAction::ChangeVolume(change) => playback_daemon.volume_level += change,
            _ => unimplemented!(),
        }
    }
    if playback_daemon.playing {
        playback_daemon.fill(data).unwrap_or_else(|err| {
            error!("Error in Stream: {}", err);
        });
    } else {
        for i in data.iter_mut() {
            *i = Sample::EQUILIBRIUM;
        }
    }
}
