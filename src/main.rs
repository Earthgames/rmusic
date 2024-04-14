use std::fs::File;
use std::io::BufReader;

use clap::Parser;
use cpal::{Sample, SampleRate, SupportedStreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cli::Cli;

use crate::ogg_opus::OpusReader;

mod cli;
mod ogg_opus;
// mod opus;

fn main() {
    let cli = Cli::parse();
    // let mut log_config = simplelog::ConfigBuilder::new();
    // let mut quiet = false;
    // TermLogger::init(
    //     match cli.loglevel {
    //         0 => {
    //             quiet = true;
    //             LevelFilter::Off
    //         }
    //         1 => LevelFilter::Error,
    //         2 => LevelFilter::Warn,
    //         3 => LevelFilter::Info,
    //         4 => LevelFilter::Debug,
    //         _ => LevelFilter::Trace,
    //     },
    //     log_config.set_time_level(LevelFilter::Off).build(),
    //     simplelog::TerminalMode::Stdout,
    //     simplelog::ColorChoice::Auto,
    // ).unwrap();

    // Music file
    let music_file = File::open(cli.opus_file).unwrap();

    // Opus reader
    let mut opus_reader = OpusReader::new(BufReader::new(music_file));

    // Audio output
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available"); // Add log
    let mut supported_configs_range = device.supported_output_configs()
        .expect("error while querying configs");
    let config = supported_configs_range.next().unwrap();
    let supported_config = SupportedStreamConfig::new(2, SampleRate { 0: 48000 }, *config.buffer_size(), config.sample_format())
        ;

    // Stream setup
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
    let decoder = move |data: &mut [f32], callback: &_| decode(data, callback, &mut opus_reader);
    let stream = device.build_output_stream(&supported_config.into(), decoder, err_fn, None).unwrap();
    stream.play().unwrap();


    std::thread::sleep(std::time::Duration::from_millis(10000));
}

// use opus directly
fn decode(data: &mut [f32], _: &cpal::OutputCallbackInfo, opus_reader: &mut OpusReader) {
    opus_reader.fill(data)
}
