use std::sync::mpsc::Receiver;

use cpal::Sample;
use log::error;

use crate::playback::{PlaybackAction, PlaybackDaemon};

pub fn playback_loop(
    data: &mut [f32],
    _callback: &cpal::OutputCallbackInfo,
    playback_daemon: &mut PlaybackDaemon,
    rx: &Receiver<PlaybackAction>,
) {
    if let Ok(status) = rx.try_recv() {
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
            PlaybackAction::Play(track) => playback_daemon
                .play(track, vec![])
                .unwrap_or_else(|err| error!("Error in Stream: {}", err)),
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
