use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Stereo WAV Session Recorder for KaloKorg M20s
pub struct SessionRecorder {
    pub is_recording: bool,
    pub sample_rate: u32,
    pub samples_interleaved: Vec<f32>,
    pub last_saved_path: Option<String>,
    pub last_wav_bytes: Vec<u8>,
}

impl SessionRecorder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            is_recording: false,
            sample_rate,
            samples_interleaved: Vec::with_capacity(sample_rate as usize * 60 * 2),
            last_saved_path: None,
            last_wav_bytes: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.samples_interleaved.clear();
        self.is_recording = true;
    }

    pub fn push_stereo(&mut self, left: f32, right: f32) {
        if self.is_recording {
            self.samples_interleaved.push(left.clamp(-1.0, 1.0));
            self.samples_interleaved.push(right.clamp(-1.0, 1.0));
        }
    }

    pub fn duration_seconds(&self) -> f32 {
        (self.samples_interleaved.len() / 2) as f32 / (self.sample_rate.max(1) as f32)
    }

    pub fn stop_and_save(&mut self) -> Result<String, String> {
        self.is_recording = false;
        if self.samples_interleaved.is_empty() {
            return Err("No audio frames recorded".to_string());
        }

        let wav_bytes = encode_wav_16bit_stereo(&self.samples_interleaved, self.sample_rate);
        self.last_wav_bytes = wav_bytes.clone();

        let dir = PathBuf::from("recordings");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let filename = format!("kalokorg_m20s_{}.wav", ts);
        let path = dir.join(&filename);

        let mut file = File::create(&path).map_err(|e| e.to_string())?;
        file.write_all(&wav_bytes).map_err(|e| e.to_string())?;

        let display_path = format!("recordings/{}", filename);
        self.last_saved_path = Some(display_path.clone());
        Ok(display_path)
    }
}

/// Encode interleaved stereo f32 samples (-1.0..+1.0) into standard 16-bit PCM RIFF WAV bytes
pub fn encode_wav_16bit_stereo(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let byte_rate: u32 = sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
    let block_align: u16 = num_channels * (bits_per_sample / 8);
    let data_bytes_len: u32 = (samples.len() * 2) as u32;
    let riff_chunk_size: u32 = 36 + data_bytes_len;

    let mut out = Vec::with_capacity(44 + data_bytes_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_chunk_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");

    // fmt subchunk
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size = 16 for PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat = 1 (PCM)
    out.extend_from_slice(&num_channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data subchunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes_len.to_le_bytes());

    for &s in samples {
        let clamped = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&clamped.to_le_bytes());
    }

    out
}
