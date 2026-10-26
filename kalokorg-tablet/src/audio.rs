//! Lock-free real-time audio pipeline for KaloKorg M20s Tablet (`kalokorg-tablet`).
//!
//! Uses a bounded `crossbeam_channel` (`capacity = 256`) to dispatch `ControlEvent` messages
//! from the `egui` UI thread to the real-time audio callback without locks or heap allocations.
//! - On Android Tablet (`target_os = "android"`): uses Google `oboe` in `LowLatency` + `Exclusive` mode.
//! - On iPadOS & Desktop (`ios`, `macos`, `windows`, `linux`): uses `cpal` (CoreAudio AudioUnit on iOS/macOS)
//!   with a fixed buffer size of `<= 256` frames (`5.33 ms` at 48 kHz, sub-10ms latency).

use crossbeam_channel::{bounded, Receiver, Sender};
use korg_ms20::{
    engine::{KaloKorgEngine, Preset},
    patchbay::{PatchDest, PatchSource},
    AudioProcessor,
};

/// Maximum audio callback block size in frames (`256` samples = `5.33 ms` at `48 kHz`).
pub const TARGET_BUFFER_FRAMES: u32 = 256;

/// Default lock-free control channel capacity.
pub const CONTROL_CHANNEL_CAPACITY: usize = 256;

/// Extended parameter IDs for tablet controls beyond the core `0..146` IDs.
pub mod param_ids {
    pub const VCO1_WAVEFORM: u32 = 0;
    pub const VCO1_PW: u32 = 1;
    pub const VCO1_SCALE: u32 = 2;
    pub const PORTAMENTO_TIME: u32 = 3;
    pub const VCO2_WAVEFORM: u32 = 4;
    pub const VCO2_PITCH: u32 = 5;
    pub const VCO2_SCALE: u32 = 6;
    pub const MASTER_TUNE: u32 = 7;
    pub const VCO1_LEVEL: u32 = 8;
    pub const VCO2_LEVEL: u32 = 9;
    pub const FM_MG_TEXT: u32 = 10;
    pub const FM_EG1_EXT: u32 = 11;
    pub const HPF_CUTOFF: u32 = 12;
    pub const HPF_PEAK: u32 = 13;
    pub const HPF_MG_TEXT: u32 = 14;
    pub const HPF_EG2_EXT: u32 = 15;
    pub const LPF_CUTOFF: u32 = 16;
    pub const LPF_PEAK: u32 = 17;
    pub const LPF_MG_TEXT: u32 = 18;
    pub const LPF_EG2_EXT: u32 = 19;
    pub const MG_WAVEFORM: u32 = 20;
    pub const MG_FREQUENCY: u32 = 21;
    pub const EG1_DELAY: u32 = 22;
    pub const EG1_ATTACK: u32 = 23;
    pub const EG1_RELEASE: u32 = 24;
    pub const EG2_HOLD: u32 = 25;
    pub const EG2_ATTACK: u32 = 26;
    pub const EG2_DECAY: u32 = 27;
    pub const EG2_SUSTAIN: u32 = 28;
    pub const EG2_RELEASE: u32 = 29;
    pub const MASTER_VOLUME: u32 = 30;
    pub const FILTER_REVISION: u32 = 36;
    pub const DRONE_GATE: u32 = 37;
    pub const OUTPUT_PAD_DB: u32 = 38;
    pub const CONTROL_WHEEL: u32 = 39;
    pub const MOMENTARY_SWITCH: u32 = 40;

    // Effects section: Zoom MS-70CDR (100..105), BOSS DD-8 Delay (110..115),
    // Red Panda Particle 2 (120..127), Strymon Cloudburst (130..137), BOSS CE-2W Chorus (140..146)
    pub const ZOOM_ENABLED: u32 = 100;
    pub const ZOOM_MODE: u32 = 101;
    pub const ZOOM_DEPTH: u32 = 102;
    pub const ZOOM_RATE: u32 = 103;
    pub const ZOOM_TONE: u32 = 104;
    pub const ZOOM_MIX: u32 = 105;

    pub const DD8_ENABLED: u32 = 110;
    pub const DD8_MODE: u32 = 111;
    pub const DD8_LEVEL: u32 = 112;
    pub const DD8_FEEDBACK: u32 = 113;
    pub const DD8_TIME: u32 = 114;
    pub const DD8_MOD_DEPTH: u32 = 115;

    pub const PARTICLE_ENABLED: u32 = 120;
    pub const PARTICLE_MODE: u32 = 121;
    pub const PARTICLE_BLEND: u32 = 122;
    pub const PARTICLE_CHOP: u32 = 123;
    pub const PARTICLE_PITCH: u32 = 124;
    pub const PARTICLE_GRAIN_SIZE: u32 = 125;
    pub const PARTICLE_FEEDBACK: u32 = 126;
    pub const PARTICLE_FREEZE: u32 = 127;

    pub const CLOUDBURST_ENABLED: u32 = 130;
    pub const CLOUDBURST_ENSEMBLE_MODE: u32 = 131;
    pub const CLOUDBURST_DECAY: u32 = 132;
    pub const CLOUDBURST_ENSEMBLE: u32 = 133;
    pub const CLOUDBURST_MIX: u32 = 134;
    pub const CLOUDBURST_TONE: u32 = 135;

    pub const CHORUS_ENABLED: u32 = 140;
    pub const CHORUS_MODE: u32 = 141;
    pub const CHORUS_RATE: u32 = 142;
    pub const CHORUS_DEPTH: u32 = 143;
    pub const CHORUS_STEREO_WIDTH: u32 = 144;
    pub const CHORUS_WARMTH: u32 = 145;
    pub const CHORUS_MIX: u32 = 146;

    // Extended tablet control IDs (handled without heap allocation on the audio thread)
    pub const SATURATOR_DRIVE: u32 = 200;
    pub const LOAD_FACTORY_PRESET: u32 = 201;
    pub const PATCH_CONNECT_ENCODED: u32 = 202;
    pub const PATCH_DISCONNECT_DEST: u32 = 203;
    pub const PATCH_CLEAR_ALL: u32 = 204;
    pub const PITCH_BEND_SEMITONES: u32 = 205;
}

/// Lock-free message sent from the `egui` UI thread to the real-time audio callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlEvent {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    ParameterChange { id: u32, value: f32 },
}

/// Pre-allocated real-time DSP processor wrapper used inside the audio callback.
///
/// Pre-loads the factory presets at initialization time so preset switching and all
/// parameter/MIDI updates run with strictly zero heap allocations (`no-alloc`) on the audio thread.
pub struct RealtimeTabletProcessor {
    pub engine: KaloKorgEngine,
    pub saturator_drive: f32,
    presets: Vec<Preset>,
}

impl RealtimeTabletProcessor {
    pub fn new(sample_rate: f32) -> Self {
        let mut engine = KaloKorgEngine::new(sample_rate);
        engine.params.drone_gate = false;
        Self {
            engine,
            saturator_drive: 2.5,
            presets: Preset::factory_presets(),
        }
    }

    /// Drain all pending `ControlEvent` messages from the lock-free receiver without blocking.
    #[inline]
    pub fn poll_events(&mut self, rx: &Receiver<ControlEvent>) {
        while let Ok(event) = rx.try_recv() {
            self.apply_event(event);
        }
    }

    /// Apply a single `ControlEvent` with zero heap allocations.
    #[inline]
    pub fn apply_event(&mut self, event: ControlEvent) {
        match event {
            ControlEvent::NoteOn { note, velocity } => {
                let vel_u8 = (velocity.clamp(0.0, 1.0) * 127.0).round() as u8;
                self.engine
                    .handle_midi_event([0x90, note & 0x7F, vel_u8.max(1)]);
            }
            ControlEvent::NoteOff { note } => {
                self.engine.handle_midi_event([0x80, note & 0x7F, 0]);
            }
            ControlEvent::ParameterChange { id, value } => match id {
                param_ids::SATURATOR_DRIVE => {
                    self.saturator_drive = value.clamp(0.0, 10.0);
                    // Also warm up the BBD chorus saturator stage proportionally
                    self.engine.pedals_params.chorus.warmth =
                        (4.0 + self.saturator_drive * 0.6).clamp(0.0, 10.0);
                }
                param_ids::LOAD_FACTORY_PRESET => {
                    let idx = (value.max(0.0) as usize) % self.presets.len().max(1);
                    if let Some(p) = self.presets.get(idx) {
                        let pad = self.engine.params.output_pad_db;
                        let drone = self.engine.params.drone_gate;
                        self.engine.params = p.synth.clone();
                        self.engine.params.output_pad_db = pad;
                        self.engine.params.drone_gate = drone;
                        self.engine.pedals_params = p.pedals.clone();
                        self.engine.patchbay.clear_all();
                        for cable in &p.cables {
                            self.engine
                                .connect_patch_cable(cable.source, cable.dest);
                        }
                    }
                }
                param_ids::PATCH_CONNECT_ENCODED => {
                    let encoded = value.max(0.0) as usize;
                    let src_idx = encoded / 100;
                    let dst_idx = encoded % 100;
                    self.engine.connect_patch_cable(
                        PatchSource::from_index(src_idx),
                        PatchDest::from_index(dst_idx),
                    );
                }
                param_ids::PATCH_DISCONNECT_DEST => {
                    let dst_idx = value.max(0.0) as usize;
                    self.engine
                        .disconnect_patch_cable(PatchDest::from_index(dst_idx));
                }
                param_ids::PATCH_CLEAR_ALL => {
                    self.engine.clear_patch_cables();
                }
                param_ids::PITCH_BEND_SEMITONES => {
                    self.engine.voice.pitch_bend_semitones = value.clamp(-12.0, 12.0);
                }
                _ => {
                    self.engine.set_parameter(id, value);
                }
            },
        }
    }

    /// Render a single stereo frame with analog saturator drive applied before soft-limiting.
    #[inline]
    pub fn render_stereo_frame(&mut self) -> (f32, f32) {
        let (l, r) = self.engine.render_frame();
        if self.saturator_drive <= 0.05 {
            (l, r)
        } else {
            let drive = 1.0 + (self.saturator_drive / 10.0) * 2.4;
            let norm = 1.0 / drive.tanh().max(0.5);
            let sat_l = (l * drive).tanh() * norm * 0.85;
            let sat_r = (r * drive).tanh() * norm * 0.85;
            (sat_l.clamp(-0.98, 0.98), sat_r.clamp(-0.98, 0.98))
        }
    }

    /// Process an interleaved audio buffer (`channels >= 1`) in-place with zero allocations.
    #[inline]
    pub fn process_interleaved(
        &mut self,
        rx: &Receiver<ControlEvent>,
        output: &mut [f32],
        channels: usize,
    ) {
        self.poll_events(rx);
        let ch = channels.max(1);
        for frame in output.chunks_exact_mut(ch) {
            let (l, r) = self.render_stereo_frame();
            if ch >= 2 {
                frame[0] = l;
                frame[1] = r;
                for extra in &mut frame[2..] {
                    *extra = (l + r) * 0.5;
                }
            } else {
                frame[0] = (l + r) * 0.5;
            }
        }
    }
}

// ============================================================================
// PLATFORM BACKEND 1: ANDROID TABLET (Google Oboe Low-Latency Exclusive Stream)
// ============================================================================

#[cfg(target_os = "android")]
mod android_backend {
    use super::*;
    use oboe::{
        AudioOutputCallback, AudioOutputStreamSafeTrait, AudioStream, AudioStreamAsync,
        AudioStreamBuilder, DataCallbackResult, Output, PerformanceMode, SharingMode,
    };

    struct OboeTabletCallback {
        processor: RealtimeTabletProcessor,
        rx: Receiver<ControlEvent>,
    }

    impl AudioOutputCallback for OboeTabletCallback {
        type FrameType = (f32, f32);

        #[inline]
        fn on_audio_ready(
            &mut self,
            _stream: &mut dyn AudioOutputStreamSafeTrait,
            frames: &mut [(f32, f32)],
        ) -> DataCallbackResult {
            self.processor.poll_events(&self.rx);
            for frame in frames.iter_mut() {
                *frame = self.processor.render_stereo_frame();
            }
            DataCallbackResult::Continue
        }
    }

    pub struct PlatformAudioStream {
        stream: AudioStreamAsync<Output, OboeTabletCallback>,
        pub sample_rate: u32,
        pub buffer_frames: u32,
    }

    impl PlatformAudioStream {
        pub fn start(rx: Receiver<ControlEvent>) -> Result<Self, String> {
            let sample_rate = 48_000;
            let buffer_frames = TARGET_BUFFER_FRAMES;
            let callback = OboeTabletCallback {
                processor: RealtimeTabletProcessor::new(sample_rate as f32),
                rx,
            };

            let mut stream = AudioStreamBuilder::default()
                .set_performance_mode(PerformanceMode::LowLatency)
                .set_sharing_mode(SharingMode::Exclusive)
                .set_sample_rate(sample_rate as i32)
                .set_frames_per_callback(buffer_frames as i32)
                .set_stereo()
                .set_f32()
                .set_output()
                .set_callback(callback)
                .open_stream()?;

            stream.start()?;

            Ok(Self {
                stream,
                sample_rate,
                buffer_frames,
            })
        }
    }

    impl Drop for PlatformAudioStream {
        fn drop(&mut self) {
            let _ = self.stream.stop();
        }
    }
}

// ============================================================================
// PLATFORM BACKEND 2: iPadOS & DESKTOP PREVIEW (CPAL / CoreAudio AudioUnit)
// ============================================================================

#[cfg(any(
    target_os = "ios",
    target_os = "macos",
    target_os = "windows",
    target_os = "linux"
))]
mod cpal_backend {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{BufferSize, SampleRate, Stream, StreamConfig};

    pub struct PlatformAudioStream {
        _stream: Stream,
        pub sample_rate: u32,
        pub buffer_frames: u32,
    }

    impl PlatformAudioStream {
        pub fn start(rx: Receiver<ControlEvent>) -> Result<Self, String> {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or_else(|| "No default output audio device found".to_string())?;

            let default_cfg = device
                .default_output_config()
                .map_err(|e| format!("Failed to query default output config: {}", e))?;

            let sample_rate = default_cfg.sample_rate().0.max(44_100);
            let channels = default_cfg.channels().max(1);
            let buffer_frames = TARGET_BUFFER_FRAMES;

            let config = StreamConfig {
                channels,
                sample_rate: SampleRate(sample_rate),
                buffer_size: BufferSize::Fixed(buffer_frames),
            };

            let mut processor = RealtimeTabletProcessor::new(sample_rate as f32);
            let ch_usize = channels as usize;

            let stream = device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        processor.process_interleaved(&rx, data, ch_usize);
                    },
                    move |err| {
                        eprintln!("[kalokorg-tablet] Audio stream error: {}", err);
                    },
                    None,
                )
                .map_err(|e| format!("Failed to build output audio stream: {}", e))?;

            stream
                .play()
                .map_err(|e| format!("Failed to start output audio stream: {}", e))?;

            Ok(Self {
                _stream: stream,
                sample_rate,
                buffer_frames,
            })
        }
    }
}

/// Active low-latency audio engine handle owned by the tablet UI application.
pub struct TabletAudioEngine {
    pub tx: Sender<ControlEvent>,
    pub sample_rate: u32,
    pub buffer_frames: u32,
    pub backend_label: String,
    #[cfg(target_os = "android")]
    _stream: Option<android_backend::PlatformAudioStream>,
    #[cfg(any(
        target_os = "ios",
        target_os = "macos",
        target_os = "windows",
        target_os = "linux"
    ))]
    _stream: Option<cpal_backend::PlatformAudioStream>,
}

impl TabletAudioEngine {
    /// Initialize the bounded lock-free `ControlEvent` channel (`capacity = 256`) and start
    /// the low-latency platform audio output stream (`<= 256` frames per buffer).
    pub fn start() -> Self {
        let (tx, rx) = bounded::<ControlEvent>(CONTROL_CHANNEL_CAPACITY);

        #[cfg(target_os = "android")]
        {
            match android_backend::PlatformAudioStream::start(rx) {
                Ok(stream) => {
                    let sr = stream.sample_rate;
                    let bf = stream.buffer_frames;
                    return Self {
                        tx,
                        sample_rate: sr,
                        buffer_frames: bf,
                        backend_label: format!("Oboe LowLatency Exclusive ({} Hz, {} f)", sr, bf),
                        _stream: Some(stream),
                    };
                }
                Err(err) => {
                    eprintln!("[kalokorg-tablet] Oboe stream fallback: {}", err);
                    return Self {
                        tx,
                        sample_rate: 48_000,
                        buffer_frames: TARGET_BUFFER_FRAMES,
                        backend_label: "Oboe Offline Fallback (48000 Hz, 256 f)".to_string(),
                        _stream: None,
                    };
                }
            }
        }

        #[cfg(any(
            target_os = "ios",
            target_os = "macos",
            target_os = "windows",
            target_os = "linux"
        ))]
        {
            match cpal_backend::PlatformAudioStream::start(rx) {
                Ok(stream) => {
                    let sr = stream.sample_rate;
                    let bf = stream.buffer_frames;
                    let latency_ms = (bf as f32 * 1000.0) / (sr as f32);
                    return Self {
                        tx,
                        sample_rate: sr,
                        buffer_frames: bf,
                        backend_label: format!(
                            "CPAL/CoreAudio ({} Hz, {} frames / {:.1} ms)",
                            sr, bf, latency_ms
                        ),
                        _stream: Some(stream),
                    };
                }
                Err(err) => {
                    eprintln!("[kalokorg-tablet] CPAL stream fallback: {}", err);
                    return Self {
                        tx,
                        sample_rate: 48_000,
                        buffer_frames: TARGET_BUFFER_FRAMES,
                        backend_label: "CPAL Offline Fallback (48000 Hz, 256 frames)".to_string(),
                        _stream: None,
                    };
                }
            }
        }
    }

    /// Create a pair `(TabletAudioEngine, Receiver<ControlEvent>)` for deterministic unit testing.
    pub fn new_test_pair() -> (Self, Receiver<ControlEvent>) {
        let (tx, rx) = bounded::<ControlEvent>(CONTROL_CHANNEL_CAPACITY);
        let engine = Self {
            tx,
            sample_rate: 48_000,
            buffer_frames: TARGET_BUFFER_FRAMES,
            backend_label: "Test Lock-Free Channel (48000 Hz, 256 frames)".to_string(),
            _stream: None,
        };
        (engine, rx)
    }

    /// Non-blocking send of a `ControlEvent` from the UI thread to the audio callback.
    #[inline]
    pub fn send(&self, event: ControlEvent) {
        let _ = self.tx.try_send(event);
    }

    #[inline]
    pub fn send_param(&self, id: u32, value: f32) {
        let _ = self.tx.try_send(ControlEvent::ParameterChange { id, value });
    }

    #[inline]
    pub fn note_on(&self, note: u8, velocity: f32) {
        let _ = self.tx.try_send(ControlEvent::NoteOn { note, velocity });
    }

    #[inline]
    pub fn note_off(&self, note: u8) {
        let _ = self.tx.try_send(ControlEvent::NoteOff { note });
    }
}
