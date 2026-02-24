//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).
//!
//! DSP engine with CPAL audio output.
//!
//! Provides real-time audio synthesis with:
//! - Multiple waveform types (Sine, Saw, Triangle, Pulse)
//! - Dual oscillators with LFO modulation
//! - Lock-free parameter sharing via atomics
//! - Scope data via triple buffer
//! - CPAL output stream for real audio playback

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, Stream, StreamConfig};
use triple_buffer::triple_buffer;

use xilem_synth_widgets::ScopeSource;

const SCOPE_SAMPLES: usize = 4096;

/// Waveform types for the oscillators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Waveform {
    Sine = 0,
    Saw = 1,
    Triangle = 2,
    Pulse = 3,
}

impl Waveform {
    pub fn from_index(idx: u32) -> Self {
        match idx {
            0 => Waveform::Sine,
            1 => Waveform::Saw,
            2 => Waveform::Triangle,
            3 => Waveform::Pulse,
            _ => Waveform::Sine,
        }
    }

    /// Generate a sample for the given phase (0.0..1.0).
    pub fn sample(self, phase: f64) -> f64 {
        match self {
            Waveform::Sine => (phase * std::f64::consts::TAU).sin(),
            Waveform::Saw => 2.0 * phase - 1.0,
            Waveform::Triangle => 4.0 * (phase - (phase + 0.5).floor()).abs() - 1.0,
            Waveform::Pulse => {
                if phase < 0.55 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

/// Atomic f32 using AtomicU32 bit-casting (standard pattern for lock-free DSP).
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn store(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

/// Parameters shared between UI and DSP threads via atomics.
pub struct SharedParams {
    pub freq1: AtomicF32,
    pub freq2: AtomicF32,
    pub volume_db: AtomicF32,
    pub lfo_enabled: AtomicU32,
    pub mute: AtomicU32,
    pub waveform: AtomicU32,
    /// Max LFO rate the drift reaches (Hz). Default 8.0.
    pub lfo_range: AtomicF32,
    /// How fast the LFO rate drifts (per sample). Default 0.0001.
    pub lfo_speed: AtomicF32,
}

impl SharedParams {
    pub fn new(freq1: f32, freq2: f32, volume_db: f32, lfo_enabled: bool) -> Self {
        Self {
            freq1: AtomicF32::new(freq1),
            freq2: AtomicF32::new(freq2),
            volume_db: AtomicF32::new(volume_db),
            lfo_enabled: AtomicU32::new(lfo_enabled as u32),
            mute: AtomicU32::new(0),
            waveform: AtomicU32::new(0),
            lfo_range: AtomicF32::new(8.0),
            lfo_speed: AtomicF32::new(0.0001),
        }
    }

    pub fn set_lfo_enabled(&self, v: bool) {
        self.lfo_enabled.store(v as u32, Ordering::Relaxed);
    }

    pub fn lfo_enabled(&self) -> bool {
        self.lfo_enabled.load(Ordering::Relaxed) != 0
    }

    pub fn set_mute(&self, v: bool) {
        self.mute.store(v as u32, Ordering::Relaxed);
    }

    pub fn muted(&self) -> bool {
        self.mute.load(Ordering::Relaxed) != 0
    }

    pub fn set_waveform(&self, idx: u32) {
        self.waveform.store(idx, Ordering::Relaxed);
    }

    pub fn waveform(&self) -> Waveform {
        Waveform::from_index(self.waveform.load(Ordering::Relaxed))
    }
}

/// Enumerate available audio output device names.
pub fn list_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                names.push(name);
            }
        }
    }
    names
}

/// Handle returned from `DspEngine::start()` for the UI side.
///
/// Read scope data via `read_scope()`. Parameters are updated
/// directly on the `SharedParams` via atomics.
pub struct DspHandle {
    pub params: Arc<SharedParams>,
    scope_source: ScopeSource,
    stream: Option<Stream>,
}

impl DspHandle {
    /// Create a handle without an active audio stream (for initial state).
    pub fn new_idle(params: Arc<SharedParams>) -> Self {
        let (_scope_input, scope_output) = triple_buffer(&Vec::<f32>::new());
        Self {
            params,
            scope_source: ScopeSource::new(scope_output),
            stream: None,
        }
    }

    /// Get the scope source for passing to the Scope widget.
    pub fn scope_source(&self) -> ScopeSource {
        self.scope_source.clone()
    }

    /// Whether the audio stream is currently active.
    pub fn is_running(&self) -> bool {
        self.stream.is_some()
    }

    /// Stop the audio stream.
    pub fn stop(&mut self) {
        self.stream = None;
    }
}

/// DSP engine that creates CPAL output streams.
pub struct DspEngine;

impl DspEngine {
    /// Start the DSP with a CPAL audio stream on the given device.
    /// If `device_name` is None, uses the default output device.
    pub fn start(
        device_name: Option<&str>,
        params: Arc<SharedParams>,
    ) -> Result<DspHandle, String> {
        let host = cpal::default_host();

        let device = if let Some(name) = device_name {
            host.output_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().unwrap_or_default() == name)
                .ok_or_else(|| format!("Device not found: {name}"))?
        } else {
            host.default_output_device()
                .ok_or_else(|| "No default output device".to_string())?
        };

        let supported_config = device
            .default_output_config()
            .map_err(|e| e.to_string())?;
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();
        let sample_rate = config.sample_rate.0 as f64;
        let channels = config.channels as usize;

        let (scope_input, scope_output) = triple_buffer(&Vec::<f32>::new());

        let stream = match sample_format {
            SampleFormat::I8 => Self::make_stream::<i8>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            SampleFormat::I16 => Self::make_stream::<i16>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            SampleFormat::I32 => Self::make_stream::<i32>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            SampleFormat::U8 => Self::make_stream::<u8>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            SampleFormat::U16 => Self::make_stream::<u16>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            SampleFormat::U32 => Self::make_stream::<u32>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            SampleFormat::F32 => Self::make_stream::<f32>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            SampleFormat::F64 => Self::make_stream::<f64>(&device, &config, sample_rate, channels, Arc::clone(&params), scope_input),
            f => return Err(format!("Unsupported sample format: {f:?}")),
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(DspHandle {
            params,
            scope_source: ScopeSource::new(scope_output),
            stream: Some(stream),
        })
    }

    fn make_stream<S: FromSample<f32> + cpal::SizedSample>(
        device: &cpal::Device,
        config: &StreamConfig,
        sample_rate: f64,
        channels: usize,
        params: Arc<SharedParams>,
        mut scope_input: triple_buffer::Input<Vec<f32>>,
    ) -> Result<Stream, cpal::BuildStreamError> {
        let mut phase1: f64 = 0.0;
        let mut phase2: f64 = 0.0;
        let mut lfo_phase: f64 = 0.0;
        let mut lfo_rate: f64 = 2.0;
        let mut lfo_direction: f64 = 1.0;
        let mut scope_accum: Vec<f32> = Vec::with_capacity(SCOPE_SAMPLES);

        device.build_output_stream(
            config,
            move |data: &mut [S], _: &cpal::OutputCallbackInfo| {
                let freq1 = params.freq1.load() as f64;
                let freq2 = params.freq2.load() as f64;
                let volume_db = params.volume_db.load() as f64;
                let lfo_on = params.lfo_enabled();
                let muted = params.muted();
                let waveform = params.waveform();
                let lfo_max = params.lfo_range.load() as f64;
                let lfo_speed = params.lfo_speed.load() as f64;

                let volume_linear = if muted {
                    0.0
                } else {
                    10.0_f64.powf(volume_db / 20.0)
                };

                let frames = data.len() / channels;
                for frame in 0..frames {
                    let osc1 = waveform.sample(phase1);
                    let osc2 = waveform.sample(phase2);
                    let mixed = (osc1 + osc2) * 0.5;

                    let lfo_mod = if lfo_on {
                        0.5 + 0.5 * (lfo_phase * std::f64::consts::TAU).sin()
                    } else {
                        1.0
                    };

                    let sample = (mixed * lfo_mod * volume_linear) as f32;
                    let clamped = sample.clamp(-1.0, 1.0);

                    // Write to all channels
                    for ch in 0..channels {
                        data[frame * channels + ch] = S::from_sample(clamped);
                    }

                    scope_accum.push(clamped);

                    phase1 += freq1 / sample_rate;
                    if phase1 >= 1.0 {
                        phase1 -= 1.0;
                    }
                    phase2 += freq2 / sample_rate;
                    if phase2 >= 1.0 {
                        phase2 -= 1.0;
                    }
                    lfo_phase += lfo_rate / sample_rate;
                    if lfo_phase >= 1.0 {
                        lfo_phase -= 1.0;
                    }

                    // Drift the LFO rate up and down between 1.0 and lfo_max
                    lfo_rate += lfo_direction * lfo_speed;
                    if lfo_rate > lfo_max {
                        lfo_direction = -1.0;
                    } else if lfo_rate < 1.0 {
                        lfo_direction = 1.0;
                    }
                }

                // Publish scope data when we have enough
                if scope_accum.len() >= SCOPE_SAMPLES {
                    let buf = scope_input.input_buffer_mut();
                    buf.clear();
                    buf.extend_from_slice(&scope_accum[..SCOPE_SAMPLES]);
                    scope_input.publish();
                    scope_accum.clear();
                }
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )
    }
}
