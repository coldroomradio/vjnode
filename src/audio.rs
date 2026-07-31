use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex32, Fft, FftPlanner};

const FFT_SIZE: usize = 1024;

/// Snapshot of audio energy in a few frequency bands, updated live.
/// This is what you'd read from `update()` each frame and feed into
/// shader uniforms, node parameters, particle forces, etc.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioBands {
    pub bass: f32,   // ~20-250 Hz
    pub mid: f32,    // ~250-4000 Hz
    pub treble: f32, // ~4000+ Hz
    pub rms: f32,    // overall loudness
}

/// Owns the live input stream. Keep this alive for as long as you want
/// audio capture running — dropping it stops the stream.
pub struct AudioAnalyzer {
    _stream: cpal::Stream,
    pub bands: Arc<Mutex<AudioBands>>,
}

impl AudioAnalyzer {
    /// Starts capturing from the system default input device (mic, or
    /// whatever's set as default input — e.g. BlackHole/Loopback if you
    /// route a DAW's output into an input device, which is the usual
    /// trick for reacting to "the song" rather than raw mic pickup).
    pub fn start() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("no input audio device found — check System Settings > Sound > Input");
        let supported_config = device
            .default_input_config()
            .expect("no default input config for this device");

        let sample_rate = supported_config.sample_rate().0 as f32;
        let channels = supported_config.channels() as usize;
        let sample_format = supported_config.sample_format();
        let stream_config: cpal::StreamConfig = supported_config.into();

        let bands = Arc::new(Mutex::new(AudioBands::default()));
        let bands_for_stream = bands.clone();

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        let mut sample_buffer: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);

        let err_fn = |err| eprintln!("audio stream error: {err}");

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        downmix_and_analyze(
                            data,
                            channels,
                            &mut sample_buffer,
                            &fft,
                            sample_rate,
                            &bands_for_stream,
                        );
                    },
                    err_fn,
                    None,
                )
                .expect("failed to build input stream"),
            other => {
                panic!(
                    "unsupported input sample format: {other:?} — this scaffold only \
                     handles F32 input right now; most Macs default to F32 so this \
                     should be rare. Ping me with the format and I'll add a branch."
                )
            }
        };

        stream.play().expect("failed to start audio input stream");

        Self {
            _stream: stream,
            bands,
        }
    }
}

fn downmix_and_analyze(
    data: &[f32],
    channels: usize,
    sample_buffer: &mut Vec<f32>,
    fft: &Arc<dyn Fft<f32>>,
    sample_rate: f32,
    bands_out: &Arc<Mutex<AudioBands>>,
) {
    for frame in data.chunks(channels) {
        let mono: f32 = frame.iter().sum::<f32>() / channels as f32;
        sample_buffer.push(mono);
    }

    while sample_buffer.len() >= FFT_SIZE {
        let chunk: Vec<f32> = sample_buffer.drain(..FFT_SIZE).collect();
        let result = analyze_chunk(&chunk, fft, sample_rate);
        if let Ok(mut b) = bands_out.lock() {
            *b = result;
        }
    }
}

fn analyze_chunk(chunk: &[f32], fft: &Arc<dyn Fft<f32>>, sample_rate: f32) -> AudioBands {
    // Hann window reduces spectral leakage (smearing) in the FFT result.
    let mut buffer: Vec<Complex32> = chunk
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let w = 0.5
                - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0)).cos();
            Complex32::new(s * w, 0.0)
        })
        .collect();

    fft.process(&mut buffer);

    let bin_hz = sample_rate / FFT_SIZE as f32;
    let mut bass = 0.0f32;
    let mut mid = 0.0f32;
    let mut treble = 0.0f32;

    // Real input -> symmetric spectrum, only need the first half.
    for (i, c) in buffer.iter().take(FFT_SIZE / 2).enumerate() {
        let freq = i as f32 * bin_hz;
        let mag = (c.re * c.re + c.im * c.im).sqrt();

        if freq < 250.0 {
            bass += mag;
        } else if freq < 4000.0 {
            mid += mag;
        } else {
            treble += mag;
        }
    }

    let mut rms = 0.0f32;
    for s in chunk {
        rms += s * s;
    }
    rms = (rms / chunk.len() as f32).sqrt();

    // Rough normalization so typical mic/line levels land near 0..1.
    // This will need tuning by ear once you're actually running it —
    // treat these divisors as the first knobs you turn.
    let norm = FFT_SIZE as f32 * 0.5;
    AudioBands {
        bass: (bass / norm).min(1.0),
        mid: (mid / norm).min(1.0),
        treble: (treble / norm).min(1.0),
        rms: (rms * 4.0).min(1.0),
    }
}
