use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ms20_core::engine::KaloKorgEngine;
use crate::recorder::SessionRecorder;

/// Shared real-time state between the audio callback, TUI, and Web Server
pub struct SharedSynthState {
    pub engine: KaloKorgEngine,
    pub recorder: SessionRecorder,
    pub midi_ports: Vec<String>,
    pub active_midi_port: Option<usize>,
    pub last_midi_message: String,
    pub selected_preset_idx: usize,
    pub audio_backend_name: String,
}

impl SharedSynthState {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            engine: KaloKorgEngine::new(sample_rate as f32),
            recorder: SessionRecorder::new(sample_rate),
            midi_ports: Vec::new(),
            active_midi_port: None,
            last_midi_message: "Ready (MIDI & Keyboard Active)".to_string(),
            selected_preset_idx: 0,
            audio_backend_name: "CoreAudio 48kHz Stereo".to_string(),
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_audio {
    use super::*;

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
        fn AudioQueueStop(in_aq: AudioQueueRef, in_immediate: u8) -> OSStatus;
        fn AudioQueueDispose(in_aq: AudioQueueRef, in_immediate: u8) -> OSStatus;
    }

    const K_AUDIO_FORMAT_LINEAR_PCM: u32 = u32::from_be_bytes(*b"lpcm");
    const K_AUDIO_FORMAT_FLAG_IS_FLOAT: u32 = 1 << 0;
    const K_AUDIO_FORMAT_FLAG_IS_PACKED: u32 = 1 << 3;

    extern "C" fn audio_queue_callback(
        in_user_data: *mut c_void,
        in_aq: AudioQueueRef,
        in_buffer: AudioQueueBufferRef,
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            if in_user_data.is_null() || in_buffer.is_null() {
                return;
            }
            let state_arc = &*(in_user_data as *const Arc<Mutex<SharedSynthState>>);
            let buf = &mut *in_buffer;
            let total_floats = (buf.m_audio_data_bytes_capacity / 4) as usize;
            let frames = total_floats / 2;
            let out_slice =
                std::slice::from_raw_parts_mut(buf.m_audio_data as *mut f32, frames * 2);

            if let Ok(mut guard) = state_arc.lock() {
                for i in 0..frames {
                    let (l, r) = guard.engine.render_frame();
                    out_slice[i * 2] = l;
                    out_slice[i * 2 + 1] = r;
                    if guard.recorder.is_recording {
                        guard.recorder.push_stereo(l, r);
                    }
                }
            } else {
                for sample in out_slice.iter_mut() {
                    *sample = 0.0;
                }
            }

            buf.m_audio_data_byte_size = (frames * 2 * 4) as u32;
            let _ = AudioQueueEnqueueBuffer(in_aq, in_buffer, 0, ptr::null());
        }));
    }

    pub struct MacAudioStream {
        queue: AudioQueueRef,
        user_data_ptr: *mut Arc<Mutex<SharedSynthState>>,
    }

    unsafe impl Send for MacAudioStream {}

    impl MacAudioStream {
        pub fn start(state: Arc<Mutex<SharedSynthState>>, sample_rate: u32) -> Result<Self, String> {
            let desc = AudioStreamBasicDescription {
                m_sample_rate: sample_rate as f64,
                m_format_id: K_AUDIO_FORMAT_LINEAR_PCM,
                m_format_flags: K_AUDIO_FORMAT_FLAG_IS_FLOAT | K_AUDIO_FORMAT_FLAG_IS_PACKED,
                m_bytes_per_packet: 8,
                m_frames_per_packet: 1,
                m_bytes_per_frame: 8,
                m_channels_per_frame: 2,
                m_bits_per_channel: 32,
                m_reserved: 0,
            };

            let boxed_state = Box::new(state);
            let user_data_ptr = Box::into_raw(boxed_state);
            let mut queue: AudioQueueRef = ptr::null_mut();

            let status = unsafe {
                AudioQueueNewOutput(
                    &desc,
                    audio_queue_callback,
                    user_data_ptr as *mut c_void,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                    &mut queue,
                )
            };

            if status != 0 || queue.is_null() {
                unsafe {
                    let _ = Box::from_raw(user_data_ptr);
                }
                return Err(format!("CoreAudio AudioQueueNewOutput failed ({})", status));
            }

            // Allocate 3 low-latency buffers of 512 stereo frames (4096 bytes each, ~10.6ms)
            let buffer_bytes: u32 = 512 * 2 * 4;
            for _ in 0..3 {
                let mut buf_ref: AudioQueueBufferRef = ptr::null_mut();
                let alloc_st = unsafe { AudioQueueAllocateBuffer(queue, buffer_bytes, &mut buf_ref) };
                if alloc_st == 0 && !buf_ref.is_null() {
                    audio_queue_callback(user_data_ptr as *mut c_void, queue, buf_ref);
                }
            }

            let start_st = unsafe { AudioQueueStart(queue, ptr::null()) };
            if start_st != 0 {
                unsafe {
                    AudioQueueDispose(queue, 1);
                    let _ = Box::from_raw(user_data_ptr);
                }
                return Err(format!("CoreAudio AudioQueueStart failed ({})", start_st));
            }

            Ok(Self {
                queue,
                user_data_ptr,
            })
        }
    }

    impl Drop for MacAudioStream {
        fn drop(&mut self) {
            unsafe {
                if !self.queue.is_null() {
                    AudioQueueStop(self.queue, 1);
                    AudioQueueDispose(self.queue, 1);
                }
                if !self.user_data_ptr.is_null() {
                    let _ = Box::from_raw(self.user_data_ptr);
                }
            }
        }
    }
}

pub struct AudioHost {
    #[cfg(target_os = "macos")]
    _mac_stream: Option<macos_audio::MacAudioStream>,
}

impl AudioHost {
    pub fn start(state: Arc<Mutex<SharedSynthState>>, sample_rate: u32) -> Self {
        #[cfg(target_os = "macos")]
        {
            match macos_audio::MacAudioStream::start(state.clone(), sample_rate) {
                Ok(stream) => {
                    if let Ok(mut guard) = state.lock() {
                        guard.audio_backend_name =
                            format!("CoreAudio {}Hz Stereo", sample_rate);
                    }
                    return Self {
                        _mac_stream: Some(stream),
                    };
                }
                Err(_) => {
                    // Fallback to software clock thread if headless/sandboxed audio
                }
            }
        }

        // Software real-time clock fallback so scopes, meters, and WAV recording always run
        if let Ok(mut guard) = state.lock() {
            guard.audio_backend_name = format!("DSP Engine {}Hz (WebAudio + WAV Active)", sample_rate);
        }
        thread::spawn(move || {
            let block_frames = 512usize;
            let sleep_dur = Duration::from_micros((block_frames as u64 * 1_000_000) / (sample_rate as u64));
            loop {
                if let Ok(mut guard) = state.lock() {
                    for _ in 0..block_frames {
                        let (l, r) = guard.engine.render_frame();
                        if guard.recorder.is_recording {
                            guard.recorder.push_stereo(l, r);
                        }
                    }
                }
                thread::sleep(sleep_dur);
            }
        });

        Self {
            #[cfg(target_os = "macos")]
            _mac_stream: None,
        }
    }
}
