use std::f32::consts::PI;
use crate::params::{
    BossCe2wParams, BossDd8Mode, BossDd8Params, ChorusMode, CloudburstEnsembleMode,
    CloudburstParams, Particle2Params, ParticleMode, PedalId, PedalboardParams, ZoomMode,
    ZoomMs70Params,
};

/// 100% bounds-safe linear interpolation into a circular delay buffer.
/// Uses `f64` for `rem_euclid` to avoid `f32` negative-epsilon rounding to `len`,
/// and applies `% len` to both `idx0` and `idx1` so out-of-bounds indexing is impossible.
#[inline]
fn lerp_buf(buf: &[f32], write_pos: usize, delay_samples: f32) -> f32 {
    let len = buf.len();
    if len == 0 {
        return 0.0;
    }
    let safe_delay = if delay_samples.is_finite() {
        delay_samples.max(0.0) as f64
    } else {
        0.0
    };
    let read_f = (write_pos as f64) - safe_delay;
    let read_wrapped = read_f.rem_euclid(len as f64);
    let idx0 = (read_wrapped as usize) % len;
    let idx1 = (idx0 + 1) % len;
    let frac = (read_wrapped - (read_wrapped.floor())) as f32;
    buf[idx0] * (1.0 - frac) + buf[idx1] * frac
}

// ============================================================================
// 1. ZOOM MS-70CDR (MultiStomp Chorus / Delay / Reverb)
// ============================================================================
#[derive(Clone)]
pub struct ZoomMs70Dsp {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write_pos: usize,
    lfo_phase: f32,
    shimmer_phase1: f32,
    shimmer_phase2: f32,
    lp_l: f32,
    lp_r: f32,
    phaser_ap_l: [f32; 6],
    phaser_ap_r: [f32; 6],
}

impl ZoomMs70Dsp {
    pub fn new(max_sr: usize) -> Self {
        let len = (max_sr * 2).max(48000); // 2 seconds stereo buffer
        Self {
            buf_l: vec![0.0; len],
            buf_r: vec![0.0; len],
            write_pos: 0,
            lfo_phase: 0.0,
            shimmer_phase1: 0.0,
            shimmer_phase2: 0.0,
            lp_l: 0.0,
            lp_r: 0.0,
            phaser_ap_l: [0.0; 6],
            phaser_ap_r: [0.0; 6],
        }
    }

    #[inline]
    pub fn process(&mut self, in_l: f32, in_r: f32, p: &ZoomMs70Params, sr: f32) -> (f32, f32) {
        if !p.enabled {
            return (in_l, in_r);
        }

        let len = self.buf_l.len();
        let depth = (p.depth / 10.0).clamp(0.0, 1.0);
        let rate_norm = (p.rate / 10.0).clamp(0.0, 1.0);
        let rate_hz = 0.15 + rate_norm.powf(1.8) * 7.5;
        let mix = (p.mix / 10.0).clamp(0.0, 1.0);
        let tone_alpha = 0.18 + (p.tone / 10.0).clamp(0.0, 1.0) * 0.80;

        self.lfo_phase = (self.lfo_phase + rate_hz / sr).rem_euclid(1.0);
        let s1 = (2.0 * PI * self.lfo_phase).sin();
        let s2 = (2.0 * PI * (self.lfo_phase + 0.3333)).sin();
        let s3 = (2.0 * PI * (self.lfo_phase + 0.6667)).sin();

        let (wet_l, wet_r, fb_l, fb_r) = match p.mode {
            ZoomMode::CoronaTriChorus => {
                // Wide Stereo Dimension Tri-Chorus + Pitch Detune
                let base_del = 0.012 * sr;
                let mod_span = (0.0025 + depth * 0.014) * sr;
                let d1 = base_del + s1 * mod_span;
                let d2 = base_del * 1.35 + s2 * mod_span;
                let d3 = base_del * 0.75 + s3 * mod_span;

                let c1_l = lerp_buf(&self.buf_l, self.write_pos, d1);
                let c2_l = lerp_buf(&self.buf_l, self.write_pos, d2);
                let c3_l = lerp_buf(&self.buf_r, self.write_pos, d3);

                // Opposite phase modulation in Right channel for massive stereo width
                let c1_r = lerp_buf(&self.buf_r, self.write_pos, base_del - s1 * mod_span);
                let c2_r = lerp_buf(&self.buf_r, self.write_pos, base_del * 1.35 - s2 * mod_span);
                let c3_r = lerp_buf(&self.buf_l, self.write_pos, base_del * 0.75 - s3 * mod_span);

                let wl = (c1_l + c2_l * 0.9 - c3_l * 0.7) * 0.75;
                let wr = (c1_r + c2_r * 0.9 - c3_r * 0.7) * 0.75;
                let fb = 0.25 + depth * 0.45;
                (
                    wl,
                    wr,
                    (in_l + wl * fb).tanh(),
                    (in_r + wr * fb).tanh(),
                )
            }
            ZoomMode::IceShimmerDelay => {
                // Dual-Voice +12st (Octave Up) & +24st (2 Octaves Up) Crystal Ice Shimmer
                let grain_len = 0.055 * sr;
                self.shimmer_phase1 = (self.shimmer_phase1 + 1.0 / grain_len).rem_euclid(1.0);
                self.shimmer_phase2 = (self.shimmer_phase2 + 2.0 / grain_len).rem_euclid(1.0);

                let ph_a = self.shimmer_phase1;
                let ph_b = (self.shimmer_phase1 + 0.5).rem_euclid(1.0);
                let win_a = (PI * ph_a).sin().powi(2);
                let win_b = (PI * ph_b).sin().powi(2);

                // +12st reads 2x faster -> delay shrinks from grain_len to 0
                let pre_del = (0.015 + rate_norm * 0.18) * sr;
                let d_a = pre_del + (1.0 - ph_a) * grain_len;
                let d_b = pre_del + (1.0 - ph_b) * grain_len;

                let oct1_l = lerp_buf(&self.buf_l, self.write_pos, d_a) * win_a
                    + lerp_buf(&self.buf_l, self.write_pos, d_b) * win_b;
                let oct1_r = lerp_buf(&self.buf_r, self.write_pos, d_a + 35.0) * win_a
                    + lerp_buf(&self.buf_r, self.write_pos, d_b + 35.0) * win_b;

                // +24st crystal upper harmonic layer
                let ph2_a = self.shimmer_phase2;
                let ph2_b = (self.shimmer_phase2 + 0.5).rem_euclid(1.0);
                let w2_a = (PI * ph2_a).sin().powi(2);
                let w2_b = (PI * ph2_b).sin().powi(2);
                let d2_a = pre_del * 0.5 + (1.0 - ph2_a) * grain_len;
                let d2_b = pre_del * 0.5 + (1.0 - ph2_b) * grain_len;
                let oct2_l = lerp_buf(&self.buf_l, self.write_pos, d2_a) * w2_a
                    + lerp_buf(&self.buf_l, self.write_pos, d2_b) * w2_b;
                let oct2_r = lerp_buf(&self.buf_r, self.write_pos, d2_a + 50.0) * w2_a
                    + lerp_buf(&self.buf_r, self.write_pos, d2_b + 50.0) * w2_b;

                let wl = (oct1_l * 1.15 + oct2_l * depth * 0.75) * (0.6 + depth * 0.7);
                let wr = (oct1_r * 1.15 + oct2_r * depth * 0.75) * (0.6 + depth * 0.7);
                let fb = 0.55 + depth * 0.36;
                (
                    wl,
                    wr,
                    (in_l + wr * fb).tanh(),
                    (in_r + wl * fb).tanh(),
                )
            }
            ZoomMode::SpaceHoleReverb => {
                // Deep 6-stage Stereo All-Pass Phaser + Black-Hole Modulated Nebula Wash
                let ap_coeff = (0.15 + 0.70 * (0.5 + 0.5 * s1)).clamp(-0.9, 0.9);
                let mut pl = in_l;
                let mut pr = in_r;
                for stage in 0..6 {
                    let y_l = -ap_coeff * pl + self.phaser_ap_l[stage];
                    self.phaser_ap_l[stage] = pl + ap_coeff * y_l;
                    pl = y_l;

                    let y_r = ap_coeff * pr + self.phaser_ap_r[stage];
                    self.phaser_ap_r[stage] = pr - ap_coeff * y_r;
                    pr = y_r;
                }

                let t1 = (0.037 * sr) + s1 * 120.0;
                let t2 = (0.089 * sr) + s2 * 160.0;
                let t3 = (0.157 * sr) + s3 * 210.0;
                let t4 = (0.251 * sr) - s1 * 180.0;

                let l1 = lerp_buf(&self.buf_l, self.write_pos, t1);
                let l2 = lerp_buf(&self.buf_r, self.write_pos, t2);
                let l3 = lerp_buf(&self.buf_l, self.write_pos, t3);
                let l4 = lerp_buf(&self.buf_r, self.write_pos, t4);

                let wl = pl * 0.65 + (l1 + l2 - l3 + l4) * 0.55;
                let wr = pr * 0.65 + (l2 - l1 + l4 + l3) * 0.55;
                let fb = (0.62 + depth * 0.33).min(0.95);
                (
                    wl,
                    wr,
                    (in_l + pl * 0.5 + wr * fb).tanh(),
                    (in_r + pr * 0.5 - wl * fb).tanh(),
                )
            }
            ZoomMode::LoFiTapeVibrato => {
                // Heavy Boards-of-Canada Warbling Tape Pitch Drift + Overdrive Saturation
                let wow_l = (0.004 + depth * 0.022 * (0.5 + 0.5 * s1) + 0.004 * s3) * sr;
                let wow_r = (0.005 + depth * 0.022 * (0.5 + 0.5 * s2) + 0.004 * s1) * sr;
                let raw_l = lerp_buf(&self.buf_l, self.write_pos, wow_l);
                let raw_r = lerp_buf(&self.buf_r, self.write_pos, wow_r);
                let drive = 1.6 + depth * 3.2;
                let wl = (raw_l * drive).tanh() * 0.9;
                let wr = (raw_r * drive).tanh() * 0.9;
                (wl, wr, in_l, in_r)
            }
        };

        self.lp_l += tone_alpha * (wet_l - self.lp_l);
        self.lp_r += tone_alpha * (wet_r - self.lp_r);

        let wp = self.write_pos % len;
        self.buf_l[wp] = fb_l;
        self.buf_r[wp] = fb_r;
        self.write_pos = (wp + 1) % len;

        // Equal-power wet/dry blend so turning Mix up has immediate dramatic presence
        let dry_gain = (1.0 - mix * 0.65).max(0.0);
        let wet_gain = (mix * 1.35).min(1.35);
        let out_l = in_l * dry_gain + self.lp_l * wet_gain;
        let out_r = in_r * dry_gain + self.lp_r * wet_gain;
        (out_l, out_r)
    }
}

// ============================================================================
// 2. BOSS DD-8 DIGITAL DELAY (Warm, Reverse, Warp, Glitch, Tape)
// ============================================================================
#[derive(Clone)]
pub struct BossDd8Dsp {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write_pos: usize,
    rev_phase: f32,
    glitch_pos: usize,
    lfo_phase: f32,
    damp_l: f32,
    damp_r: f32,
}

impl BossDd8Dsp {
    pub fn new(max_sr: usize) -> Self {
        let len = (max_sr * 3).max(96000); // 3 seconds stereo delay buffer
        Self {
            buf_l: vec![0.0; len],
            buf_r: vec![0.0; len],
            write_pos: 0,
            rev_phase: 0.0,
            glitch_pos: 0,
            lfo_phase: 0.0,
            damp_l: 0.0,
            damp_r: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, in_l: f32, in_r: f32, p: &BossDd8Params, sr: f32) -> (f32, f32) {
        if !p.enabled {
            return (in_l, in_r);
        }

        let len = self.buf_l.len();
        let level = (p.effect_level / 10.0).clamp(0.0, 1.0);
        let fb_norm = (p.feedback / 10.0).clamp(0.0, 1.0);
        let delay_sec = 0.045 + (p.delay_time / 10.0).clamp(0.0, 1.0).powf(1.6) * 1.25;
        let delay_samples = (delay_sec * sr).clamp(64.0, (len - 128) as f32);
        let mod_depth = (p.mod_depth / 10.0).clamp(0.0, 1.0);

        self.lfo_phase = (self.lfo_phase + 1.25 / sr).rem_euclid(1.0);
        let lfo_s = (2.0 * PI * self.lfo_phase).sin();
        let lfo_c = (2.0 * PI * self.lfo_phase).cos();

        let (wet_l, wet_r, fb_gain, damp_coeff) = match p.mode {
            BossDd8Mode::Warm => {
                // Wide Stereo Dotted-Eighth (3:4 ratio) Warm Analog Echo
                let dl = delay_samples + lfo_s * mod_depth * 65.0;
                let dr = delay_samples * 0.75 + lfo_c * mod_depth * 65.0;
                (
                    lerp_buf(&self.buf_l, self.write_pos, dl),
                    lerp_buf(&self.buf_r, self.write_pos, dr),
                    0.25 + fb_norm * 0.72,
                    0.35,
                )
            }
            BossDd8Mode::Reverse => {
                // Dramatic Backwards Envelope-Swelled Stereo Delay
                let grain_win_samples = (delay_samples * 0.85).max(1024.0);
                self.rev_phase = (self.rev_phase + 1.0 / grain_win_samples).rem_euclid(1.0);
                let ph2 = (self.rev_phase + 0.5).rem_euclid(1.0);

                // Reading backwards at -1x speed means delay increases by 2.0 samples per sample
                let rev_d1 = (self.rev_phase * grain_win_samples * 2.0).clamp(8.0, (len - 16) as f32);
                let rev_d2 = (ph2 * grain_win_samples * 2.0).clamp(8.0, (len - 16) as f32);

                // Asymmetric exponential swell window (slow fade-in, sharp cut like backwards tape)
                let win1 = self.rev_phase.powf(2.2) * (PI * self.rev_phase).sin();
                let win2 = ph2.powf(2.2) * (PI * ph2).sin();

                let rl = lerp_buf(&self.buf_l, self.write_pos, rev_d1) * win1 * 1.45
                    + lerp_buf(&self.buf_r, self.write_pos, rev_d2) * win2 * 0.85;
                let rr = lerp_buf(&self.buf_r, self.write_pos, rev_d2) * win2 * 1.45
                    + lerp_buf(&self.buf_l, self.write_pos, rev_d1) * win1 * 0.85;
                (rl, rr, 0.25 + fb_norm * 0.68, 0.75)
            }
            BossDd8Mode::Warp => {
                // Expressive Self-Oscillating Pitch-Swept Feedback Swell
                let sweep = lfo_s * (45.0 + mod_depth * 180.0);
                let dl = (delay_samples * 0.65 + sweep).clamp(32.0, (len - 64) as f32);
                let dr = (delay_samples * 0.85 - sweep).clamp(32.0, (len - 64) as f32);
                let wl = lerp_buf(&self.buf_l, self.write_pos, dl) * 1.25;
                let wr = lerp_buf(&self.buf_r, self.write_pos, dr) * 1.25;
                (wl, wr, (0.65 + fb_norm * 0.45).min(1.08), 0.55)
            }
            BossDd8Mode::Glitch => {
                // Hard Machine-Gun Rhythmic Stutter Repeater + Octave Jump
                let slice_samples = ((0.022 + (p.delay_time / 10.0) * 0.11) * sr) as usize;
                let slice_len = slice_samples.clamp(384, 12000);
                self.glitch_pos = (self.glitch_pos + 1) % (slice_len * 2);

                // Alternate between 1x speed stutter and 1.5x/2x pitch-stepped stutter
                let rate_mult = if self.glitch_pos < slice_len { 0.5 } else { 1.5 };
                let d_glitch = ((self.glitch_pos % slice_len) as f32 * rate_mult).max(16.0);
                // Hard rhythmic gate envelope on the glitch slice
                let gate_env = if (self.glitch_pos % slice_len) < (slice_len * 4 / 5) {
                    1.25
                } else {
                    0.0
                };
                let gl = lerp_buf(&self.buf_l, self.write_pos, d_glitch) * gate_env;
                let gr = lerp_buf(&self.buf_r, self.write_pos, d_glitch + 48.0) * gate_env;
                (gl, gr, fb_norm * 0.85, 0.90)
            }
            BossDd8Mode::Tape => {
                // Multi-Head Roland RE-201 Space Echo (3 Tape Heads panned L/C/R + Wow/Flutter)
                let wow = lfo_s * (25.0 + mod_depth * 110.0) + lfo_c * 18.0;
                let h1 = (delay_samples * 0.38 + wow).clamp(16.0, (len - 32) as f32);
                let h2 = (delay_samples * 0.66 - wow * 0.8).clamp(16.0, (len - 32) as f32);
                let h3 = (delay_samples + wow * 1.2).clamp(16.0, (len - 32) as f32);

                let t1_l = lerp_buf(&self.buf_l, self.write_pos, h1);
                let t2_r = lerp_buf(&self.buf_r, self.write_pos, h2);
                let t3_c = lerp_buf(&self.buf_l, self.write_pos, h3);

                let tl = ((t1_l * 0.9 + t3_c * 0.7) * 1.35).tanh();
                let tr = ((t2_r * 0.9 + t3_c * 0.7) * 1.35).tanh();
                (tl, tr, 0.30 + fb_norm * 0.68, 0.28)
            }
        };

        self.damp_l += damp_coeff * (wet_l - self.damp_l);
        self.damp_r += damp_coeff * (wet_r - self.damp_r);

        // Ping-pong cross-coupled stereo feedback so echoes bounce visibly L <-> R
        let next_l = (in_l + (self.damp_r * 0.75 + self.damp_l * 0.25) * fb_gain).tanh();
        let next_r = (in_r + (self.damp_l * 0.75 + self.damp_r * 0.25) * fb_gain).tanh();

        let wp = self.write_pos % len;
        self.buf_l[wp] = next_l;
        self.buf_r[wp] = next_r;
        self.write_pos = (wp + 1) % len;

        // In Reverse and Glitch modes, duck the dry signal proportionally so the effect is front-and-center
        let dry_duck = match p.mode {
            BossDd8Mode::Reverse | BossDd8Mode::Glitch => (1.0 - level * 0.75).max(0.15),
            _ => (1.0 - level * 0.35).max(0.45),
        };
        let wet_boost = (level * 1.35).min(1.4);

        (
            in_l * dry_duck + self.damp_l * wet_boost,
            in_r * dry_duck + self.damp_r * wet_boost,
        )
    }
}

// ============================================================================
// 3. RED PANDA PARTICLE 2 (8-Grain Granular Delay & Pitch Shifter with FREEZE)
// ============================================================================
#[derive(Clone, Copy, Default)]
struct Grain {
    active: bool,
    start_delay: f32,
    phase: f32,
    phase_inc: f32,
    pitch_ratio: f32,
    reverse: bool,
    pan_l: f32,
    pan_r: f32,
}

#[derive(Clone)]
pub struct RedPandaParticle2Dsp {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write_pos: usize,
    grains: [Grain; 8],
    spawn_counter: u32,
    rng_state: u64,
    lfo_phase: f32,
}

impl RedPandaParticle2Dsp {
    pub fn new(max_sr: usize) -> Self {
        let len = (max_sr * 3).max(96000); // 3 seconds granular buffer
        Self {
            buf_l: vec![0.0; len],
            buf_r: vec![0.0; len],
            write_pos: 0,
            grains: [Grain::default(); 8],
            spawn_counter: 0,
            rng_state: 0xC0FF_EE12_3456_789A,
            lfo_phase: 0.0,
        }
    }

    #[inline]
    fn next_rand(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32) / 16777216.0
    }

    #[inline]
    pub fn process(&mut self, in_l: f32, in_r: f32, p: &Particle2Params, sr: f32) -> (f32, f32) {
        if !p.enabled {
            return (in_l, in_r);
        }

        let len = self.buf_l.len();
        let blend = (p.blend / 10.0).clamp(0.0, 1.0);
        let chop_norm = (p.chop / 10.0).clamp(0.0, 1.0);
        let grain_dur_sec = 0.022 + (p.grain_size / 10.0).clamp(0.0, 1.0) * 0.28;
        let grain_samples = (grain_dur_sec * sr).max(256.0);
        let fb = (p.feedback / 10.0).clamp(0.0, 0.94);

        self.lfo_phase = (self.lfo_phase + 0.65 / sr).rem_euclid(1.0);
        let lfo_val = 0.5 + 0.5 * (2.0 * PI * self.lfo_phase).sin();

        // Spawn interval across 8 grain slots
        let spawn_interval = ((grain_samples * (0.45 - chop_norm * 0.36)) as u32).max(48);
        self.spawn_counter += 1;
        if self.spawn_counter >= spawn_interval {
            self.spawn_counter = 0;
            let slot_idx = self
                .grains
                .iter()
                .position(|g| !g.active)
                .unwrap_or((self.rng_state as usize) & 7);

            let r1 = self.next_rand();
            let r2 = self.next_rand();

            let base_delay = (0.04 + (p.pitch_detune / 10.0) * 0.55) * sr;
            let (start_delay, pitch_ratio, reverse) = match p.mode {
                ParticleMode::Density => {
                    // Dense harmonic grain swarm with octave/fifth micro-sparkles
                    let harmonic = if r2 > 0.72 {
                        2.0
                    } else if r2 > 0.48 {
                        1.4983
                    } else {
                        1.0 + (r2 - 0.25) * 0.08
                    };
                    (base_delay * (0.15 + 0.85 * r1), harmonic, r1 > 0.75)
                }
                ParticleMode::LfoTime => {
                    // Time-warped LFO scrubbing with Doppler pitch bend
                    let doppler = 0.75 + 0.5 * (2.0 * PI * self.lfo_phase).cos();
                    ((0.02 + lfo_val * 0.65) * sr, doppler, false)
                }
                ParticleMode::RandomPitch => {
                    // Sparkling random pitch arpeggiator (-12st, +7st, +12st, +19st, +24st)
                    let ratios = [0.5, 1.4983, 2.0, 2.9966, 4.0];
                    let pr = ratios[(r2 * 5.0) as usize % 5];
                    (base_delay * (0.1 + 0.9 * r1), pr, r1 > 0.6)
                }
                ParticleMode::PitchShift => {
                    // Pitch knob 0..10 maps from -12st (0.5x) through +7st (1.5x) to +12st (2.0x)
                    let semitones = ((p.pitch_detune / 10.0) - 0.35) * 24.0;
                    let pr = 2.0_f32.powf(semitones / 12.0);
                    (0.05 * sr + r1 * 380.0, pr, false)
                }
                ParticleMode::ReverseGrains => {
                    // Overlapping reverse tape grain swarm with subtle octave overtones
                    let pr = if r2 > 0.75 { 2.0 } else { 1.0 };
                    (base_delay * (0.25 + 0.75 * r1), pr, true)
                }
            };

            let pan = 0.05 + 0.90 * r2;
            self.grains[slot_idx] = Grain {
                active: true,
                start_delay: start_delay.clamp(128.0, (len - 4096) as f32),
                phase: 0.0,
                phase_inc: 1.0 / grain_samples,
                pitch_ratio,
                reverse,
                pan_l: (1.0 - pan).sqrt(),
                pan_r: pan.sqrt(),
            };
        }

        let mut wet_l = 0.0;
        let mut wet_r = 0.0;

        for g in &mut self.grains {
            if !g.active {
                continue;
            }
            let elapsed_samples = g.phase / g.phase_inc;
            let delay_offset = if g.reverse {
                g.start_delay + elapsed_samples * (1.0 + g.pitch_ratio)
            } else {
                let freeze_comp = if p.freeze { 0.0 } else { 1.0 };
                g.start_delay + elapsed_samples * (freeze_comp - g.pitch_ratio)
            };

            let safe_delay = delay_offset.clamp(8.0, (len - 16) as f32);
            let win = (PI * g.phase).sin();

            let sl = lerp_buf(&self.buf_l, self.write_pos, safe_delay) * win;
            let sr_val = lerp_buf(&self.buf_r, self.write_pos, safe_delay) * win;

            wet_l += sl * g.pan_l;
            wet_r += sr_val * g.pan_r;

            g.phase += g.phase_inc;
            if g.phase >= 1.0 {
                g.active = false;
            }
        }

        wet_l = (wet_l * 0.95).tanh();
        wet_r = (wet_r * 0.95).tanh();

        if !p.freeze {
            let wp = self.write_pos % len;
            self.buf_l[wp] = (in_l + wet_r * fb).tanh();
            self.buf_r[wp] = (in_r + wet_l * fb).tanh();
            self.write_pos = (wp + 1) % len;
        }

        let dry_gain = (1.0 - blend * 0.75).max(0.0);
        let wet_gain = (blend * 1.45).min(1.45);
        let out_l = in_l * dry_gain + wet_l * wet_gain;
        let out_r = in_r * dry_gain + wet_r * wet_gain;
        (out_l, out_r)
    }
}

// ============================================================================
// 4. STRYMON CLOUDBURST (Ambient Reverb + Harmonic String Ensemble)
// ============================================================================
#[derive(Clone)]
pub struct StrymonCloudburstDsp {
    predelay_l: Vec<f32>,
    predelay_r: Vec<f32>,
    pre_pos: usize,
    tank_bufs: [Vec<f32>; 4],
    tank_pos: [usize; 4],
    tank_lp: [f32; 4],
    diff_bufs: [Vec<f32>; 2],
    diff_pos: [usize; 2],
    ens_buf: Vec<f32>,
    ens_pos: usize,
    ens_env: f32,
    ens_shimmer_phase: f32,
    lfo_phase: f32,
}

impl StrymonCloudburstDsp {
    pub fn new(max_sr: usize) -> Self {
        let half_sr = (max_sr / 2).max(24000);
        Self {
            predelay_l: vec![0.0; half_sr],
            predelay_r: vec![0.0; half_sr],
            pre_pos: 0,
            tank_bufs: [
                vec![0.0; 2903],
                vec![0.0; 3571],
                vec![0.0; 4409],
                vec![0.0; 5417],
            ],
            tank_pos: [0; 4],
            tank_lp: [0.0; 4],
            diff_bufs: [vec![0.0; 677], vec![0.0; 997]],
            diff_pos: [0; 2],
            ens_buf: vec![0.0; half_sr],
            ens_pos: 0,
            ens_env: 0.0,
            ens_shimmer_phase: 0.0,
            lfo_phase: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, in_l: f32, in_r: f32, p: &CloudburstParams, sr: f32) -> (f32, f32) {
        if !p.enabled {
            return (in_l, in_r);
        }

        let decay_norm = (p.decay / 10.0).clamp(0.0, 1.0);
        let fb_gain = 0.78 + decay_norm.powf(0.55) * 0.205; // up to 0.985 infinite cathedral cloud
        let mix = (p.mix / 10.0).clamp(0.0, 1.0);
        let tone_damp = 0.16 + (p.tone / 10.0).clamp(0.0, 1.0) * 0.78;
        let mod_spd = 0.20 + (p.mod_speed / 10.0).clamp(0.0, 1.0) * 2.4;

        self.lfo_phase = (self.lfo_phase + mod_spd / sr).rem_euclid(1.0);
        let lfo1 = (2.0 * PI * self.lfo_phase).sin();
        let lfo2 = (2.0 * PI * (self.lfo_phase + 0.25)).sin();
        let lfo3 = (2.0 * PI * (self.lfo_phase + 0.5)).sin();
        let lfo4 = (2.0 * PI * (self.lfo_phase + 0.75)).sin();

        // 1. Pre-delay stage (100% bounds-safe)
        let pre_len = self.predelay_l.len();
        let pre_samples = (4.0 + (p.pre_delay / 10.0).clamp(0.0, 1.0) * (pre_len.saturating_sub(64)) as f32)
            .clamp(4.0, (pre_len.saturating_sub(16)) as f32);
        let pd_l = lerp_buf(&self.predelay_l, self.pre_pos, pre_samples);
        let pd_r = lerp_buf(&self.predelay_r, self.pre_pos, pre_samples);
        let pp_idx = self.pre_pos % pre_len;
        self.predelay_l[pp_idx] = in_l;
        self.predelay_r[pp_idx] = in_r;
        self.pre_pos = (pp_idx + 1) % pre_len;

        // 2. Strymon Cloudburst Harmonic String Ensemble Engine (OFF / MP / FORTE)
        let ens_drive = match p.ensemble_mode {
            CloudburstEnsembleMode::Off => 0.0,
            CloudburstEnsembleMode::MezzoPiano => (p.ensemble / 10.0).clamp(0.0, 1.0) * 0.85,
            CloudburstEnsembleMode::Forte => (p.ensemble / 10.0).clamp(0.0, 1.0) * 1.65,
        };

        let mono_in = (pd_l + pd_r) * 0.5;
        let rect = mono_in.abs();
        let env_alpha = if rect > self.ens_env { 0.005 } else { 0.00025 };
        self.ens_env += env_alpha * (rect - self.ens_env);

        let mut ens_sig_l = 0.0;
        let mut ens_sig_r = 0.0;
        let ens_len = self.ens_buf.len();
        if ens_drive > 0.01 && ens_len > 2048 {
            // Write harmonic-enriched signal to ensemble buffer
            let ep = self.ens_pos % ens_len;
            let full_wave_oct = (mono_in.abs() - self.ens_env * 0.5) * 2.2;
            self.ens_buf[ep] = (mono_in + full_wave_oct * 0.85).tanh();

            // Dual-grain +12st octave string shimmer + 4-voice wide ensemble chorus
            let grain_w = 0.048 * sr;
            self.ens_shimmer_phase = (self.ens_shimmer_phase + 1.0 / grain_w).rem_euclid(1.0);
            let ph_a = self.ens_shimmer_phase;
            let ph_b = (ph_a + 0.5).rem_euclid(1.0);
            let wa = (PI * ph_a).sin().powi(2);
            let wb = (PI * ph_b).sin().powi(2);

            let oct_a = lerp_buf(&self.ens_buf, ep, 64.0 + (1.0 - ph_a) * grain_w) * wa;
            let oct_b = lerp_buf(&self.ens_buf, ep, 64.0 + (1.0 - ph_b) * grain_w) * wb;
            let oct_up = oct_a + oct_b;

            let v1 = lerp_buf(&self.ens_buf, ep, 220.0 + lfo1 * 65.0);
            let v2 = lerp_buf(&self.ens_buf, ep, 370.0 + lfo2 * 75.0);
            let v3 = lerp_buf(&self.ens_buf, ep, 540.0 + lfo3 * 85.0);
            let v4 = lerp_buf(&self.ens_buf, ep, 710.0 + lfo4 * 95.0);
            self.ens_pos = (ep + 1) % ens_len;

            let bloom = (0.55 + 2.2 * self.ens_env).min(1.65);
            ens_sig_l = ((v1 + v3 * 0.85 + oct_up * 0.95) * ens_drive * bloom).tanh();
            ens_sig_r = ((v2 + v4 * 0.85 + oct_up * 0.95) * ens_drive * bloom).tanh();
        }

        // 3. Input All-Pass Diffusers
        let mut d_l = pd_l + ens_sig_l * 0.95;
        let mut d_r = pd_r + ens_sig_r * 0.95;

        let d0_len = self.diff_bufs[0].len();
        let dp0 = self.diff_pos[0] % d0_len;
        let ap0 = self.diff_bufs[0][dp0];
        let in0 = d_l + ap0 * 0.65;
        self.diff_bufs[0][dp0] = in0;
        self.diff_pos[0] = (dp0 + 1) % d0_len;
        d_l = ap0 - in0 * 0.65;

        let d1_len = self.diff_bufs[1].len();
        let dp1 = self.diff_pos[1] % d1_len;
        let ap1 = self.diff_bufs[1][dp1];
        let in1 = d_r + ap1 * 0.65;
        self.diff_bufs[1][dp1] = in1;
        self.diff_pos[1] = (dp1 + 1) % d1_len;
        d_r = ap1 - in1 * 0.65;

        // 4. 4x4 Hadamard Modulated Cloud FDN Tank
        let mut t_out = [0.0_f32; 4];
        let mod_offsets = [lfo1 * 24.0, lfo2 * 29.0, lfo3 * 33.0, lfo4 * 27.0];
        for i in 0..4 {
            let blen = (self.tank_bufs[i].len().saturating_sub(48)) as f32 + mod_offsets[i];
            let raw = lerp_buf(&self.tank_bufs[i], self.tank_pos[i], blen.max(8.0));
            self.tank_lp[i] += tone_damp * (raw - self.tank_lp[i]);
            t_out[i] = self.tank_lp[i];
        }

        let sum = (t_out[0] + t_out[1] + t_out[2] + t_out[3]) * 0.5;
        let fb0 = (sum - t_out[0]) * fb_gain + d_l * 0.55;
        let fb1 = (sum - t_out[1]) * fb_gain + d_r * 0.55;
        let fb2 = (sum - t_out[2]) * fb_gain - d_l * 0.55;
        let fb3 = (sum - t_out[3]) * fb_gain - d_r * 0.55;

        let fbs = [fb0, fb1, fb2, fb3];
        for i in 0..4 {
            let tlen = self.tank_bufs[i].len();
            let tp = self.tank_pos[i] % tlen;
            self.tank_bufs[i][tp] = fbs[i].tanh();
            self.tank_pos[i] = (tp + 1) % tlen;
        }

        let wet_l = (t_out[0] + t_out[2] * 0.9 + ens_sig_l * 0.85) * 0.85;
        let wet_r = (t_out[1] + t_out[3] * 0.9 + ens_sig_r * 0.85) * 0.85;

        let dry_gain = (1.0 - mix * 0.65).max(0.0);
        let wet_gain = (mix * 1.40).min(1.40);
        let out_l = in_l * dry_gain + wet_l * wet_gain;
        let out_r = in_r * dry_gain + wet_r * wet_gain;
        (out_l, out_r)
    }
}

// ============================================================================
// 5. BOSS CE-2W / DC-2W (Waza Craft Stereo Chorus & Dimension Expander)
//    Specifically engineered to convert the mono Korg MS-20 signal into a lush,
//    phase-decorrelated true-stereo image before or within the pedal chain.
// ============================================================================
#[derive(Clone)]
pub struct BossCe2wChorusDsp {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write_pos: usize,
    lfo_phase1: f32,
    lfo_phase2: f32,
    lp_l: f32,
    lp_r: f32,
    hp_cross_l: f32,
    hp_cross_r: f32,
}

impl BossCe2wChorusDsp {
    pub fn new(max_sr: usize) -> Self {
        let len = (max_sr / 4).max(16384); // ~250ms stereo BBD buffer
        Self {
            buf_l: vec![0.0; len],
            buf_r: vec![0.0; len],
            write_pos: 0,
            lfo_phase1: 0.0,
            lfo_phase2: 0.37,
            lp_l: 0.0,
            lp_r: 0.0,
            hp_cross_l: 0.0,
            hp_cross_r: 0.0,
        }
    }

    #[inline]
    fn tri_wave(phase: f32) -> f32 {
        let p = phase.rem_euclid(1.0);
        if p < 0.5 {
            p * 4.0 - 1.0
        } else {
            3.0 - p * 4.0
        }
    }

    #[inline]
    pub fn process(&mut self, in_l: f32, in_r: f32, p: &BossCe2wParams, sr: f32) -> (f32, f32) {
        if !p.enabled {
            return (in_l, in_r);
        }

        let len = self.buf_l.len();
        let rate_norm = (p.rate / 10.0).clamp(0.0, 1.0);
        let depth_norm = (p.depth / 10.0).clamp(0.0, 1.0);
        let width_norm = (p.stereo_width / 10.0).clamp(0.0, 1.0);
        let warmth_norm = (p.warmth / 10.0).clamp(0.0, 1.0);
        let mix_norm = (p.mix / 10.0).clamp(0.0, 1.0);

        // MN3007 BBD analog pre-emphasis / warmth saturation
        let drive = 1.0 + warmth_norm * 0.95;
        let sat_l = (in_l * drive).tanh() / drive.sqrt();
        let sat_r = (in_r * drive).tanh() / drive.sqrt();

        // Advance primary and secondary LFOs
        let base_hz = match p.mode {
            ChorusMode::Ce2AnalogStereo => 0.18 + rate_norm.powf(1.4) * 4.2,
            ChorusMode::DimensionCDc2w => 0.12 + rate_norm.powf(1.3) * 2.4,
            ChorusMode::TriStereo3Voice => 0.22 + rate_norm.powf(1.35) * 3.8,
            ChorusMode::JunoSwirl => 0.40 + rate_norm.powf(1.5) * 5.8,
        };
        self.lfo_phase1 = (self.lfo_phase1 + base_hz / sr).rem_euclid(1.0);
        self.lfo_phase2 = (self.lfo_phase2 + (base_hz * 1.31) / sr).rem_euclid(1.0);

        // Haas stereo offset (up to 6.5 ms) for immediate mono-to-stereo spaciousness
        let haas_samples = width_norm * 0.0065 * sr;

        let (raw_wet_l, raw_wet_r, fb_l, fb_r) = match p.mode {
            ChorusMode::Ce2AnalogStereo => {
                // Classic BOSS CE-2W / CE-1 Stereo BBD Mode:
                // 50% triangle + 50% sine LFOs in 180-degree phase opposition on L and R
                let tri = Self::tri_wave(self.lfo_phase1);
                let sin = (2.0 * PI * self.lfo_phase1).sin();
                let lfo_l = 0.55 * tri + 0.45 * sin;
                let lfo_r = -lfo_l * (0.35 + 0.65 * width_norm)
                    + (2.0 * PI * (self.lfo_phase1 + 0.25)).sin() * 0.35 * width_norm;

                let center_l = 0.0072 * sr; // 7.2 ms BBD center
                let center_r = 0.0098 * sr + haas_samples * 0.5; // 9.8 ms + Haas offset
                let mod_excursion = (0.0006 + depth_norm * 0.0052) * sr;

                let wl = lerp_buf(&self.buf_l, self.write_pos, center_l + lfo_l * mod_excursion);
                let wr = lerp_buf(&self.buf_r, self.write_pos, center_r + lfo_r * mod_excursion);

                // Phase-decorrelated CE-1 stereo cross-blend
                let stereo_r = wr * (1.0 - width_norm * 0.45) - wl * (width_norm * 0.45);
                let stereo_l = wl * (1.0 - width_norm * 0.25) + wr * (width_norm * 0.25);
                (stereo_l, stereo_r, wl * 0.15 * warmth_norm, wr * 0.15 * warmth_norm)
            }
            ChorusMode::DimensionCDc2w => {
                // Roland Dimension D / BOSS DC-2W Spatial Expander:
                // Dual opposite-phase triangle BBD lines with high-pass cross-channel feedback
                let tri_a = Self::tri_wave(self.lfo_phase1);
                let tri_b = Self::tri_wave(self.lfo_phase2);
                let mod_a = (0.0005 + depth_norm * 0.0044) * sr;
                let mod_b = (0.0005 + depth_norm * 0.0048) * sr;

                let d_l1 = lerp_buf(&self.buf_l, self.write_pos, 0.0064 * sr + tri_a * mod_a);
                let d_r1 = lerp_buf(
                    &self.buf_r,
                    self.write_pos,
                    0.0089 * sr + haas_samples * 0.4 - tri_a * mod_a,
                );
                let d_l2 = lerp_buf(&self.buf_l, self.write_pos, 0.0128 * sr + tri_b * mod_b);
                let d_r2 = lerp_buf(
                    &self.buf_r,
                    self.write_pos,
                    0.0152 * sr + haas_samples * 0.6 - tri_b * mod_b,
                );

                let wl = (d_l1 + d_l2 * 0.85) * 0.72;
                let wr = (d_r1 + d_r2 * 0.85) * 0.72;

                // High-pass filter on the wet taps for Dimension D cross-injection
                self.hp_cross_l += 0.045 * (wl - self.hp_cross_l);
                self.hp_cross_r += 0.045 * (wr - self.hp_cross_r);
                let hp_l = wl - self.hp_cross_l;
                let hp_r = wr - self.hp_cross_r;

                let cross = 0.55 * width_norm;
                (wl - hp_r * cross, wr - hp_l * cross, hp_r * 0.22, hp_l * 0.22)
            }
            ChorusMode::TriStereo3Voice => {
                // 3-Voice Tri-Stereo Chorus (0, 120, 240 deg phases)
                let ph0 = 2.0 * PI * self.lfo_phase1;
                let ph1 = ph0 + 2.0 * PI / 3.0;
                let ph2 = ph0 + 4.0 * PI / 3.0;

                let lfo0 = ph0.sin() + 0.25 * (2.0 * PI * self.lfo_phase2).sin();
                let lfo1 = ph1.sin() + 0.25 * (2.0 * PI * (self.lfo_phase2 + 0.33)).sin();
                let lfo2 = ph2.sin() + 0.25 * (2.0 * PI * (self.lfo_phase2 + 0.67)).sin();

                let mod_s = (0.0008 + depth_norm * 0.0062) * sr;
                let v_left = lerp_buf(&self.buf_l, self.write_pos, 0.0075 * sr + lfo0 * mod_s);
                let v_center = lerp_buf(
                    &self.buf_l,
                    self.write_pos,
                    0.0118 * sr + haas_samples * 0.3 + lfo1 * mod_s,
                );
                let v_right = lerp_buf(
                    &self.buf_r,
                    self.write_pos,
                    0.0155 * sr + haas_samples * 0.7 + lfo2 * mod_s,
                );

                let spread = 0.35 + width_norm * 0.65;
                let wl = v_left * 0.85 + v_center * 0.45 - v_right * (0.25 * spread);
                let wr = v_right * 0.85 + v_center * 0.45 - v_left * (0.25 * spread);
                (wl, wr, v_center * 0.22 * warmth_norm, -v_center * 0.22 * warmth_norm)
            }
            ChorusMode::JunoSwirl => {
                // Roland Juno-60 Chorus I+II Ensemble Swirl:
                // Dual BBD lines with rich triangle modulation and warm analog feedback
                let tri1 = Self::tri_wave(self.lfo_phase1);
                let tri2 = Self::tri_wave(self.lfo_phase2);
                let mod_s = (0.0010 + depth_norm * 0.0070) * sr;

                let j_l1 = lerp_buf(&self.buf_l, self.write_pos, 0.0056 * sr + tri1 * mod_s);
                let j_r1 = lerp_buf(
                    &self.buf_r,
                    self.write_pos,
                    0.0078 * sr + haas_samples * 0.5 - tri1 * mod_s,
                );
                let j_l2 = lerp_buf(&self.buf_l, self.write_pos, 0.0105 * sr + tri2 * mod_s * 0.8);
                let j_r2 = lerp_buf(
                    &self.buf_r,
                    self.write_pos,
                    0.0132 * sr + haas_samples * 0.8 - tri2 * mod_s * 0.8,
                );

                let wl = (j_l1 * 0.75 + j_l2 * 0.65 - j_r1 * 0.25 * width_norm) * 0.85;
                let wr = (j_r1 * 0.75 + j_r2 * 0.65 - j_l1 * 0.25 * width_norm) * 0.85;
                (wl, wr, wl * 0.25 * warmth_norm, wr * 0.25 * warmth_norm)
            }
        };

        // Analog BBD reconstruction low-pass filter (warmer at higher Warmth settings)
        let bbd_cutoff_alpha = (0.88 - warmth_norm * 0.52).clamp(0.25, 0.92);
        self.lp_l += bbd_cutoff_alpha * (raw_wet_l - self.lp_l);
        self.lp_r += bbd_cutoff_alpha * (raw_wet_r - self.lp_r);

        let wet_l = (self.lp_l * 1.15).tanh();
        let wet_r = (self.lp_r * 1.15).tanh();

        let wp = self.write_pos % len;
        self.buf_l[wp] = (sat_l + fb_l).tanh();
        self.buf_r[wp] = (sat_r + fb_r).tanh();
        self.write_pos = (wp + 1) % len;

        // Equal-power / lush stereo chorus blend
        let dry_gain = (1.0 - mix_norm * 0.45).max(0.25);
        let wet_gain = (mix_norm * 1.25).min(1.25);
        let out_l = in_l * dry_gain + wet_l * wet_gain;
        let out_r = in_r * dry_gain + wet_r * wet_gain;
        (out_l, out_r)
    }
}

// ============================================================================
// MASTER PEDALBOARD CHAIN (Configurable Order & Individual Stomp Bypass)
// ============================================================================
#[derive(Clone)]
pub struct PedalboardDsp {
    pub chorus: BossCe2wChorusDsp,
    pub zoom: ZoomMs70Dsp,
    pub dd8: BossDd8Dsp,
    pub particle: RedPandaParticle2Dsp,
    pub cloudburst: StrymonCloudburstDsp,
}

impl PedalboardDsp {
    pub fn new(max_sr: usize) -> Self {
        Self {
            chorus: BossCe2wChorusDsp::new(max_sr),
            zoom: ZoomMs70Dsp::new(max_sr),
            dd8: BossDd8Dsp::new(max_sr),
            particle: RedPandaParticle2Dsp::new(max_sr),
            cloudburst: StrymonCloudburstDsp::new(max_sr),
        }
    }

    #[inline]
    pub fn process_stereo(
        &mut self,
        in_l: f32,
        in_r: f32,
        params: &PedalboardParams,
        sample_rate: f32,
    ) -> (f32, f32) {
        let mut l = in_l;
        let mut r = in_r;

        for pedal_id in &params.chain_order {
            match pedal_id {
                PedalId::BossCe2wChorus => {
                    let (nl, nr) = self.chorus.process(l, r, &params.chorus, sample_rate);
                    l = nl;
                    r = nr;
                }
                PedalId::ZoomMs70Cdr => {
                    let (nl, nr) = self.zoom.process(l, r, &params.zoom, sample_rate);
                    l = nl;
                    r = nr;
                }
                PedalId::BossDd8 => {
                    let (nl, nr) = self.dd8.process(l, r, &params.dd8, sample_rate);
                    l = nl;
                    r = nr;
                }
                PedalId::RedPandaParticle2 => {
                    let (nl, nr) = self.particle.process(l, r, &params.particle, sample_rate);
                    l = nl;
                    r = nr;
                }
                PedalId::StrymonCloudburst => {
                    let (nl, nr) = self.cloudburst.process(l, r, &params.cloudburst, sample_rate);
                    l = nl;
                    r = nr;
                }
            }
        }

        (l.clamp(-1.5, 1.5), r.clamp(-1.5, 1.5))
    }
}

