use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use color_eyre::eyre::{eyre, Result, WrapErr};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};
use ringbuf::{
    traits::{Consumer, Observer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};

const INPUT_BUFFER_SECONDS: usize = 2;
const OUTPUT_BUFFER_SECONDS: usize = 8;
const PLAYBACK_BACKPRESSURE_WAIT: Duration = Duration::from_millis(5);

pub struct Microphone {
    _stream: Stream,
    samples: RefCell<HeapCons<i16>>,
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
        let (producer, consumer) = HeapRb::new(audio_buffer_capacity(
            sample_rate,
            channels,
            INPUT_BUFFER_SECONDS,
        ))
        .split();
        let stream = build_input_stream(&device, &config, producer)?;
        stream.play().wrap_err("Failed to start microphone")?;
        Ok(Self {
            _stream: stream,
            samples: RefCell::new(consumer),
            sample_rate,
            channels,
        })
    }

    pub fn take_pcm16(&self) -> Vec<i16> {
        let Ok(mut samples) = self.samples.try_borrow_mut() else {
            return Vec::new();
        };
        let mut buffer = vec![0; samples.occupied_len()];
        let read = samples.pop_slice(&mut buffer);
        buffer.truncate(read);
        resample_mono(
            &downmix_to_mono(&buffer, self.channels),
            self.sample_rate,
            24_000,
        )
    }
}

pub struct Speaker {
    _stream: Stream,
    samples: RefCell<HeapProd<i16>>,
    active: Arc<AtomicBool>,
    stats: Arc<AudioStats>,
    resampler: RefCell<StreamingResampler>,
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
        let (producer, consumer) = HeapRb::new(audio_buffer_capacity(
            sample_rate,
            channels,
            OUTPUT_BUFFER_SECONDS,
        ))
        .split();
        let active = Arc::new(AtomicBool::new(true));
        let stats = Arc::new(AudioStats::default());
        let stream = build_output_stream(
            &device,
            &config,
            consumer,
            Arc::clone(&active),
            Arc::clone(&stats),
        )?;
        stream.play().wrap_err("Failed to start speaker")?;
        tracing::debug!(
            "Started output audio stream sample_rate={sample_rate} channels={channels}"
        );
        Ok(Self {
            _stream: stream,
            samples: RefCell::new(producer),
            active,
            stats,
            resampler: RefCell::new(StreamingResampler::new(24_000, sample_rate)),
            sample_rate,
            channels,
        })
    }

    pub async fn push_pcm16(&self, bytes: &[u8]) {
        let decoded = bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let resampled = self.resampler.borrow_mut().process(&decoded);
        let interleaved = interleave_mono(&resampled, self.channels);
        self.push_all(&interleaved).await;
    }

    pub fn has_pending_audio(&self) -> bool {
        self.samples
            .try_borrow()
            .is_ok_and(|samples| samples.occupied_len() > 0)
    }

    async fn push_all(&self, incoming: &[i16]) {
        let mut written = 0;
        let waits_before = self.stats.backpressure_waits.load(Ordering::Relaxed);
        while written < incoming.len() {
            written = self
                .samples
                .try_borrow_mut()
                .map(|mut samples| push_next_available(&mut samples, incoming, written))
                .unwrap_or(written);

            if written < incoming.len() {
                self.stats
                    .backpressure_waits
                    .fetch_add(1, Ordering::Relaxed);
                tokio::time::sleep(PLAYBACK_BACKPRESSURE_WAIT).await;
            }
        }
        let waits_after = self.stats.backpressure_waits.load(Ordering::Relaxed);
        if waits_after > waits_before {
            let queued_seconds = self.queued_seconds();
            let underruns = self.stats.underruns.load(Ordering::Relaxed);
            let waits = waits_after - waits_before;
            tracing::debug!(
                "Playback waited for speaker buffer space waits={waits} queued_seconds={queued_seconds:.2} underruns={underruns}"
            );
        }
    }

    fn queued_seconds(&self) -> f64 {
        self.samples.try_borrow().map_or(0.0, |samples| {
            samples.occupied_len() as f64 / self.channels as f64 / self.sample_rate as f64
        })
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
    mut samples: HeapProd<i16>,
) -> Result<Stream> {
    let stream_config = config.config();
    let err_fn = |err| tracing::error!("Input audio stream error: {err}");
    match config.sample_format() {
        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| {
                push_available(&mut samples, data);
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| {
                samples.push_iter(data.iter().map(u16_to_i16));
            },
            err_fn,
            None,
        ),
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                samples.push_iter(data.iter().map(|sample| f32_to_i16(*sample)));
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
    mut samples: HeapCons<i16>,
    active: Arc<AtomicBool>,
    stats: Arc<AudioStats>,
) -> Result<Stream> {
    let stream_config = config.config();
    let err_fn = |err| tracing::error!("Output audio stream error: {err}");
    match config.sample_format() {
        SampleFormat::I16 => device.build_output_stream(
            &stream_config,
            move |data: &mut [i16], _| {
                fill_output(data, &mut samples, &active, &stats, |sample| sample);
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            &stream_config,
            move |data: &mut [u16], _| {
                fill_output(data, &mut samples, &active, &stats, i16_to_u16);
            },
            err_fn,
            None,
        ),
        SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| {
                fill_output(data, &mut samples, &active, &stats, i16_to_f32);
            },
            err_fn,
            None,
        ),
        other => return Err(eyre!("Unsupported output audio sample format: {other:?}")),
    }
    .wrap_err("Failed to build speaker stream")
}

fn audio_buffer_capacity(sample_rate: u32, channels: usize, seconds: usize) -> usize {
    (sample_rate as usize * channels * seconds).max(1)
}

fn push_available(samples: &mut HeapProd<i16>, incoming: &[i16]) -> usize {
    samples.push_slice(incoming)
}

fn push_next_available(samples: &mut HeapProd<i16>, incoming: &[i16], written: usize) -> usize {
    written + push_available(samples, &incoming[written..])
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

    if samples.len() == 1 {
        return vec![samples[0]];
    }

    let output_len = samples.len() * to_rate as usize / from_rate as usize;
    if output_len == 0 {
        return Vec::new();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    (0..output_len)
        .map(|index| {
            let position = index as f64 * ratio;
            let left_index = position.floor() as usize;
            let right_index = (left_index + 1).min(samples.len() - 1);
            let fraction = position - left_index as f64;
            let left = samples[left_index] as f64;
            let right = samples[right_index] as f64;
            (left + (right - left) * fraction).round() as i16
        })
        .collect()
}

struct StreamingResampler {
    from_rate: u32,
    to_rate: u32,
    position: f64,
    samples: Vec<i16>,
}

impl StreamingResampler {
    fn new(from_rate: u32, to_rate: u32) -> Self {
        Self {
            from_rate,
            to_rate,
            position: 0.0,
            samples: Vec::new(),
        }
    }

    fn process(&mut self, incoming: &[i16]) -> Vec<i16> {
        if incoming.is_empty() {
            return Vec::new();
        }

        if self.from_rate == self.to_rate {
            return incoming.to_vec();
        }

        self.samples.extend_from_slice(incoming);
        if self.samples.len() < 2 {
            return Vec::new();
        }

        let ratio = self.from_rate as f64 / self.to_rate as f64;
        let mut output = Vec::new();
        while self.position + 1.0 < self.samples.len() as f64 {
            let left_index = self.position.floor() as usize;
            let right_index = left_index + 1;
            let fraction = self.position - left_index as f64;
            let left = self.samples[left_index] as f64;
            let right = self.samples[right_index] as f64;
            output.push((left + (right - left) * fraction).round() as i16);
            self.position += ratio;
        }

        let consumed = self.position.floor() as usize;
        if consumed > 0 {
            self.samples.drain(..consumed);
            self.position -= consumed as f64;
        }

        output
    }
}

#[derive(Default)]
struct AudioStats {
    underruns: AtomicU64,
    backpressure_waits: AtomicU64,
}

fn fill_output<T>(
    output: &mut [T],
    samples: &mut HeapCons<i16>,
    active: &Arc<AtomicBool>,
    stats: &AudioStats,
    convert: impl Fn(i16) -> T,
) where
    T: Copy + Default,
{
    if !active.load(Ordering::Relaxed) {
        output.fill(T::default());
        return;
    }

    for item in output {
        let sample = samples.try_pop().unwrap_or_else(|| {
            stats.underruns.fetch_add(1, Ordering::Relaxed);
            0
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drains_microphone_ring_buffer() {
        let (mut producer, mut consumer) = HeapRb::new(8).split();
        assert_eq!(producer.push_slice(&[10, 20, 30]), 3);

        let mut buffer = vec![0; consumer.occupied_len()];
        let read = consumer.pop_slice(&mut buffer);
        buffer.truncate(read);

        assert_eq!(buffer, [10, 20, 30]);
        assert_eq!(consumer.occupied_len(), 0);
    }

    #[test]
    fn fills_silence_on_output_underrun() {
        let active = Arc::new(AtomicBool::new(true));
        let stats = AudioStats::default();
        let (mut producer, mut consumer) = HeapRb::new(4).split();
        assert_eq!(producer.push_slice(&[1, -2]), 2);
        let mut output = [0; 4];

        fill_output(&mut output, &mut consumer, &active, &stats, |sample| sample);

        assert_eq!(output, [1, -2, 0, 0]);
        assert_eq!(stats.underruns.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn preserves_order_when_playback_push_waits_for_space() {
        let (mut producer, consumer) = HeapRb::new(3).split();
        let incoming = [1, 2, 3, 4, 5];
        let mut written = 0;

        written = push_next_available(&mut producer, &incoming, written);
        assert_eq!(written, 3);

        assert_eq!(producer.vacant_len(), 0);
        assert_eq!(consumer.occupied_len(), 3);

        let mut consumer = consumer;
        let mut output = [0; 2];
        assert_eq!(consumer.pop_slice(&mut output), 2);
        assert_eq!(output, [1, 2]);

        written = push_next_available(&mut producer, &incoming, written);
        assert_eq!(written, incoming.len());

        let mut output = [0; 3];
        assert_eq!(consumer.pop_slice(&mut output), 3);
        assert_eq!(output, [3, 4, 5]);
    }

    #[test]
    fn streaming_resampler_is_stable_across_chunk_boundaries() {
        let input = (0..256).map(|sample| sample * 64).collect::<Vec<i16>>();
        let mut single_chunk = StreamingResampler::new(24_000, 48_000);
        let expected = single_chunk.process(&input);

        let mut split_chunks = StreamingResampler::new(24_000, 48_000);
        let actual = input
            .chunks(7)
            .flat_map(|chunk| split_chunks.process(chunk))
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }
}
