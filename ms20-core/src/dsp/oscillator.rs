use crate::params::{Vco1Scale, Vco1Waveform, Vco2Scale, Vco2Waveform};

/// Polynomial Band-Limited Step (PolyBLEP) residual function as specified in
/// `Korg_MS20_DSP_Engineering_Specification.docx` Section 2.
#[inline]
pub fn poly_blep(mut t: f32, dt: f32) -> f32 {
    if dt <= 1e-7 {
        return 0.0;
    }
    if t < dt {
        t /= dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// Fast deterministic PRNG (xorshift64*) for zero-allocation audio-thread White & Pink Noise
#[derive(Clone, Debug)]
pub struct NoiseGenerator {
    state: u64,
    // Paul Kellet 1/f pink noise filter state
    b0: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
    b6: f32,
    pub last_white: f32,
    pub last_pink: f32,
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        Self {
            state: 0x853C_49E6_748F_EA9B,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            b3: 0.0,
            b4: 0.0,
            b5: 0.0,
            b6: 0.0,
            last_white: 0.0,
            last_pink: 0.0,
        }
    }
}

impl NoiseGenerator {
    #[inline]
    pub fn next_white(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let normalized = ((r >> 40) as f32) * (2.0 / 16777216.0) - 1.0;
        self.last_white = normalized;
        normalized
    }

    #[inline]
    pub fn step(&mut self) -> (f32, f32) {
        let white = self.next_white();
        self.b0 = 0.99886 * self.b0 + white * 0.0555179;
        self.b1 = 0.99332 * self.b1 + white * 0.0750759;
        self.b2 = 0.96900 * self.b2 + white * 0.1538520;
        self.b3 = 0.86650 * self.b3 + white * 0.3104856;
        self.b4 = 0.55000 * self.b4 + white * 0.5329522;
        self.b5 = -0.7616 * self.b5 - white * 0.0168980;
        let pink = (self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + white * 0.5362) * 0.18;
        self.b6 = white * 0.115926;
        self.last_pink = pink.clamp(-1.0, 1.0);
        (white, self.last_pink)
    }
}

/// Dual VCO Section (VCO 1 & VCO 2) + TC4011BP CMOS NAND/XOR Ring Modulator
#[derive(Clone, Debug)]
pub struct DualVco {
    pub phase1: f32,
    pub phase2: f32,
    tri_integrator: f32,
    drift1: f32,
    drift2: f32,
}

impl Default for DualVco {
    fn default() -> Self {
        Self {
            phase1: 0.0,
            phase2: 0.27, // Natural unlocked analog phase offset
            tri_integrator: 0.0,
            drift1: 0.0,
            drift2: 0.0,
        }
    }
}

impl DualVco {
    /// Compute one sample of VCO 1 and VCO 2 given base frequencies and parameters.
    /// Returns `(vco1_out, vco2_out)`.
    #[inline]
    pub fn process(
        &mut self,
        sample_rate: f32,
        base_freq_vco1: f32,
        base_freq_vco2: f32,
        vco1_wave: Vco1Waveform,
        vco1_pw_knob: f32, // 0..10
        vco1_scale: Vco1Scale,
        vco2_wave: Vco2Waveform,
        vco2_pitch_knob: f32, // -5..+5
        vco2_scale: Vco2Scale,
        master_tune_knob: f32, // -5..+5
        white_noise: f32,
    ) -> (f32, f32) {
        // Subtle analog VCO thermal drift (~ +/- 0.8 cents slow wander)
        self.drift1 = 0.99995 * self.drift1 + 0.00005 * white_noise * 0.0012;
        self.drift2 = 0.99995 * self.drift2 - 0.00005 * white_noise * 0.0014;

        // Master Tune: -5..+5 -> -100..+100 cents (+/- 1 semitone)
        let master_ratio = 2.0_f32.powf((master_tune_knob / 5.0) / 12.0);

        // VCO 1 frequency
        let f1 = (base_freq_vco1 * vco1_scale.octave_multiplier() * master_ratio * (1.0 + self.drift1))
            .clamp(10.0, sample_rate * 0.47);
        let dt1 = f1 / sample_rate;

        // VCO 2 pitch offset: -5..+5 knob maps to -12..+12 semitones (1 full octave down/up)
        let vco2_semitones = (vco2_pitch_knob / 5.0) * 12.0;
        let vco2_ratio = 2.0_f32.powf(vco2_semitones / 12.0);
        let f2 = (base_freq_vco2 * vco2_scale.octave_multiplier() * vco2_ratio * master_ratio * (1.0 + self.drift2))
            .clamp(10.0, sample_rate * 0.47);
        let dt2 = f2 / sample_rate;

        // VCO 1 Pulse Width (0..10 maps from 0.50 square to 0.92 narrow pulse)
        let pw1 = (0.5 + (vco1_pw_knob.clamp(0.0, 10.0) / 10.0) * 0.43).clamp(0.05, 0.95);

        // VCO 1 Square/Pulse (always computed because TC4011BP Ring Mod uses VCO 1 Square & VCO 2 Pulse!)
        let mut pulse1 = if self.phase1 < pw1 { 1.0 } else { -1.0 };
        pulse1 += poly_blep(self.phase1, dt1);
        pulse1 -= poly_blep((self.phase1 - pw1 + 1.0).fract(), dt1);

        // VCO 1 selected waveform
        let vco1_out = match vco1_wave {
            Vco1Waveform::Triangle => {
                // Direct band-limited triangle via leaky integration of 50% PolyBLEP square
                let mut sq50 = if self.phase1 < 0.5 { 1.0 } else { -1.0 };
                sq50 += poly_blep(self.phase1, dt1);
                sq50 -= poly_blep((self.phase1 - 0.5 + 1.0).fract(), dt1);
                self.tri_integrator = (1.0 - 2.0 * dt1) * self.tri_integrator + 4.0 * dt1 * sq50;
                // Analytic fallback blended for low-frequency stability
                let naive_tri = 1.0 - 4.0 * (self.phase1 - 0.5).abs();
                (0.75 * self.tri_integrator + 0.25 * naive_tri).clamp(-1.1, 1.1)
            }
            Vco1Waveform::Sawtooth => {
                let mut saw = 2.0 * self.phase1 - 1.0;
                saw -= poly_blep(self.phase1, dt1);
                saw
            }
            Vco1Waveform::Pulse => {
                // DC-compensated PWM
                pulse1 - (2.0 * pw1 - 1.0) * 0.5
            }
            Vco1Waveform::WhiteNoise => white_noise,
        };

        // VCO 2 pulse wave for Square / NarrowPulse / TC4011BP Ring Modulator
        let pw2 = match vco2_wave {
            Vco2Waveform::NarrowPulse => 0.18,
            _ => 0.50,
        };
        let mut pulse2 = if self.phase2 < pw2 { 1.0 } else { -1.0 };
        pulse2 += poly_blep(self.phase2, dt2);
        pulse2 -= poly_blep((self.phase2 - pw2 + 1.0).fract(), dt2);

        let vco2_out = match vco2_wave {
            Vco2Waveform::Sawtooth => {
                let mut saw2 = 2.0 * self.phase2 - 1.0;
                saw2 -= poly_blep(self.phase2, dt2);
                saw2
            }
            Vco2Waveform::Square | Vco2Waveform::NarrowPulse => pulse2 - (2.0 * pw2 - 1.0) * 0.5,
            Vco2Waveform::RingMod => {
                // Authentic Korg MS-20 TC4011BP Quad 2-Input CMOS NAND Gate XOR Ring Modulator:
                // Per `korn-ms20-internal-circuitry.md` Section 4 & `Korg_MS20_DSP_Engineering_Specification.docx` Section 2:
                // RingOut = sgn(VCO1_Pulse) * sgn(VCO2_Pulse), smoothed across PolyBLEP edges
                let logic1 = (pulse1 * 4.5).tanh();
                let logic2 = (pulse2 * 4.5).tanh();
                logic1 * logic2
            }
        };

        // Advance phase accumulators
        self.phase1 += dt1;
        if self.phase1 >= 1.0 {
            self.phase1 -= 1.0;
        }
        self.phase2 += dt2;
        if self.phase2 >= 1.0 {
            self.phase2 -= 1.0;
        }

        (vco1_out, vco2_out)
    }
}
