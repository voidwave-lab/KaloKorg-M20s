use std::f32::consts::PI;
use crate::params::FilterRevision;

/// Single KORG35 / LM13600 2nd-Order (12 dB/oct) Sallen-Key TPT Zero-Delay Feedback Filter Cell
#[derive(Clone, Debug, Default)]
pub struct Korg35Cell {
    s1: f32,
    s2: f32,
    dc_block_x: f32,
    dc_block_y: f32,
}

impl Korg35Cell {
    /// Nonlinear saturation function modeling the KORG35 (IC1/IC2 on KLM-128) vs LM13600 OTA (KLM-307/3162).
    /// - Rev. 1 (KORG35): Single-ended PNP emitter follower (2SA733) + 1SS355 diode feedback clipper
    ///   producing asymmetric odd + even harmonic distortion when Peak approaches 10.
    /// - Rev. 2 (LM13600/NJM13700): Linearizing input diodes + Darlington buffer producing cleaner,
    ///   symmetric tanh saturation.
    #[inline]
    fn saturate(v: f32, rev: FilterRevision) -> f32 {
        match rev {
            FilterRevision::Rev1Korg35 => {
                // Asymmetric diode/transistor junction clipper (2SC945 / 2SA733 + 1SS355 pair)
                if v >= 0.0 {
                    (v * 1.35).tanh()
                } else {
                    // Harder negative knee from single-ended PNP emitter follower bias
                    (v * 1.85).tanh() * 0.86 + 0.03 * (v * v).min(1.0)
                }
            }
            FilterRevision::Rev2Ota => {
                // Symmetric OTA linearizing diode saturation
                v.tanh()
            }
        }
    }

    /// Process a single sample as a 12 dB/oct TPT Sallen-Key Low-Pass Filter.
    /// Uses Zero-Delay Feedback (ZDF) trapezoidal bilinear integration as specified in
    /// `Korg_MS20_DSP_Engineering_Specification.docx` Section 3.
    #[inline]
    pub fn process_lpf(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        peak_knob: f32, // 0.0 .. 10.0
        sample_rate: f32,
        rev: FilterRevision,
    ) -> f32 {
        let fc = cutoff_hz.clamp(16.0, sample_rate * 0.44);
        let wa = 2.0 * PI * fc;
        let g = (wa / (2.0 * sample_rate)).tan();
        let big_g = g / (1.0 + g);

        // Resonance feedback factor k: 0..10 maps to 0.0 .. 2.08 (self-oscillation above ~7.8)
        let norm_peak = (peak_knob / 10.0).clamp(0.0, 1.0);
        // Nonlinear taper matching the MS-20 100kB Peak pot + R190/R220 feedback network
        let k = (norm_peak * 1.45 + norm_peak * norm_peak * 0.63).min(2.08);

        // Estimate bandpass feedback state from trapezoidal integrators s1, s2
        let bp_est = (self.s1 - self.s2) / (1.0 + g);
        let fb_nl = Self::saturate(bp_est * (1.0 + 0.35 * norm_peak), rev);

        // Input drive into KORG35 differential pair (Pin 1)
        let x_in = Self::saturate(input * (1.0 + 0.25 * norm_peak), rev);

        // Zero-Delay Loop Resolution: u[n] = x[n] + k * f_sat(bp) - g_comp * y2_est
        let denom = (1.0 - k * big_g * (1.0 - big_g)).max(0.12);
        let u = (x_in + k * fb_nl) / denom.sqrt();

        // Trapezoidal Integrator 1
        let v1 = big_g * (u - self.s1);
        let y1 = v1 + self.s1;
        self.s1 = y1 + v1;

        // Trapezoidal Integrator 2 (with KORG35 inter-stage saturation)
        let y1_sat = Self::saturate(y1, rev);
        let v2 = big_g * (y1_sat - self.s2);
        let y2 = v2 + self.s2;
        self.s2 = y2 + v2;

        // Subtle DC blocker to remove asymmetric diode offset while preserving sub-bass
        let out = y2;
        let dc_out = out - self.dc_block_x + 0.9992 * self.dc_block_y;
        self.dc_block_x = out;
        self.dc_block_y = dc_out;

        dc_out.clamp(-2.5, 2.5)
    }

    /// Process a single sample as a 12 dB/oct TPT Sallen-Key High-Pass Filter.
    /// Uses the TPT Sallen-Key equations from `Korg_MS20_DSP_Engineering_Specification.docx`:
    /// `y_HP = u[n] - 2 * y1 + y2`
    #[inline]
    pub fn process_hpf(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        peak_knob: f32, // 0.0 .. 10.0
        sample_rate: f32,
        rev: FilterRevision,
    ) -> f32 {
        let fc = cutoff_hz.clamp(14.0, sample_rate * 0.44);
        let wa = 2.0 * PI * fc;
        let g = (wa / (2.0 * sample_rate)).tan();
        let big_g = g / (1.0 + g);

        let norm_peak = (peak_knob / 10.0).clamp(0.0, 1.0);
        let k = (norm_peak * 1.45 + norm_peak * norm_peak * 0.63).min(2.08);

        let bp_est = (self.s1 - self.s2) / (1.0 + g);
        let fb_nl = Self::saturate(bp_est * (1.0 + 0.35 * norm_peak), rev);

        let x_in = Self::saturate(input * (1.0 + 0.25 * norm_peak), rev);
        let denom = (1.0 - k * big_g * (1.0 - big_g)).max(0.12);
        let u = (x_in + k * fb_nl) / denom.sqrt();

        // State updates:
        // v1 = G * (u[n] - s1), y1 = v1 + s1, s1 <- y1 + v1
        // v2 = G * (y1 - s2),   y2 = v2 + s2, s2 <- y2 + v2
        let v1 = big_g * (u - self.s1);
        let y1 = v1 + self.s1;
        self.s1 = y1 + v1;

        let y1_sat = Self::saturate(y1, rev);
        let v2 = big_g * (y1_sat - self.s2);
        let y2 = v2 + self.s2;
        self.s2 = y2 + v2;

        // HPF Output Node per specification: y_HP = u[n] - 2 * y1 + y2
        let y_hp = u - 2.0 * y1 + y2;

        let dc_out = y_hp - self.dc_block_x + 0.9992 * self.dc_block_y;
        self.dc_block_x = y_hp;
        self.dc_block_y = dc_out;

        dc_out.clamp(-2.5, 2.5)
    }
}

/// Dual KORG35 Series Filter (12dB/oct VC HPF -> 12dB/oct VC LPF) on KLM-128 / KLM-3162 Board
#[derive(Clone, Debug, Default)]
pub struct DualKorg35Filter {
    pub hpf: Korg35Cell,
    pub lpf: Korg35Cell,
}

impl DualKorg35Filter {
    /// Convert 0..10 MS-20 HPF Cutoff knob + CV octaves to Hz (16 Hz .. 15,000 Hz)
    #[inline]
    pub fn hpf_knob_to_hz(knob: f32, mod_octaves: f32) -> f32 {
        // At 0.0 -> ~16 Hz (sub-audible, passes full bass); at 10.0 -> ~12 kHz
        let base_oct = (knob.clamp(0.0, 10.0) / 10.0) * 9.6;
        16.0 * 2.0_f32.powf((base_oct + mod_octaves).clamp(0.0, 10.2))
    }

    /// Convert 0..10 MS-20 LPF Cutoff knob + CV octaves to Hz (22 Hz .. 18,500 Hz)
    #[inline]
    pub fn lpf_knob_to_hz(knob: f32, mod_octaves: f32) -> f32 {
        let base_oct = (knob.clamp(0.0, 10.0) / 10.0) * 9.7;
        22.0 * 2.0_f32.powf((base_oct + mod_octaves).clamp(0.0, 10.2))
    }

    #[inline]
    pub fn process(
        &mut self,
        input: f32,
        hpf_cutoff_hz: f32,
        hpf_peak: f32,
        lpf_cutoff_hz: f32,
        lpf_peak: f32,
        sample_rate: f32,
        rev: FilterRevision,
    ) -> f32 {
        // KORG35 IC1: Voltage-Controlled High-Pass Filter
        let hp_out = self.hpf.process_hpf(input, hpf_cutoff_hz, hpf_peak, sample_rate, rev);
        // KORG35 IC2: Voltage-Controlled Low-Pass Filter in series after HPF
        self.lpf.process_lpf(hp_out, lpf_cutoff_hz, lpf_peak, sample_rate, rev)
    }
}
