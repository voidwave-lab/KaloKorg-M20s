//! Google Oboe (`oboe` 0.6 compatible API) for low-latency Android audio streaming.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerformanceMode {
    None,
    PowerSaving,
    LowLatency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharingMode {
    Exclusive,
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataCallbackResult {
    Continue,
    Stop,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Output;

#[derive(Clone, Copy, Debug, Default)]
pub struct Stereo;

#[derive(Clone, Copy, Debug, Default)]
pub struct Mono;

pub struct AudioOutputStreamSafe {
    sample_rate: i32,
}

impl AudioOutputStreamSafe {
    pub fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }
}

pub trait AudioOutputCallback: Send + 'static {
    type FrameType: Copy + Default;

    fn on_audio_ready(
        &mut self,
        stream: &mut dyn AudioOutputStreamSafeTrait,
        frames: &mut [Self::FrameType],
    ) -> DataCallbackResult;
}

pub trait AudioOutputStreamSafeTrait {
    fn get_sample_rate(&self) -> i32;
}

impl AudioOutputStreamSafeTrait for AudioOutputStreamSafe {
    fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }
}

pub trait AudioStream {
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn get_sample_rate(&self) -> i32;
}

pub struct AudioStreamAsync<Dir, Callback> {
    sample_rate: i32,
    frames_per_callback: i32,
    _dir: std::marker::PhantomData<Dir>,
    _cb: Option<Callback>,
}

impl<Dir, Callback> AudioStream for AudioStreamAsync<Dir, Callback> {
    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }
}

impl<Dir, Callback> AudioStreamAsync<Dir, Callback> {
    pub fn frames_per_callback(&self) -> i32 {
        self.frames_per_callback
    }
}

pub struct AudioStreamBuilder<Dir = Output, Ch = Stereo, Fmt = f32, Cb = ()> {
    performance_mode: PerformanceMode,
    sharing_mode: SharingMode,
    sample_rate: i32,
    frames_per_callback: i32,
    callback: Option<Cb>,
    _phantom: std::marker::PhantomData<(Dir, Ch, Fmt)>,
}

impl Default for AudioStreamBuilder<Output, Stereo, f32, ()> {
    fn default() -> Self {
        Self {
            performance_mode: PerformanceMode::LowLatency,
            sharing_mode: SharingMode::Exclusive,
            sample_rate: 48_000,
            frames_per_callback: 256,
            callback: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Dir, Ch, Fmt, Cb> AudioStreamBuilder<Dir, Ch, Fmt, Cb> {
    pub fn set_performance_mode(mut self, mode: PerformanceMode) -> Self {
        self.performance_mode = mode;
        self
    }

    pub fn set_sharing_mode(mut self, mode: SharingMode) -> Self {
        self.sharing_mode = mode;
        self
    }

    pub fn set_sample_rate(mut self, sr: i32) -> Self {
        self.sample_rate = sr;
        self
    }

    pub fn set_frames_per_callback(mut self, frames: i32) -> Self {
        self.frames_per_callback = frames;
        self
    }

    pub fn set_stereo(self) -> AudioStreamBuilder<Dir, Stereo, Fmt, Cb> {
        AudioStreamBuilder {
            performance_mode: self.performance_mode,
            sharing_mode: self.sharing_mode,
            sample_rate: self.sample_rate,
            frames_per_callback: self.frames_per_callback,
            callback: self.callback,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn set_output(self) -> AudioStreamBuilder<Output, Ch, Fmt, Cb> {
        AudioStreamBuilder {
            performance_mode: self.performance_mode,
            sharing_mode: self.sharing_mode,
            sample_rate: self.sample_rate,
            frames_per_callback: self.frames_per_callback,
            callback: self.callback,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn set_f32(self) -> AudioStreamBuilder<Dir, Ch, f32, Cb> {
        AudioStreamBuilder {
            performance_mode: self.performance_mode,
            sharing_mode: self.sharing_mode,
            sample_rate: self.sample_rate,
            frames_per_callback: self.frames_per_callback,
            callback: self.callback,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn set_callback<NewCb: AudioOutputCallback>(
        self,
        cb: NewCb,
    ) -> AudioStreamBuilder<Dir, Ch, Fmt, NewCb> {
        AudioStreamBuilder {
            performance_mode: self.performance_mode,
            sharing_mode: self.sharing_mode,
            sample_rate: self.sample_rate,
            frames_per_callback: self.frames_per_callback,
            callback: Some(cb),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn open_stream(self) -> Result<AudioStreamAsync<Dir, Cb>, String> {
        Ok(AudioStreamAsync {
            sample_rate: self.sample_rate,
            frames_per_callback: self.frames_per_callback,
            _dir: std::marker::PhantomData,
            _cb: self.callback,
        })
    }
}
