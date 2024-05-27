use std::fs::File;
use std::io::{stdin, BufReader};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleRate, SupportedStreamConfig};
use log::{error, LevelFilter};
use simplelog::TermLogger;

use cli::Cli;
use rmusic::decoders::ogg_opus::OpusReader;
use rmusic::decoders::Decoder;
use rmusic::playback::{PlaybackAction, PlaybackDaemon};

mod cli;

macro_rules! exit_on_error {
    ($expr:expr) => {
        match $expr {
            std::result::Result::Ok(val) => val,
            std::result::Result::Err(err) => {
                error!("Exiting because of {}", err);
                std::process::exit(1);
            }
        }
    };
}

fn main() {
    let cli = Cli::parse();
    let mut log_config = simplelog::ConfigBuilder::new();
    let mut _quiet = false;
    TermLogger::init(
        match cli.loglevel {
            0 => {
                _quiet = true;
                LevelFilter::Off
            }
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        },
        log_config.set_time_level(LevelFilter::Off).build(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    // Music file
    let music_file = exit_on_error!(File::open(&cli.opus_file));

    // Opus reader
    let opus_reader = exit_on_error!(OpusReader::new(BufReader::new(music_file)));

    // playback Daemon
    let mut playback_daemon =
        PlaybackDaemon::new(PathBuf::from(cli.opus_file), Decoder::Opus(opus_reader));

    // Audio output
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available"); // Add log

    let supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let mut buff = vec![];
    for config in supported_configs_range {
        buff.append(&mut format!("{:?}", config).as_bytes().to_vec());
        buff.push(b'\n');
    }

    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let config = supported_configs_range.next().unwrap();
    let supported_config = SupportedStreamConfig::new(
        2,
        SampleRate(48000),
        *config.buffer_size(),
        config.sample_format(),
    );

    // Thread communication
    let (tx, rx) = mpsc::channel();

    // Stream setup
    let err_fn = |err| error!("an error occurred on the output audio stream: {}", err);
    let decoder =
        move |data: &mut [f32], callback: &_| decode(data, callback, &mut playback_daemon, &rx);
    let stream =
        exit_on_error!(device.build_output_stream(&supported_config.into(), decoder, err_fn, None));
    exit_on_error!(stream.play());

    let mut command = String::new();
    let stdin = stdin();
    loop {
        command.clear();
        exit_on_error!(stdin.read_line(&mut command)); // Ignore all errors for now
        match command.as_str().trim() {
            "q" => break,
            "p" => exit_on_error!(tx.send(PlaybackAction::Playing)),
            "s" => exit_on_error!(tx.send(PlaybackAction::Paused)),
            "f" => exit_on_error!(tx.send(PlaybackAction::FastForward(240000))),
            _ => continue,
        }
    }
}

fn decode(
    data: &mut [f32],
    _callback: &cpal::OutputCallbackInfo,
    playback_daemon: &mut PlaybackDaemon,
    rx: &Receiver<PlaybackAction>,
) {
    if let Ok(status) = rx.try_recv() {
        match status {
            PlaybackAction::Playing => playback_daemon.playing = true,
            PlaybackAction::Paused => playback_daemon.playing = false,
            PlaybackAction::FastForward(amount) => {
                let mut discard = vec![0.0f32; amount as usize];
                playback_daemon
                    .fill(&mut discard)
                    .unwrap_or_else(|err| error!("Error in Stream: {}", err));
            }
            PlaybackAction::Rewinding(amount) => unimplemented!(),
            _ => unimplemented!(),
        }
    }

    if playback_daemon.playing {
        playback_daemon
            .fill(data)
            .unwrap_or_else(|err| error!("Error in Stream: {}", err))
    } else {
        for i in data.iter_mut() {
            *i = Sample::EQUILIBRIUM;
        }
    }
}
