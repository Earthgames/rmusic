use std::sync::mpsc::Receiver;

use cpal::Sample;
use log::{error, info};

use crate::{playback::PlaybackDaemon, queue::queue_items::QueueItem};

#[derive(Debug)]
pub enum PlaybackAction {
    Playing,
    Paused,
    /// Toggle between playing and paused
    PlayPause,
    //TODO: think about if using seconds is the right move
    //
    /// Number of seconds to go back
    Rewind(u64),
    /// Number of seconds to skip
    FastForward(u64),
    /// Number of seconds to go to in a song
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
    while let Ok(status) = rx.try_recv() {
        info!(target: "rmusic::playback_loop", "Received: {:?}", status);
        match status {
            PlaybackAction::Playing => playback_daemon.playing = true,
            PlaybackAction::Paused => playback_daemon.playing = false,
            PlaybackAction::PlayPause => playback_daemon.playing = !playback_daemon.playing,
            PlaybackAction::GoTo(target) => playback_daemon
                .goto(target * playback_daemon.sample_rate_input() as u64)
                .unwrap_or_else(|err| error!("Error in Stream: {:?}", err)),
            PlaybackAction::FastForward(amount) => {
                let current = playback_daemon.current_length() - playback_daemon.left();
                let target = current + amount * playback_daemon.sample_rate_input() as u64;
                let goto = if target <= playback_daemon.current_length() {
                    target
                } else {
                    //TODO: replace with next track
                    playback_daemon.current_length() - 1
                };
                playback_daemon
                    .goto(goto)
                    .unwrap_or_else(|err| error!("Error in Stream: {:?}", err))
            }
            PlaybackAction::Rewind(amount) => {
                let current = playback_daemon.current_length() - playback_daemon.left();
                let amount = amount * playback_daemon.sample_rate_input() as u64;
                let goto = current.saturating_sub(amount);
                playback_daemon
                    .goto(goto)
                    .unwrap_or_else(|err| error!("Error in Stream: {:?}", err))
            }
            //TODO: Change this to include the flatten option
            PlaybackAction::Play(item) => playback_daemon
                .play(item, true)
                .unwrap_or_else(|err| error!("Error in Stream: {:?}", err)),
            PlaybackAction::SetVolume(volume) => playback_daemon.set_volume(volume),
            PlaybackAction::ChangeVolume(change) => playback_daemon.change_volume(change),
            _ => unimplemented!(),
        }
    }
    if playback_daemon.playing {
        playback_daemon.fill(data).unwrap_or_else(|err| {
            error!("Error in Stream: {:?}", err);
        });
    } else {
        for i in data.iter_mut() {
            *i = Sample::EQUILIBRIUM;
        }
    }
}
