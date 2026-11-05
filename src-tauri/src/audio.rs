//! The microphone: opened when a dictation starts and closed when it ends (so the system shows the microphone in
//! use only while dictating). Collects mono samples and knows how long it has been quiet.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample, Stream};

/// Silence is judged per block of this length.
const BLOCKS_PER_SECOND: u32 = 10;

struct Captured {
    samples: Vec<f32>,
    block_energy: f32,
    block_length: u32,
    last_loud: Instant,
}

pub struct Recording {
    // Dropping the stream closes the microphone.
    _stream: Stream,
    captured: Arc<Mutex<Captured>>,
    sample_rate: u32,
    started: Instant,
}

/// Open `microphone` (empty = the system's default) and start recording. `silence_level`: RMS below which a
/// block counts as quiet.
pub fn start(microphone: &str, silence_level: f32) -> Result<Recording, String> {
    let host = cpal::default_host();
    let device = if microphone.is_empty() {
        host.default_input_device().ok_or("no microphone")?
    } else {
        host.input_devices()
            .map_err(|error| error.to_string())?
            .find(|device| device.name().is_ok_and(|name| name == microphone))
            .ok_or_else(|| format!("microphone {microphone:?} not found"))?
    };
    let supported = device.default_input_config().map_err(|error| error.to_string())?;
    let sample_rate = supported.sample_rate().0;
    let config = supported.config();
    let now = Instant::now();
    let captured =
        Arc::new(Mutex::new(Captured { samples: Vec::new(), block_energy: 0.0, block_length: 0, last_loud: now }));
    let block = sample_rate / BLOCKS_PER_SECOND;
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => open::<f32>(&device, &config, &captured, block, silence_level),
        cpal::SampleFormat::I16 => open::<i16>(&device, &config, &captured, block, silence_level),
        cpal::SampleFormat::U16 => open::<u16>(&device, &config, &captured, block, silence_level),
        cpal::SampleFormat::I32 => open::<i32>(&device, &config, &captured, block, silence_level),
        other => return Err(format!("microphone sample format {other} is not supported")),
    }?;
    stream.play().map_err(|error| error.to_string())?;
    Ok(Recording { _stream: stream, captured, sample_rate, started: now })
}

fn open<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    captured: &Arc<Mutex<Captured>>,
    block: u32,
    silence_level: f32,
) -> Result<Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let captured = Arc::clone(captured);
    let threshold = silence_level * silence_level;
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mut captured = captured.lock().unwrap_or_else(PoisonError::into_inner);
                for frame in data.chunks(channels) {
                    #[allow(clippy::cast_precision_loss)] // a handful of channels
                    let mono = frame.iter().map(|&sample| sample.to_sample::<f32>()).sum::<f32>() / channels as f32;
                    captured.samples.push(mono);
                    captured.block_energy += mono * mono;
                    captured.block_length += 1;
                    if captured.block_length == block {
                        #[allow(clippy::cast_precision_loss)]
                        let mean_square = captured.block_energy / block as f32;
                        if mean_square >= threshold {
                            captured.last_loud = Instant::now();
                        }
                        captured.block_energy = 0.0;
                        captured.block_length = 0;
                    }
                }
            },
            // A broken stream ends with what it has; the error shows in the log.
            |error| eprintln!("microphone: {error}"),
            None,
        )
        .map_err(|error| error.to_string())
}

impl Recording {
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn quiet_for(&self) -> Duration {
        self.captured.lock().unwrap_or_else(PoisonError::into_inner).last_loud.elapsed()
    }

    /// Close the microphone; the samples (mono, -1 to 1) and their rate.
    pub fn finish(self) -> (Vec<f32>, u32) {
        let sample_rate = self.sample_rate;
        let captured = Arc::clone(&self.captured);
        drop(self);
        let samples = std::mem::take(&mut captured.lock().unwrap_or_else(PoisonError::into_inner).samples);
        (samples, sample_rate)
    }
}
