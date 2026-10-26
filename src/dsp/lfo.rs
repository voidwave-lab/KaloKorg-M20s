/// Modulation Generator (MG / LFO)
/// Matches the MS-20 continuous waveform skew circuit:
/// - Left output jack (`/\` Triangle/Saw): variable slope from Ramp-Down (0) -> Triangle (5) -> Ramp-Up (10), -1.0..+1.0
/// - Right output jack (`|_|` Pulse/Square): variable duty pulse (0.0..+1.0 or bipolar -1.0..+1.0 for S&H clock & trigs)
#[derive(Clone, Debug, Default)]
pub struct ModulationGenerator {
    pub phase: f32,
    pub last_tri_out: f32,
    pub last_pulse_out: f32,
}

impl ModulationGenerator {
    #[inline]
    pub fn process(&mut self, waveform_knob: f32, freq_knob: f32, sample_rate: f32) -> (f32, f32) {
        // Frequency knob 0..10 maps exponentially from 0.1 Hz to 32.0 Hz
        let norm_f = (freq_knob.clamp(0.0, 10.0) / 10.0).powf(2.1);
        let freq_hz = 0.1 + norm_f * 31.9;
        let dt = freq_hz / sample_rate;

        // Skew ratio (symmetry midpoint p in [0.04, 0.96])
        let p = (0.04 + (waveform_knob.clamp(0.0, 10.0) / 10.0) * 0.92).clamp(0.04, 0.96);

        let tri_out = if self.phase < p {
            -1.0 + 2.0 * (self.phase / p)
        } else {
            1.0 - 2.0 * ((self.phase - p) / (1.0 - p))
        };

        let pulse_out = if self.phase < p { 1.0 } else { -1.0 };

        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        self.last_tri_out = tri_out;
        self.last_pulse_out = pulse_out;
        (tri_out, pulse_out)
    }
}
