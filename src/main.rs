pub mod real_time_streaming;

use std::{env, io};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use getopts::Options;

use std::sync::{atomic::Ordering, Arc};

/// SOME GLOBAL CONSTANTS
const BLOCKSIZE: usize = 128;
const BLOCKSIZE_FLOAT: f32 = 128.0;

fn print_help(program: &str, opts: Options) {
    let description = format!(
        "{prog}: a perpetual audio recorder

Usage:
    {prog}
      ",
        prog = program,
    );
    println!("{}", opts.usage(&description));
}

fn main() -> Result<(), anyhow::Error> {
    let mut argv = env::args();
    let program = argv.next().unwrap();

    let mut opts = Options::new();

    opts.optflag("h", "help", "Print this help");
    opts.optflag("v", "version", "Print version");
    opts.optflag("l", "list-devices", "list available audio devices");
    opts.optopt("d", "device", "choose device", "default");

    #[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd"))]
    let host = cpal::host_from_id(cpal::available_hosts()
				  .into_iter()
				  .find(|id| *id == cpal::HostId::Jack)
				  .expect(
				      "make sure --features jack is specified. only works on OSes where jack is available",
				  )).expect("jack host unavailable");

    #[cfg(not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd")))]
    let host = cpal::default_host();

    let matches = match opts.parse(argv) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}. Please see --help for more details", e);
            return Ok(());
        }
    };

    if matches.opt_present("h") {
        print_help(&program, opts);
        return Ok(());
    }

    if matches.opt_present("v") {
        println!("prec - The Perpetual Recorder - Version 0.0.1");
        return Ok(());
    }

    if matches.opt_present("l") {
        for dev in host.output_devices()? {
            println!("out {:?}", dev.name());
        }
        for dev in host.input_devices()? {
            println!("in {:?}", dev.name());
        }
        return Ok(());
    }

    let out_device = if let Some(dev) = matches.opt_str("d") {
        dev
    } else {
        "default".to_string()
    };

    let input_device = if out_device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == out_device).unwrap_or(false))
    }
    .expect("failed to find input device");

    let in_config: cpal::SupportedStreamConfig = input_device.default_input_config().unwrap();

    // let's assume it's the same for both ...
    let sample_format = in_config.sample_format();

    let mut in_conf: cpal::StreamConfig = in_config.into();
    in_conf.channels = 2;
    match sample_format {
        cpal::SampleFormat::F32 => run::<f32, 2>(&input_device, &in_conf)?,
        cpal::SampleFormat::I16 => run::<i16, 2>(&input_device, &in_conf)?,
        cpal::SampleFormat::U16 => run::<u16, 2>(&input_device, &in_conf)?,
    }

    Ok(())
}

fn run<T, const NCHAN: usize>(
    input_device: &cpal::Device,
    in_config: &cpal::StreamConfig,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = in_config.sample_rate.0 as f32;
    let in_channels = in_config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    // INPUT RECORDING
    let (throw_in, catch_in, to_disk) = real_time_streaming::init_real_time_stream::<
        BLOCKSIZE,
        NCHAN,
	>((BLOCKSIZE_FLOAT / sample_rate) as f64, 0.5);

    println!("build input stream");

    // INPUT STREAM CALLBACK
    let in_stream = input_device.build_input_stream(
        in_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut stream_item = throw_in.prep_next().unwrap();
            // there might be a faster way to de-interleave here ...
            for (f, frame) in data.chunks(in_channels).enumerate() {
                for (ch, s) in frame.iter().enumerate() {
                    stream_item.buffer[ch][f] = *s;
                }
                stream_item.size += 1; // increment once per frame
            }
            throw_in.throw_next(stream_item);
        },
        err_fn,
    )?;

    println!("start input stream");
    // start input stream callback
    in_stream.play()?;

    println!("start buffer thread");
    // start thread that handles receiving the audio
    let catch_in_handle = Some(real_time_streaming::start_writer_thread(
        catch_in,
        sample_rate as u32,
        Arc::clone(&to_disk),
        "dulle_dalle.wav".to_string(),
    ));

    // temporary
    let mut input = String::new();

    println!("press return to start recording");
    io::stdin().read_line(&mut input).unwrap();

    to_disk.store(true, Ordering::SeqCst);

    println!("press return to stop recording");
    io::stdin().read_line(&mut input).unwrap();

    if let Some(catch_handle) = catch_in_handle {
        real_time_streaming::stop_writer_thread(catch_handle);
    }

    Ok(())
}
