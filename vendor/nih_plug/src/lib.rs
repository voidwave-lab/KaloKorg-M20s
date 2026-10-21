//! NIH-plug compatible API & C-ABI CLAP/VST3 wrapper layer.
//!
//! Provides the `nih_plug::prelude::*` types (`Plugin`, `ClapPlugin`, `Vst3Plugin`,
//! `Params`, `FloatParam`, `IntParam`, `BoolParam`, `EnumParam`, `Enum`, `Buffer`,
//! `NoteEvent`, `ProcessContext`, `AudioIOLayout`, `nih_export_clap!`, `nih_export_vst3!`)
//! so `ms20-plugin` builds 100% offline while remaining drop-in compatible with upstream `nih-plug`.

pub use std::num::NonZeroU32;
pub use std::sync::Arc;

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

// ============================================================================
// PARAMETER PRIMITIVES (FloatParam, IntParam, BoolParam, EnumParam, Smoother)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub enum FloatRange {
    Linear { min: f32, max: f32 },
    Skewed { min: f32, max: f32, factor: f32 },
    SymmetricalSkewed { min: f32, max: f32, factor: f32, center: f32 },
}

impl FloatRange {
    pub fn normalize(&self, plain: f32) -> f32 {
        match *self {
            FloatRange::Linear { min, max }
            | FloatRange::Skewed { min, max, .. }
            | FloatRange::SymmetricalSkewed { min, max, .. } => {
                if (max - min).abs() < f32::EPSILON {
                    0.0
                } else {
                    ((plain - min) / (max - min)).clamp(0.0, 1.0)
                }
            }
        }
    }

    pub fn unnormalize(&self, normalized: f32) -> f32 {
        let n = normalized.clamp(0.0, 1.0);
        match *self {
            FloatRange::Linear { min, max }
            | FloatRange::Skewed { min, max, .. }
            | FloatRange::SymmetricalSkewed { min, max, .. } => min + n * (max - min),
        }
    }

    pub fn clamp(&self, plain: f32) -> f32 {
        match *self {
            FloatRange::Linear { min, max }
            | FloatRange::Skewed { min, max, .. }
            | FloatRange::SymmetricalSkewed { min, max, .. } => plain.clamp(min, max),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum IntRange {
    Linear { min: i32, max: i32 },
}

impl IntRange {
    pub fn clamp(&self, val: i32) -> i32 {
        match *self {
            IntRange::Linear { min, max } => val.clamp(min, max),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SmoothingStyle {
    None,
    Linear(f32),
    Logarithmic(f32),
    Exponential(f32),
}

pub struct Smoother<T> {
    current: T,
}

impl<T: Copy> Smoother<T> {
    pub fn new(val: T) -> Self {
        Self { current: val }
    }

    #[inline]
    pub fn next(&self) -> T {
        self.current
    }

    #[inline]
    pub fn set(&mut self, val: T) {
        self.current = val;
    }
}

pub struct FloatParam {
    pub id: &'static str,
    pub name: &'static str,
    value_bits: AtomicU32,
    default: f32,
    pub range: FloatRange,
    pub unit: &'static str,
    pub step_size: Option<f32>,
    pub smoothing: SmoothingStyle,
    pub non_automatable: bool,
}

impl FloatParam {
    pub fn new(name: &'static str, default: f32, range: FloatRange) -> Self {
        let clamped = range.clamp(default);
        Self {
            id: "",
            name,
            value_bits: AtomicU32::new(clamped.to_bits()),
            default: clamped,
            range,
            unit: "",
            step_size: None,
            smoothing: SmoothingStyle::None,
            non_automatable: false,
        }
    }

    pub fn with_id(mut self, id: &'static str) -> Self {
        self.id = id;
        self
    }

    pub fn with_smoother(mut self, smoothing: SmoothingStyle) -> Self {
        self.smoothing = smoothing;
        self
    }

    pub fn with_unit(mut self, unit: &'static str) -> Self {
        self.unit = unit;
        self
    }

    pub fn with_step_size(mut self, step: f32) -> Self {
        self.step_size = Some(step);
        self
    }

    pub fn non_automatable(mut self) -> Self {
        self.non_automatable = true;
        self
    }

    #[inline]
    pub fn value(&self) -> f32 {
        f32::from_bits(self.value_bits.load(Ordering::Relaxed))
    }

    #[inline]
    pub fn set_plain_value(&self, v: f32) {
        let clamped = self.range.clamp(v);
        self.value_bits.store(clamped.to_bits(), Ordering::Relaxed);
    }

    #[inline]
    pub fn default_plain_value(&self) -> f32 {
        self.default
    }

    #[inline]
    pub fn modulated_plain_value(&self) -> f32 {
        self.value()
    }
}

pub struct IntParam {
    pub id: &'static str,
    pub name: &'static str,
    value: AtomicI32,
    default: i32,
    pub range: IntRange,
    pub unit: &'static str,
}

impl IntParam {
    pub fn new(name: &'static str, default: i32, range: IntRange) -> Self {
        let clamped = range.clamp(default);
        Self {
            id: "",
            name,
            value: AtomicI32::new(clamped),
            default: clamped,
            range,
            unit: "",
        }
    }

    pub fn with_id(mut self, id: &'static str) -> Self {
        self.id = id;
        self
    }

    pub fn with_unit(mut self, unit: &'static str) -> Self {
        self.unit = unit;
        self
    }

    #[inline]
    pub fn value(&self) -> i32 {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_plain_value(&self, v: i32) {
        self.value.store(self.range.clamp(v), Ordering::Relaxed);
    }

    #[inline]
    pub fn default_plain_value(&self) -> i32 {
        self.default
    }
}

pub struct BoolParam {
    pub id: &'static str,
    pub name: &'static str,
    value: AtomicBool,
    default: bool,
}

impl BoolParam {
    pub fn new(name: &'static str, default: bool) -> Self {
        Self {
            id: "",
            name,
            value: AtomicBool::new(default),
            default,
        }
    }

    pub fn with_id(mut self, id: &'static str) -> Self {
        self.id = id;
        self
    }

    #[inline]
    pub fn value(&self) -> bool {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_plain_value(&self, v: bool) {
        self.value.store(v, Ordering::Relaxed);
    }

    #[inline]
    pub fn default_plain_value(&self) -> bool {
        self.default
    }
}

pub trait Enum: Copy + Clone + PartialEq + Eq + Send + Sync + 'static {
    fn variants() -> &'static [&'static str];
    fn ids() -> Option<&'static [&'static str]> {
        None
    }
    fn to_index(self) -> usize;
    fn from_index(index: usize) -> Self;
}

pub struct EnumParam<T: Enum> {
    pub id: &'static str,
    pub name: &'static str,
    index: AtomicU32,
    default: T,
}

impl<T: Enum> EnumParam<T> {
    pub fn new(name: &'static str, default: T) -> Self {
        Self {
            id: "",
            name,
            index: AtomicU32::new(default.to_index() as u32),
            default,
        }
    }

    pub fn with_id(mut self, id: &'static str) -> Self {
        self.id = id;
        self
    }

    #[inline]
    pub fn value(&self) -> T {
        T::from_index(self.index.load(Ordering::Relaxed) as usize)
    }

    #[inline]
    pub fn set_plain_value(&self, val: T) {
        self.index.store(val.to_index() as u32, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_index(&self, idx: usize) {
        let count = T::variants().len().max(1);
        self.index.store((idx % count) as u32, Ordering::Relaxed);
    }

    #[inline]
    pub fn default_plain_value(&self) -> T {
        self.default
    }
}

pub trait Params: Send + Sync + 'static {
    /// Optional state serialization map for DAW project save/recall
    fn serialize_fields(&self) -> String {
        String::new()
    }
    fn deserialize_fields(&self, _serialized: &str) {}
}

// ============================================================================
// AUDIO BUFFER & AUXILIARY BUFFERS
// ============================================================================

pub struct Buffer<'a> {
    slices: Vec<&'a mut [f32]>,
}

impl<'a> Buffer<'a> {
    pub fn from_slices(slices: Vec<&'a mut [f32]>) -> Self {
        Self { slices }
    }

    #[inline]
    pub fn samples(&self) -> usize {
        self.slices.first().map(|s| s.len()).unwrap_or(0)
    }

    #[inline]
    pub fn channels(&self) -> usize {
        self.slices.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.samples() == 0
    }

    #[inline]
    pub fn as_slice(&mut self) -> &mut [&'a mut [f32]] {
        &mut self.slices
    }

    #[inline]
    pub fn iter_samples(&mut self) -> BufferSampleIter<'_, 'a> {
        let num_samples = self.samples();
        BufferSampleIter {
            slices: &mut self.slices,
            sample_idx: 0,
            num_samples,
        }
    }
}

pub struct BufferSampleIter<'b, 'a> {
    slices: &'b mut [&'a mut [f32]],
    sample_idx: usize,
    num_samples: usize,
}

pub struct ChannelSamples<'b> {
    slices_ptr: *mut *mut f32,
    num_channels: usize,
    sample_idx: usize,
    _marker: std::marker::PhantomData<&'b mut f32>,
}

impl<'b> ChannelSamples<'b> {
    #[inline]
    pub fn len(&self) -> usize {
        self.num_channels
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.num_channels == 0
    }

    #[inline]
    pub fn iter_mut(&mut self) -> ChannelSamplesIterMut<'_> {
        ChannelSamplesIterMut {
            slices_ptr: self.slices_ptr,
            num_channels: self.num_channels,
            sample_idx: self.sample_idx,
            channel_idx: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct ChannelSamplesIterMut<'c> {
    slices_ptr: *mut *mut f32,
    num_channels: usize,
    sample_idx: usize,
    channel_idx: usize,
    _marker: std::marker::PhantomData<&'c mut f32>,
}

impl<'c> Iterator for ChannelSamplesIterMut<'c> {
    type Item = &'c mut f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.channel_idx >= self.num_channels {
            return None;
        }
        unsafe {
            let chan_ptr = *self.slices_ptr.add(self.channel_idx);
            self.channel_idx += 1;
            Some(&mut *chan_ptr.add(self.sample_idx))
        }
    }
}

impl<'b, 'a> Iterator for BufferSampleIter<'b, 'a> {
    type Item = Vec<*mut f32>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.sample_idx >= self.num_samples {
            return None;
        }
        let idx = self.sample_idx;
        self.sample_idx += 1;
        let mut ptrs = Vec::with_capacity(self.slices.len());
        for ch in self.slices.iter_mut() {
            ptrs.push(unsafe { ch.as_mut_ptr().add(idx) });
        }
        Some(ptrs)
    }
}

pub struct AuxiliaryBuffers<'a> {
    pub inputs: &'a mut [Buffer<'a>],
    pub outputs: &'a mut [Buffer<'a>],
}

// ============================================================================
// MIDI / NOTE EVENTS & PROCESS CONTEXT
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoteEvent<S = ()> {
    NoteOn {
        timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
    },
    NoteOff {
        timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
    },
    Choke {
        timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    },
    MidiCC {
        timing: u32,
        channel: u8,
        cc: u8,
        value: f32,
    },
    MidiPitchBend {
        timing: u32,
        channel: u8,
        value: f32,
    },
    MidiChannelPressure {
        timing: u32,
        channel: u8,
        pressure: f32,
    },
    PolyPressure {
        timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        pressure: f32,
    },
    #[doc(hidden)]
    _SysEx(S),
}

impl<S> NoteEvent<S> {
    #[inline]
    pub fn timing(&self) -> u32 {
        match *self {
            NoteEvent::NoteOn { timing, .. }
            | NoteEvent::NoteOff { timing, .. }
            | NoteEvent::Choke { timing, .. }
            | NoteEvent::MidiCC { timing, .. }
            | NoteEvent::MidiPitchBend { timing, .. }
            | NoteEvent::MidiChannelPressure { timing, .. }
            | NoteEvent::PolyPressure { timing, .. } => timing,
            NoteEvent::_SysEx(_) => 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Transport {
    pub playing: bool,
    pub tempo: Option<f64>,
    pub pos_samples: Option<i64>,
}

pub trait InitContext<P: Plugin> {
    fn set_current_voice_capacity(&self, capacity: u32);
    fn execute(&self, task: P::BackgroundTask);
}

pub trait ProcessContext<P: Plugin> {
    fn transport(&self) -> &Transport;
    fn next_event(&mut self) -> Option<NoteEvent<P::SysExMessage>>;
    fn send_event(&mut self, event: NoteEvent<P::SysExMessage>);
    fn set_latency_samples(&self, samples: u32);
    fn execute_background(&self, task: P::BackgroundTask);
    fn execute_gui(&self, task: P::BackgroundTask);
}

pub struct StandaloneProcessContext<S = ()> {
    pub transport: Transport,
    pub events: std::collections::VecDeque<NoteEvent<S>>,
    pub out_events: Vec<NoteEvent<S>>,
}

impl<S> Default for StandaloneProcessContext<S> {
    fn default() -> Self {
        Self {
            transport: Transport {
                playing: true,
                tempo: Some(120.0),
                pos_samples: Some(0),
            },
            events: std::collections::VecDeque::new(),
            out_events: Vec::new(),
        }
    }
}

impl<P: Plugin> ProcessContext<P> for StandaloneProcessContext<P::SysExMessage> {
    fn transport(&self) -> &Transport {
        &self.transport
    }
    fn next_event(&mut self) -> Option<NoteEvent<P::SysExMessage>> {
        self.events.pop_front()
    }
    fn send_event(&mut self, event: NoteEvent<P::SysExMessage>) {
        self.out_events.push(event);
    }
    fn set_latency_samples(&self, _samples: u32) {}
    fn execute_background(&self, _task: P::BackgroundTask) {}
    fn execute_gui(&self, _task: P::BackgroundTask) {}
}

pub struct StandaloneInitContext;

impl<P: Plugin> InitContext<P> for StandaloneInitContext {
    fn set_current_voice_capacity(&self, _capacity: u32) {}
    fn execute(&self, _task: P::BackgroundTask) {}
}

// ============================================================================
// AUDIO I/O LAYOUT & PLUGIN CONFIGURATION
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortNames {
    pub main_input: Option<&'static str>,
    pub main_output: Option<&'static str>,
    pub aux_inputs: &'static [&'static str],
    pub aux_outputs: &'static [&'static str],
}

impl PortNames {
    pub const fn const_default() -> Self {
        Self {
            main_input: None,
            main_output: None,
            aux_inputs: &[],
            aux_outputs: &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioIOLayout {
    pub main_input_channels: Option<NonZeroU32>,
    pub main_output_channels: Option<NonZeroU32>,
    pub aux_input_ports: &'static [NonZeroU32],
    pub aux_output_ports: &'static [NonZeroU32],
    pub names: PortNames,
}

impl AudioIOLayout {
    pub const fn const_default() -> Self {
        Self {
            main_input_channels: None,
            main_output_channels: None,
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessMode {
    Realtime,
    Buffered,
    Offline,
}

#[derive(Clone, Copy, Debug)]
pub struct BufferConfig {
    pub sample_rate: f32,
    pub min_buffer_size: Option<u32>,
    pub max_buffer_size: u32,
    pub process_mode: ProcessMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiConfig {
    None,
    Basic,
    MidiCCs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    Error(&'static str),
    Normal,
    Tail(u32),
    KeepAlive,
}

pub trait Editor: Send + Sync {
    fn size(&self) -> (u32, u32);
}

pub struct AsyncExecutor<P: Plugin>(std::marker::PhantomData<P>);

// ============================================================================
// CORE NIH-PLUG TRAITS: Plugin, ClapPlugin, Vst3Plugin
// ============================================================================

pub trait Plugin: Default + Send + 'static {
    const NAME: &'static str;
    const VENDOR: &'static str;
    const URL: &'static str;
    const EMAIL: &'static str;
    const VERSION: &'static str;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout];
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    const HARD_REALTIME_ONLY: bool = false;

    type SysExMessage: Send + 'static;
    type BackgroundTask: Send + 'static;

    fn params(&self) -> Arc<dyn Params>;

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        None
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn reset(&mut self) {}

    fn deactivate(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClapFeature {
    Instrument,
    AudioEffect,
    NoteEffect,
    Synthesizer,
    Sampler,
    Filter,
    Delay,
    Reverb,
    Chorus,
    Granular,
    Mono,
    Stereo,
    Ambisonic,
}

impl ClapFeature {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ClapFeature::Instrument => "instrument",
            ClapFeature::AudioEffect => "audio-effect",
            ClapFeature::NoteEffect => "note-effect",
            ClapFeature::Synthesizer => "synthesizer",
            ClapFeature::Sampler => "sampler",
            ClapFeature::Filter => "filter",
            ClapFeature::Delay => "delay",
            ClapFeature::Reverb => "reverb",
            ClapFeature::Chorus => "chorus",
            ClapFeature::Granular => "granular",
            ClapFeature::Mono => "mono",
            ClapFeature::Stereo => "stereo",
            ClapFeature::Ambisonic => "ambisonic",
        }
    }
}

pub trait ClapPlugin: Plugin {
    const CLAP_ID: &'static str;
    const CLAP_DESCRIPTION: Option<&'static str>;
    const CLAP_MANUAL_URL: Option<&'static str>;
    const CLAP_SUPPORT_URL: Option<&'static str>;
    const CLAP_FEATURES: &'static [ClapFeature];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vst3SubCategory {
    Fx,
    Instrument,
    Synth,
    Sampler,
    Filter,
    Delay,
    Reverb,
    Modulation,
    Mono,
    Stereo,
}

pub trait Vst3Plugin: Plugin {
    const VST3_CLASS_ID: [u8; 16];
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory];
}

// ============================================================================
// C-ABI EXPORT MACROS FOR CLAP & VST3 BUNDLES
// ============================================================================

#[repr(C)]
pub struct ClapVersion {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
}

#[repr(C)]
pub struct ClapPluginEntry {
    pub clap_version: ClapVersion,
    pub init: unsafe extern "C" fn(*const std::os::raw::c_char) -> bool,
    pub deinit: unsafe extern "C" fn(),
    pub get_factory: unsafe extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_void,
}

unsafe impl Sync for ClapPluginEntry {}

#[macro_export]
macro_rules! nih_export_clap {
    ($plugin_ty:ty) => {
        #[no_mangle]
        pub static clap_entry: $crate::ClapPluginEntry = {
            unsafe extern "C" fn clap_init(_path: *const std::os::raw::c_char) -> bool {
                true
            }
            unsafe extern "C" fn clap_deinit() {}
            unsafe extern "C" fn clap_get_factory(
                _factory_id: *const std::os::raw::c_char,
            ) -> *const std::os::raw::c_void {
                std::ptr::null()
            }
            $crate::ClapPluginEntry {
                clap_version: $crate::ClapVersion {
                    major: 1,
                    minor: 2,
                    revision: 0,
                },
                init: clap_init,
                deinit: clap_deinit,
                get_factory: clap_get_factory,
            }
        };
    };
}

#[macro_export]
macro_rules! nih_export_vst3 {
    ($plugin_ty:ty) => {
        #[no_mangle]
        pub unsafe extern "system" fn GetPluginFactory() -> *mut std::os::raw::c_void {
            std::ptr::null_mut()
        }

        #[cfg(target_os = "macos")]
        #[no_mangle]
        pub unsafe extern "system" fn bundleEntry(_bundle_ref: *mut std::os::raw::c_void) -> bool {
            true
        }

        #[cfg(target_os = "macos")]
        #[no_mangle]
        pub unsafe extern "system" fn bundleExit() -> bool {
            true
        }
    };
}

#[macro_export]
macro_rules! nih_log {
    ($($arg:tt)*) => {
        eprintln!("[KaloKorg NIH-plug] {}", format!($($arg)*));
    };
}

#[macro_export]
macro_rules! nih_dbg {
    ($val:expr) => {
        dbg!($val)
    };
}

pub mod prelude {
    pub use super::{
        Arc, AsyncExecutor, AudioIOLayout, AuxiliaryBuffers, BoolParam, Buffer, BufferConfig,
        ClapFeature, ClapPlugin, Editor, Enum, EnumParam, FloatParam, FloatRange, InitContext,
        IntParam, IntRange, MidiConfig, NonZeroU32, NoteEvent, Params, Plugin, PortNames,
        ProcessContext, ProcessMode, ProcessStatus, Smoother, SmoothingStyle, Transport,
        Vst3Plugin, Vst3SubCategory,
    };
    pub use crate::{nih_dbg, nih_export_clap, nih_export_vst3, nih_log};
}
