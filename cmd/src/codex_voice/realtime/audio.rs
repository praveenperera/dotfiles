use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use color_eyre::eyre::{eyre, Result, WrapErr};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};

pub struct Microphone {
    _stream: Stream,
    buffer: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    channels: usize,
}

impl Microphone {
    pub fn start() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| eyre!("No default input audio device found"))?;
        let config = device
            .default_input_config()
            .wrap_err("Failed to read default input audio config")?;
        let sample_rate = config.sample_rate();
        let channels = usize::from(config.channels());
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let stream = build_input_stream(&device, &config, Arc::clone(&buffer))?;
        stream.play().wrap_err("Failed to start microphone")?;
        Ok(Self {
            _stream: stream,
            buffer,
            sample_rate,
            channels,
        })
    }

    pub fn take_pcm16(&self) -> Vec<i16> {
        let Ok(mut buffer) = self.buffer.lock() else {
            return Vec::new();
        };
        let samples = std::mem::take(&mut *buffer);
        resample_mono(
            &downmix_to_mono(&samples, self.channels),
            self.sample_rate,
            24_000,
        )
    }
}

pub struct Speaker {
    _stream: Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    active: Arc<AtomicBool>,
    sample_rate: u32,
    channels: usize,
}

impl Speaker {
    pub fn start() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| eyre!("No default output audio device found"))?;
        let config = device
            .default_output_config()
            .wrap_err("Failed to read default output audio config")?;
        let sample_rate = config.sample_rate();
        let channels = usize::from(config.channels());
        let samples = Arc::new(Mutex::new(Vec::new()));
        let active = Arc::new(AtomicBool::new(true));
        let stream =
            build_output_stream(&device, &config, Arc::clone(&samples), Arc::clone(&active))?;
        stream.play().wrap_err("Failed to start speaker")?;
        Ok(Self {
            _stream: stream,
            samples,
            active,
            sample_rate,
            channels,
        })
    }

    pub fn push_pcm16(&self, bytes: &[u8]) {
        let decoded = bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let resampled = resample_mono(&decoded, 24_000, self.sample_rate);
        let mut interleaved = interleave_mono(&resampled, self.channels);
        if let Ok(mut samples) = self.samples.lock() {
            samples.append(&mut interleaved);
        }
    }
}

impl Drop for Speaker {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
    }
}

fn build_input_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    buffer: Arc<Mutex<Vec<i16>>>,
) -> Result<Stream> {
    let stream_config = config.config();
    let err_fn = |err| tracing::error!("Input audio stream error: {err}");
    match config.sample_format() {
        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| append_i16(data.iter().copied(), &buffer),
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| append_i16(data.iter().map(u16_to_i16), &buffer),
            err_fn,
            None,
        ),
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                append_i16(data.iter().map(|sample| f32_to_i16(*sample)), &buffer)
            },
            err_fn,
            None,
        ),
        other => return Err(eyre!("Unsupported input audio sample format: {other:?}")),
    }
    .wrap_err("Failed to build microphone stream")
}

fn build_output_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    active: Arc<AtomicBool>,
) -> Result<Stream> {
    let stream_config = config.config();
    let err_fn = |err| tracing::error!("Output audio stream error: {err}");
    match config.sample_format() {
        SampleFormat::I16 => device.build_output_stream(
            &stream_config,
            move |data: &mut [i16], _| fill_output(data, &samples, &active, |sample| sample),
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            &stream_config,
            move |data: &mut [u16], _| fill_output(data, &samples, &active, i16_to_u16),
            err_fn,
            None,
        ),
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| fill_output(data, &samples, &active, i16_to_f32),
            err_fn,
            None,
        ),
        other => return Err(eyre!("Unsupported output audio sample format: {other:?}")),
    }
    .wrap_err("Failed to build speaker stream")
}

fn append_i16(samples: impl Iterator<Item = i16>, buffer: &Arc<Mutex<Vec<i16>>>) {
    if let Ok(mut buffer) = buffer.lock() {
        buffer.extend(samples);
    }
}

fn downmix_to_mono(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks(channels)
        .map(|frame| {
            let sum = frame.iter().map(|sample| i32::from(*sample)).sum::<i32>();
            (sum / frame.len() as i32) as i16
        })
        .collect()
}

fn interleave_mono(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .iter()
        .flat_map(|sample| std::iter::repeat_n(*sample, channels))
        .collect()
}

fn resample_mono(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if samples.is_empty() || from_rate == to_rate {
        return samples.to_vec();
    }

    let output_len = samples.len() * to_rate as usize / from_rate as usize;
    (0..output_len)
        .map(|index| {
            let source_index = index * from_rate as usize / to_rate as usize;
            samples
                .get(source_index)
                .copied()
                .unwrap_or_else(|| *samples.last().unwrap_or(&0))
        })
        .collect()
}

fn fill_output<T>(
    output: &mut [T],
    samples: &Arc<Mutex<Vec<i16>>>,
    active: &Arc<AtomicBool>,
    convert: impl Fn(i16) -> T,
) where
    T: Copy + Default,
{
    if !active.load(Ordering::Relaxed) {
        output.fill(T::default());
        return;
    }

    let mut source = samples.lock().ok();
    for item in output {
        let sample = source
            .as_mut()
            .and_then(|samples| (!samples.is_empty()).then(|| samples.remove(0)))
            .unwrap_or_default();
        *item = convert(sample);
    }
}

fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

fn u16_to_i16(sample: &u16) -> i16 {
    (*sample as i32 - 32768) as i16
}

fn i16_to_u16(sample: i16) -> u16 {
    (sample as i32 + 32768) as u16
}

fn i16_to_f32(sample: i16) -> f32 {
    sample as f32 / i16::MAX as f32
}
