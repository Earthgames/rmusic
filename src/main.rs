use std::fs::File;
use std::io::BufReader;

use clap::Parser;
use cpal::{SampleRate, SupportedStreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{error, LevelFilter};
use simplelog::TermLogger;

use cli::Cli;
use rmusic::ogg_opus::OpusReader;

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
    let music_file = exit_on_error!(File::open(cli.opus_file));

    // Opus reader
    let mut opus_reader = exit_on_error!(OpusReader::new(BufReader::new(music_file)));

    // Audio output
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available"); // Add log
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

    // calculate length in millis
    let length = opus_reader.length / 48;

    // Stream setup
    let err_fn = |err| error!("an error occurred on the output audio stream: {}", err);
    let decoder = move |data: &mut [f32], callback: &_| decode(data, callback, &mut opus_reader);
    let stream =
        exit_on_error!(device.build_output_stream(&supported_config.into(), decoder, err_fn, None));
    exit_on_error!(stream.play());

    std::thread::sleep(std::time::Duration::from_millis(length));
}

// use opus directly
fn decode(data: &mut [f32], _callback: &cpal::OutputCallbackInfo, opus_reader: &mut OpusReader) {
    opus_reader
        .fill(data)
        .unwrap_or_else(|err| error!("Error in Stream: {}", err))
}
