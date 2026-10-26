/// Monophonic voice allocator & portamento glide controller for KaloKorg M20s.
/// Respects the MS-20 monophonic note stack and RC portamento circuit (VR4 100kB + 2SK30A FET).
#[derive(Clone, Debug)]
pub struct MonoVoice {
    held_notes: [u8; 16],
    held_count: usize,
    pub current_note: u8,
    pub target_freq_hz: f32,
    pub glided_freq_hz: f32,
    pub gate: bool,
    pub velocity: f32,
    pub pitch_bend_semitones: f32,
}

impl Default for MonoVoice {
    fn default() -> Self {
        let base_f = midi_note_to_hz(60); // C4 = 261.63 Hz
        Self {
            held_notes: [0; 16],
            held_count: 0,
            current_note: 60,
            target_freq_hz: base_f,
            glided_freq_hz: base_f,
            gate: false,
            velocity: 1.0,
            pitch_bend_semitones: 0.0,
        }
    }
}

#[inline]
pub fn midi_note_to_hz(note: u8) -> f32 {
    440.0 * 2.0_f32.powf(((note as f32) - 69.0) / 12.0)
}

impl MonoVoice {
    pub fn note_on(&mut self, note: u8, velocity: u8) {
        if velocity == 0 {
            self.note_off(note);
            return;
        }
        // Remove if already present
        self.held_notes_remove(note);
        if self.held_count < self.held_notes.len() {
            self.held_notes[self.held_count] = note;
            self.held_count += 1;
        } else {
            // Shift left and append
            for i in 0..(self.held_notes.len() - 1) {
                self.held_notes[i] = self.held_notes[i + 1];
            }
            self.held_notes[self.held_notes.len() - 1] = note;
        }

        self.current_note = note;
        self.target_freq_hz = midi_note_to_hz(note);
        self.velocity = (velocity as f32 / 127.0).clamp(0.1, 1.0);
        self.gate = true;
    }

    pub fn note_off(&mut self, note: u8) {
        self.held_notes_remove(note);
        if self.held_count > 0 {
            let active = self.held_notes[self.held_count - 1];
            self.current_note = active;
            self.target_freq_hz = midi_note_to_hz(active);
            self.gate = true;
        } else {
            self.gate = false;
        }
    }

    pub fn all_notes_off(&mut self) {
        self.held_count = 0;
        self.gate = false;
    }

    fn held_notes_remove(&mut self, note: u8) {
        let mut i = 0;
        while i < self.held_count {
            if self.held_notes[i] == note {
                for j in i..(self.held_count - 1) {
                    self.held_notes[j] = self.held_notes[j + 1];
                }
                self.held_count -= 1;
            } else {
                i += 1;
            }
        }
    }

    /// Step portamento glide and return `(glided_freq_hz, key_cv_normalized)`
    #[inline]
    pub fn step_portamento(&mut self, portamento_knob: f32, sample_rate: f32) -> (f32, f32) {
        let bend_ratio = 2.0_f32.powf(self.pitch_bend_semitones / 12.0);
        let dest_hz = self.target_freq_hz * bend_ratio;

        if portamento_knob <= 0.05 {
            self.glided_freq_hz = dest_hz;
        } else {
            let tau = 0.002 + (portamento_knob.clamp(0.0, 10.0) / 10.0).powf(2.2) * 1.25;
            let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
            self.glided_freq_hz += alpha * (dest_hz - self.glided_freq_hz);
        }

        // Normalized pitch CV around C4 (261.63 Hz): 1 octave = 0.25 normalized units
        let key_cv = (self.glided_freq_hz / 261.62558).log2() * 0.25;
        (self.glided_freq_hz, key_cv)
    }
}
