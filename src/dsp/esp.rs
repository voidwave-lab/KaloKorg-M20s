use std::f32::consts::PI;

/// External Signal Processor (ESP) section on the bottom-right of the Korg MS-20 front panel.
/// Signal Chain:
/// `SIGNAL IN` -> `AMP (SIGNAL LEVEL)` -> `LOW CUT` -> `HIGH CUT` -> `F-V CONVERT (CV ADJUST)`
///             -> `ENVELOPE FOLLOWER (ENV OUT)` -> `TRIG DETECT (THRESHOLD LEVEL)`
#[derive(Clone, Debug, Default)]
pub struct EspProcessor {
    hp_s1: f32,
    lp_s1: f32,
    env_state: f32,
    prev_sign: bool,
    samples_since_zc: u32,
    tracked_freq_hz: f32,
    pub amp_out: f32,
    pub bpf_out: f32,
    pub fv_cv_out: f32,
    pub env_out: f32,
    pub trig_out: f32,
    pub peak_led: bool,
    pub trig_led: bool,
}

impl EspProcessor {
    #[inline]
    pub fn process(
        &mut self,
        sig_in: f32,
        signal_level_knob: f32,
        low_cut_knob: f32,
        high_cut_knob: f32,
        cv_adjust_knob: f32,
        threshold_knob: f32,
        sample_rate: f32,
    ) {
        // 1. Preamp stage (LM324 / RC4558)
        let gain = 0.5 + (signal_level_knob.clamp(0.0, 10.0) / 10.0).powf(1.3) * 8.5;
        let amp = (sig_in * gain).tanh();
        self.amp_out = amp;
        self.peak_led = amp.abs() > 0.65;

        // 2. Band-Pass Filter (Low Cut HPF + High Cut LPF)
        let low_cut_hz = 30.0 * 2.0_f32.powf((low_cut_knob.clamp(0.0, 10.0) / 10.0) * 6.5);
        let high_cut_hz = 120.0 * 2.0_f32.powf((high_cut_knob.clamp(0.0, 10.0) / 10.0) * 7.0);

        let g_hp = (PI * low_cut_hz.clamp(20.0, sample_rate * 0.4) / sample_rate).tan();
        let k_hp = g_hp / (1.0 + g_hp);
        let v_hp = k_hp * (amp - self.hp_s1);
        let lp_of_hp = v_hp + self.hp_s1;
        self.hp_s1 = lp_of_hp + v_hp;
        let hp_out = amp - lp_of_hp;

        let g_lp = (PI * high_cut_hz.clamp(60.0, sample_rate * 0.45) / sample_rate).tan();
        let k_lp = g_lp / (1.0 + g_lp);
        let v_lp = k_lp * (hp_out - self.lp_s1);
        let bpf = v_lp + self.lp_s1;
        self.lp_s1 = bpf + v_lp;
        self.bpf_out = (bpf * 1.85).tanh();

        // 3. Zero-crossing Frequency-to-Voltage (F-V) Converter
        self.samples_since_zc = self.samples_since_zc.saturating_add(1);
        let cur_sign = bpf >= 0.0;
        if cur_sign && !self.prev_sign && self.samples_since_zc > 6 {
            let inst_freq = sample_rate / (self.samples_since_zc as f32);
            if (25.0..=5000.0).contains(&inst_freq) {
                if self.tracked_freq_hz < 25.0 {
                    self.tracked_freq_hz = inst_freq;
                } else {
                    self.tracked_freq_hz = 0.80 * self.tracked_freq_hz + 0.20 * inst_freq;
                }
            }
            self.samples_since_zc = 0;
        }
        self.prev_sign = cur_sign;

        let cv_scale = (cv_adjust_knob.clamp(0.0, 10.0) / 10.0) * 2.5;
        // Convert tracked Hz to normalized CV (-1.0 .. +1.0 around 110.0 Hz A2 reference so C3/C4 produce strong positive CV)
        let f_norm = if self.tracked_freq_hz > 20.0 {
            (((self.tracked_freq_hz / 110.0).log2() * 0.35 + bpf * 0.25) * cv_scale).clamp(-1.0, 1.0)
        } else {
            (bpf * 0.4 * cv_scale).clamp(-1.0, 1.0)
        };
        self.fv_cv_out = f_norm;

        // 4. Precision full-wave rectifier & Envelope Follower
        let rect = (amp.abs() * 1.35 + bpf.abs() * 0.45).min(1.4);
        let coeff = if rect > self.env_state {
            1.0 - (-1.0 / (0.0025 * sample_rate)).exp()
        } else {
            1.0 - (-1.0 / (0.035 * sample_rate)).exp()
        };
        self.env_state += coeff * (rect - self.env_state);
        self.env_out = self.env_state.clamp(0.0, 1.0);

        // 5. Comparator Trigger Detector (responds to both envelope threshold & AC waveform peaks above threshold)
        let thresh = (threshold_knob.clamp(0.0, 10.0) / 10.0) * 0.75 + 0.04;
        self.trig_led = self.env_out >= thresh || amp > thresh;
        self.trig_out = if self.trig_led { 1.0 } else { 0.0 };
    }
}
