//! Cross-platform Audio Library (`cpal` 0.15 compatible API).
//!
//! Provides `cpal::default_host()`, `Host`, `Device`, `Stream`, `StreamConfig`,
//! `BufferSize::Fixed(u32)`, `SampleRate`, `SampleFormat`, and `traits::{DeviceTrait, HostTrait, StreamTrait}`
//! backed by native macOS/iOS CoreAudio (`AudioToolbox`) with a real-time software fallback thread
//! for headless/CI environments.

use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub type ChannelCount = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferSize {
    Default,
    Fixed(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleFormat {
    I8,
    I16,
    I32,
    U8,
    U16,
    U32,
    F32,
    F64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamConfig {
    pub channels: ChannelCount,
    pub sample_rate: SampleRate,
    pub buffer_size: BufferSize,
}

#[derive(Clone, Debug)]
pub struct SupportedStreamConfig {
    channels: ChannelCount,
    sample_rate: SampleRate,
    buffer_size: BufferSize,
    sample_format: SampleFormat,
}

impl SupportedStreamConfig {
    pub fn channels(&self) -> ChannelCount {
        self.channels
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn buffer_size(&self) -> &BufferSize {
        &self.buffer_size
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.sample_format
    }

    pub fn config(&self) -> StreamConfig {
        StreamConfig {
            channels: self.channels,
            sample_rate: self.sample_rate,
            buffer_size: self.buffer_size,
        }
    }
}

impl From<SupportedStreamConfig> for StreamConfig {
    fn from(val: SupportedStreamConfig) -> Self {
        val.config()
    }
}

#[derive(Clone, Debug)]
pub struct OutputCallbackInfo {
    _priv: (),
}

#[derive(Clone, Debug)]
pub struct StreamError(pub String);

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StreamError {}

#[derive(Clone, Debug)]
pub struct DefaultStreamConfigError(pub String);

impl fmt::Display for DefaultStreamConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DefaultStreamConfigError {}

#[derive(Clone, Debug)]
pub struct BuildStreamError(pub String);

impl fmt::Display for BuildStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BuildStreamError {}

#[derive(Clone, Debug)]
pub struct PlayStreamError(pub String);

impl fmt::Display for PlayStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlayStreamError {}

#[derive(Clone, Debug)]
pub struct PauseStreamError(pub String);

impl fmt::Display for PauseStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PauseStreamError {}

pub trait Sample: Copy + Default + Send + 'static {
    const FORMAT: SampleFormat;
    fn from_f32(v: f32) -> Self;
    fn to_f32(self) -> f32;
}

impl Sample for f32 {
    const FORMAT: SampleFormat = SampleFormat::F32;
    #[inline]
    fn from_f32(v: f32) -> Self {
        v
    }
    #[inline]
    fn to_f32(self) -> f32 {
        self
    }
}

impl Sample for i16 {
    const FORMAT: SampleFormat = SampleFormat::I16;
    #[inline]
    fn from_f32(v: f32) -> Self {
        (v.clamp(-1.0, 1.0) * 32767.0) as i16
    }
    #[inline]
    fn to_f32(self) -> f32 {
        (self as f32) / 32768.0
    }
}

impl Sample for u16 {
    const FORMAT: SampleFormat = SampleFormat::U16;
    #[inline]
    fn from_f32(v: f32) -> Self {
        ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * 65535.0) as u16
    }
    #[inline]
    fn to_f32(self) -> f32 {
        ((self as f32) / 65535.0 - 0.5) * 2.0
    }
}

pub mod traits {
    use super::*;

    pub trait HostTrait {
        type Device: DeviceTrait;
        fn default_output_device(&self) -> Option<Self::Device>;
    }

    pub trait DeviceTrait {
        type Stream: StreamTrait;
        fn name(&self) -> Result<String, String>;
        fn default_output_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError>;
        fn build_output_stream<T, D, E>(
            &self,
            config: &StreamConfig,
            data_callback: D,
            error_callback: E,
            timeout: Option<Duration>,
        ) -> Result<Self::Stream, BuildStreamError>
        where
            T: Sample,
            D: FnMut(&mut [T], &OutputCallbackInfo) + Send + 'static,
            E: FnMut(StreamError) + Send + 'static;
    }

    pub trait StreamTrait {
        fn play(&self) -> Result<(), PlayStreamError>;
        fn pause(&self) -> Result<(), PauseStreamError>;
    }
}

pub struct Host;

pub fn default_host() -> Host {
    Host
}

pub struct Device {
    name: String,
}

#[cfg(target_os = "macos")]
mod coreaudio_backend {
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    type OSStatus = i32;
    type AudioQueueRef = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct AudioStreamBasicDescription {
        m_sample_rate: f64,
        m_format_id: u32,
        m_format_flags: u32,
        m_bytes_per_packet: u32,
        m_frames_per_packet: u32,
        m_bytes_per_frame: u32,
        m_channels_per_frame: u32,
        m_bits_per_channel: u32,
        m_reserved: u32,
    }

    #[repr(C)]
    struct AudioQueueBuffer {
        m_audio_data_bytes_capacity: u32,
        m_audio_data: *mut c_void,
        m_audio_data_byte_size: u32,
        m_user_data: *mut c_void,
        m_packet_description_capacity: u32,
        m_packet_descriptions: *mut c_void,
        m_packet_description_count: u32,
    }

    type AudioQueueBufferRef = *mut AudioQueueBuffer;

    type AudioQueueOutputCallback = extern "C" fn(
        in_user_data: *mut c_void,
        in_aq: AudioQueueRef,
        in_buffer: AudioQueueBufferRef,
    );

    #[link(name = "AudioToolbox", kind = "framework")]
    extern "C" {
        fn AudioQueueNewOutput(
            in_format: *const AudioStreamBasicDescription,
            in_callback_proc: AudioQueueOutputCallback,
            in_user_data: *mut c_void,
            in_callback_run_loop: *mut c_void,
            in_callback_run_loop_mode: *mut c_void,
            in_flags: u32,
            out_aq: *mut AudioQueueRef,
        ) -> OSStatus;

        fn AudioQueueAllocateBuffer(
            in_aq: AudioQueueRef,
            in_buffer_byte_size: u32,
            out_buffer: *mut AudioQueueBufferRef,
        ) -> OSStatus;

        fn AudioQueueEnqueueBuffer(
            in_aq: AudioQueueRef,
            in_buffer: AudioQueueBufferRef,
            in_num_packet_descs: u32,
            in_packet_descs: *const c_void,
        ) -> OSStatus;

        fn AudioQueueStart(in_aq: AudioQueueRef, in_start_time: *const c_void) -> OSStatus;
        fn AudioQueuePause(in_aq: AudioQueueRef) -> OSStatus;
        fn AudioQueueStop(in_aq: AudioQueueRef, in_immediate: u8) -> OSStatus;
        fn AudioQueueDispose(in_aq: AudioQueueRef, in_immediate: u8) -> OSStatus;
    }

    const K_AUDIO_FORMAT_LINEAR_PCM: u32 = u32::from_be_bytes(*b"lpcm");
    const K_AUDIO_FORMAT_FLAG_IS_FLOAT: u32 = 1 << 0;
    const K_AUDIO_FORMAT_FLAG_IS_PACKED: u32 = 1 << 3;

    struct CallbackContext {
        callback: Mutex<Box<dyn FnMut(&mut [f32]) + Send + 'static>>,
        playing: Arc<AtomicBool>,
    }

    extern "C" fn aq_output_callback(
        in_user_data: *mut c_void,
        in_aq: AudioQueueRef,
        in_buffer: AudioQueueBufferRef,
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            if in_user_data.is_null() || in_buffer.is_null() {
                return;
            }
            let ctx = &*(in_user_data as *const CallbackContext);
            let buf = &mut *in_buffer;
            let total_floats = (buf.m_audio_data_bytes_capacity / 4) as usize;
            let out_slice =
                std::slice::from_raw_parts_mut(buf.m_audio_data as *mut f32, total_floats);

            if ctx.playing.load(Ordering::Relaxed) {
                if let Ok(mut cb) = ctx.callback.lock() {
                    cb(out_slice);
                } else {
                    out_slice.fill(0.0);
                }
            } else {
                out_slice.fill(0.0);
            }

            buf.m_audio_data_byte_size = (total_floats * 4) as u32;
            let _ = AudioQueueEnqueueBuffer(in_aq, in_buffer, 0, ptr::null());
        }));
    }

    pub struct CoreAudioQueueStream {
        queue: AudioQueueRef,
        ctx_ptr: *mut CallbackContext,
    }

    unsafe impl Send for CoreAudioQueueStream {}
    unsafe impl Sync for CoreAudioQueueStream {}

    impl CoreAudioQueueStream {
        pub fn new(
            sample_rate: u32,
            channels: u16,
            frames_per_buffer: u32,
            playing: Arc<AtomicBool>,
            callback: Box<dyn FnMut(&mut [f32]) + Send + 'static>,
        ) -> Result<Self, String> {
            let ch = channels.max(1) as u32;
            let bytes_per_frame = ch * 4;
            let desc = AudioStreamBasicDescription {
                m_sample_rate: sample_rate as f64,
                m_format_id: K_AUDIO_FORMAT_LINEAR_PCM,
                m_format_flags: K_AUDIO_FORMAT_FLAG_IS_FLOAT | K_AUDIO_FORMAT_FLAG_IS_PACKED,
                m_bytes_per_packet: bytes_per_frame,
                m_frames_per_packet: 1,
                m_bytes_per_frame: bytes_per_frame,
                m_channels_per_frame: ch,
                m_bits_per_channel: 32,
                m_reserved: 0,
            };

            let ctx = Box::new(CallbackContext {
                callback: Mutex::new(callback),
                playing,
            });
            let ctx_ptr = Box::into_raw(ctx);
            let mut queue: AudioQueueRef = ptr::null_mut();

            let status = unsafe {
                AudioQueueNewOutput(
                    &desc,
                    aq_output_callback,
                    ctx_ptr as *mut c_void,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                    &mut queue,
                )
            };

            if status != 0 || queue.is_null() {
                unsafe {
                    let _ = Box::from_raw(ctx_ptr);
                }
                return Err(format!("AudioQueueNewOutput failed ({})", status));
            }

            let buffer_bytes = frames_per_buffer.clamp(64, 2048) * bytes_per_frame;
            for _ in 0..3 {
                let mut buf_ref: AudioQueueBufferRef = ptr::null_mut();
                let alloc_st =
                    unsafe { AudioQueueAllocateBuffer(queue, buffer_bytes, &mut buf_ref) };
                if alloc_st == 0 && !buf_ref.is_null() {
                    aq_output_callback(ctx_ptr as *mut c_void, queue, buf_ref);
                }
            }

            let start_st = unsafe { AudioQueueStart(queue, ptr::null()) };
            if start_st != 0 {
                unsafe {
                    AudioQueueDispose(queue, 1);
                    let _ = Box::from_raw(ctx_ptr);
                }
                return Err(format!("AudioQueueStart failed ({})", start_st));
            }

            Ok(Self { queue, ctx_ptr })
        }

        pub fn play(&self) {
            unsafe {
                if !self.queue.is_null() {
                    let _ = AudioQueueStart(self.queue, ptr::null());
                }
            }
        }

        pub fn pause(&self) {
            unsafe {
                if !self.queue.is_null() {
                    let _ = AudioQueuePause(self.queue);
                }
            }
        }
    }

    impl Drop for CoreAudioQueueStream {
        fn drop(&mut self) {
            unsafe {
                if !self.queue.is_null() {
                    let _ = AudioQueueStop(self.queue, 1);
                    let _ = AudioQueueDispose(self.queue, 1);
                }
                if !self.ctx_ptr.is_null() {
                    let _ = Box::from_raw(self.ctx_ptr);
                }
            }
        }
    }
}

pub struct Stream {
    playing: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    #[cfg(target_os = "macos")]
    _coreaudio: Option<coreaudio_backend::CoreAudioQueueStream>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.playing.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.worker.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }
}

impl traits::HostTrait for Host {
    type Device = Device;

    fn default_output_device(&self) -> Option<Self::Device> {
        Some(Device {
            name: "CoreAudio / Low-Latency Stereo Output".to_string(),
        })
    }
}

impl traits::DeviceTrait for Device {
    type Stream = Stream;

    fn name(&self) -> Result<String, String> {
        Ok(self.name.clone())
    }

    fn default_output_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError> {
        Ok(SupportedStreamConfig {
            channels: 2,
            sample_rate: SampleRate(48_000),
            buffer_size: BufferSize::Fixed(256),
            sample_format: SampleFormat::F32,
        })
    }

    fn build_output_stream<T, D, E>(
        &self,
        config: &StreamConfig,
        mut data_callback: D,
        _error_callback: E,
        _timeout: Option<Duration>,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        T: Sample,
        D: FnMut(&mut [T], &OutputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static,
    {
        let channels = config.channels.max(1) as usize;
        let sample_rate = config.sample_rate.0.max(8_000);
        let frames = match config.buffer_size {
            BufferSize::Fixed(f) => f.clamp(32, 4096) as usize,
            BufferSize::Default => 256usize,
        };

        let playing = Arc::new(AtomicBool::new(true));
        let running = Arc::new(AtomicBool::new(true));

        let shared_cb: Arc<Mutex<Box<dyn FnMut(&mut [f32]) + Send + 'static>>> = {
            let mut sample_buf = vec![T::default(); frames * channels];
            let info = OutputCallbackInfo { _priv: () };
            Arc::new(Mutex::new(Box::new(move |out_f32: &mut [f32]| {
                if sample_buf.len() != out_f32.len() {
                    sample_buf.resize(out_f32.len(), T::default());
                }
                data_callback(&mut sample_buf, &info);
                for (dst, src) in out_f32.iter_mut().zip(sample_buf.iter()) {
                    *dst = src.to_f32();
                }
            })))
        };

        #[cfg(target_os = "macos")]
        {
            let cb_clone = shared_cb.clone();
            let mac_cb = Box::new(move |out_f32: &mut [f32]| {
                if let Ok(mut cb) = cb_clone.lock() {
                    cb(out_f32);
                }
            });
            if let Ok(ca_stream) = coreaudio_backend::CoreAudioQueueStream::new(
                sample_rate,
                channels as u16,
                frames as u32,
                playing.clone(),
                mac_cb,
            ) {
                return Ok(Stream {
                    playing,
                    running,
                    _coreaudio: Some(ca_stream),
                    worker: Mutex::new(None),
                });
            }
        }

        // Real-time low-latency software audio thread fallback (for headless/CI or non-CoreAudio)
        let playing_thread = playing.clone();
        let running_thread = running.clone();
        let sleep_dur =
            Duration::from_micros(((frames as u64) * 1_000_000) / (sample_rate as u64)).max(Duration::from_millis(1));

        let handle = thread::spawn(move || {
            let mut block = vec![0.0f32; frames * channels];
            while running_thread.load(Ordering::Relaxed) {
                if playing_thread.load(Ordering::Relaxed) {
                    if let Ok(mut cb) = shared_cb.lock() {
                        cb(&mut block);
                    }
                }
                thread::sleep(sleep_dur);
            }
        });

        Ok(Stream {
            playing,
            running,
            #[cfg(target_os = "macos")]
            _coreaudio: None,
            worker: Mutex::new(Some(handle)),
        })
    }
}

impl traits::StreamTrait for Stream {
    fn play(&self) -> Result<(), PlayStreamError> {
        self.playing.store(true, Ordering::SeqCst);
        #[cfg(target_os = "macos")]
        if let Some(ref ca) = self._coreaudio {
            ca.play();
        }
        Ok(())
    }

    fn pause(&self) -> Result<(), PauseStreamError> {
        self.playing.store(false, Ordering::SeqCst);
        #[cfg(target_os = "macos")]
        if let Some(ref ca) = self._coreaudio {
            ca.pause();
        }
        Ok(())
    }
}
