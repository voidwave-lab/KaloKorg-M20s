use serde::{Deserialize, Serialize};

use crate::dsp::{
    envelope::{Eg1Dar, Eg2Hadsr},
    esp::EspProcessor,
    korg35::DualKorg35Filter,
    lfo::ModulationGenerator,
    oscillator::{DualVco, NoiseGenerator},
    pedals::PedalboardDsp,
};
use crate::params::{
    BossDd8Mode, ChorusMode, CloudburstEnsembleMode, FilterRevision, ParticleMode,
    PedalboardParams, SynthParams, Vco1Scale, Vco1Waveform, Vco2Scale, Vco2Waveform, ZoomMode,
};
use crate::patchbay::{PatchCable, PatchDest, PatchSource, Patchbay};
use crate::voice::MonoVoice;
use crate::AudioProcessor;

/// Complete serializable snapshot of a KaloKorg M20s patch (Synth + Patchbay + Pedalboard)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub category: String,
    pub description: String,
    pub synth: SynthParams,
    pub cables: Vec<PatchCable>,
    pub pedals: PedalboardParams,
}

impl Preset {
    pub fn factory_presets() -> Vec<Preset> {
        vec![
            // 1. Classic MS-20 Manual Fig. 5
            Preset {
                name: "01. Classic MS-20 Lead (Fig.5)".to_string(),
                category: "Classic MS-20".to_string(),
                description: "Dual PolyBLEP Sawtooth 8' through Rev.1 KORG35 HPF/LPF with warm DD-8 & Cloudburst.".to_string(),
                synth: SynthParams::default(),
                cables: vec![],
                pedals: PedalboardParams::default(),
            },
            // 2. Dual-Peak KORG35 Scream
            Preset {
                name: "02. KORG35 Dual-Peak Scream".to_string(),
                category: "Aggressive / Acid".to_string(),
                description: "Both HPF and LPF Peak set high for authentic KLM-128 asymmetric diode/transistor screaming formants.".to_string(),
                synth: SynthParams {
                    vco1_waveform: Vco1Waveform::Sawtooth,
                    vco1_scale: Vco1Scale::Feet16,
                    vco2_waveform: Vco2Waveform::NarrowPulse,
                    vco2_pitch: 0.08,
                    vco2_scale: Vco2Scale::Feet8,
                    hpf_cutoff: 3.4,
                    hpf_peak: 8.4,
                    hpf_eg2_ext: 3.5,
                    lpf_cutoff: 4.2,
                    lpf_peak: 8.6,
                    lpf_eg2_ext: 6.5,
                    eg2_attack: 0.1,
                    eg2_decay: 4.5,
                    eg2_sustain: 3.5,
                    eg2_release: 3.0,
                    filter_revision: FilterRevision::Rev1Korg35,
                    ..SynthParams::default()
                },
                cables: vec![],
                pedals: PedalboardParams::default(),
            },
            // 3. TC4011BP CMOS XOR Ring Modulator
            Preset {
                name: "03. TC4011 CMOS XOR Ring Mod".to_string(),
                category: "Experimental".to_string(),
                description: "Digital CMOS NAND/XOR ring modulation between VCO 1 & VCO 2 with EG1 sweeping VCO 2 pitch via patch cable.".to_string(),
                synth: SynthParams {
                    vco1_waveform: Vco1Waveform::Pulse,
                    vco1_pw: 2.5,
                    vco1_scale: Vco1Scale::Feet8,
                    vco2_waveform: Vco2Waveform::RingMod,
                    vco2_pitch: 2.9, // ~ +7 semitones fifth interval
                    vco2_scale: Vco2Scale::Feet4,
                    vco1_level: 4.5,
                    vco2_level: 10.0,
                    hpf_cutoff: 2.0,
                    hpf_peak: 5.0,
                    lpf_cutoff: 6.2,
                    lpf_peak: 6.0,
                    lpf_eg2_ext: 5.0,
                    eg1_delay: 0.5,
                    eg1_attack: 3.5,
                    eg1_release: 4.0,
                    ..SynthParams::default()
                },
                cables: vec![PatchCable {
                    source: PatchSource::Eg1NormalOut,
                    dest: PatchDest::Vco2CvIn,
                    color_idx: 1,
                }],
                pedals: PedalboardParams::default(),
            },
            // 4. Cloudburst Cathedral Ensemble Drone
            Preset {
                name: "04. Cloudburst Cathedral Ensemble".to_string(),
                category: "Ambient / Drone".to_string(),
                description: "Lush MS-20 PWM + Triangle pad feeding Strymon Cloudburst FORTE String Ensemble & Zoom MS-70CDR Ice Shimmer.".to_string(),
                synth: SynthParams {
                    vco1_waveform: Vco1Waveform::Pulse,
                    vco1_pw: 4.5,
                    vco1_scale: Vco1Scale::Feet16,
                    portamento_time: 3.5,
                    vco2_waveform: Vco2Waveform::Sawtooth,
                    vco2_pitch: 0.06,
                    vco2_scale: Vco2Scale::Feet8,
                    hpf_cutoff: 1.8,
                    hpf_peak: 4.2,
                    lpf_cutoff: 4.8,
                    lpf_peak: 4.5,
                    lpf_mg_text: 2.2,
                    lpf_eg2_ext: 3.8,
                    mg_waveform: 5.0,
                    mg_frequency: 2.2,
                    eg2_attack: 4.5,
                    eg2_decay: 6.5,
                    eg2_sustain: 8.5,
                    eg2_release: 7.5,
                    ..SynthParams::default()
                },
                cables: vec![],
                pedals: {
                    let mut p = PedalboardParams::default();
                    p.zoom.enabled = true;
                    p.zoom.mode = ZoomMode::IceShimmerDelay;
                    p.zoom.depth = 6.5;
                    p.zoom.mix = 5.5;
                    p.cloudburst.enabled = true;
                    p.cloudburst.ensemble_mode = CloudburstEnsembleMode::Forte;
                    p.cloudburst.decay = 9.2;
                    p.cloudburst.ensemble = 8.5;
                    p.cloudburst.mix = 7.2;
                    p
                },
            },
            // 5. Red Panda Particle 2 Granular Cosmos
            Preset {
                name: "05. Particle 2 Granular Cosmos".to_string(),
                category: "Granular / Experimental".to_string(),
                description: "Sample & Hold Pink Noise modulating LPF Cutoff into Red Panda Particle 2 Random Pitch Grains + Reverse DD-8.".to_string(),
                synth: SynthParams {
                    vco1_waveform: Vco1Waveform::Triangle,
                    vco1_scale: Vco1Scale::Feet8,
                    vco2_waveform: Vco2Waveform::Square,
                    vco2_pitch: 2.916, // +7st
                    vco2_scale: Vco2Scale::Feet4,
                    lpf_cutoff: 5.0,
                    lpf_peak: 7.2,
                    lpf_mg_text: 6.5,
                    mg_waveform: 5.0,
                    mg_frequency: 6.2,
                    eg2_attack: 0.2,
                    eg2_decay: 3.5,
                    eg2_sustain: 6.0,
                    eg2_release: 5.5,
                    ..SynthParams::default()
                },
                cables: vec![
                    PatchCable {
                        source: PatchSource::PinkNoiseOut,
                        dest: PatchDest::SampleHoldIn,
                        color_idx: 0,
                    },
                    PatchCable {
                        source: PatchSource::MgPulseOut,
                        dest: PatchDest::SampleHoldClock,
                        color_idx: 2,
                    },
                    PatchCable {
                        source: PatchSource::SampleHoldOut,
                        dest: PatchDest::TotalExt,
                        color_idx: 3,
                    },
                ],
                pedals: {
                    let mut p = PedalboardParams::default();
                    p.particle.enabled = true;
                    p.particle.mode = ParticleMode::RandomPitch;
                    p.particle.blend = 6.5;
                    p.particle.chop = 7.2;
                    p.particle.grain_size = 4.8;
                    p.particle.feedback = 6.8;
                    p.dd8.enabled = true;
                    p.dd8.mode = BossDd8Mode::Reverse;
                    p.dd8.effect_level = 5.5;
                    p.cloudburst.enabled = true;
                    p.cloudburst.decay = 8.5;
                    p
                },
            },
            // 6. Sub-Bass KORG35 Growl
            Preset {
                name: "06. Sub-Bass KORG35 Growl".to_string(),
                category: "Bass".to_string(),
                description: "Classic MS-20 trick: 32' Triangle + 16' Saw with resonant HPF Peak boosting the sub fundamental!".to_string(),
                synth: SynthParams {
                    vco1_waveform: Vco1Waveform::Triangle,
                    vco1_scale: Vco1Scale::Feet32,
                    vco2_waveform: Vco2Waveform::Sawtooth,
                    vco2_pitch: 0.04,
                    vco2_scale: Vco2Scale::Feet16,
                    hpf_cutoff: 1.6,
                    hpf_peak: 7.4,
                    lpf_cutoff: 3.2,
                    lpf_peak: 5.8,
                    lpf_eg2_ext: 6.2,
                    eg2_attack: 0.0,
                    eg2_decay: 2.8,
                    eg2_sustain: 2.5,
                    eg2_release: 1.8,
                    ..SynthParams::default()
                },
                cables: vec![],
                pedals: {
                    let mut p = PedalboardParams::default();
                    p.zoom.enabled = true;
                    p.zoom.mode = ZoomMode::CoronaTriChorus;
                    p.zoom.mix = 2.5;
                    p.dd8.enabled = false;
                    p.particle.enabled = false;
                    p.cloudburst.enabled = false;
                    p
                },
            },
        ]
    }
}

/// Master KaloKorg M20s DSP Engine
pub struct KaloKorgEngine {
    pub sample_rate: f32,
    pub params: SynthParams,
    pub pedals_params: PedalboardParams,
    pub patchbay: Patchbay,
    pub voice: MonoVoice,

    pub vco: DualVco,
    pub noise: NoiseGenerator,
    pub filter: DualKorg35Filter,
    pub eg1: Eg1Dar,
    pub eg2: Eg2Hadsr,
    pub mg: ModulationGenerator,
    pub esp: EspProcessor,
    pub pedals_dsp: PedalboardDsp,

    /// Oscilloscope ring buffer (256 samples) for TUI and Web UI visualization
    pub scope_buffer: [f32; 256],
    pub scope_write_idx: usize,
    scope_decim: u32,

    /// Peak level meters (Left, Right)
    pub peak_l: f32,
    pub peak_r: f32,

    /// Last synthesized output sample (available at SIGNAL OUT / PHONES patch jacks)
    last_signal_out: f32,

    /// Optional external audio input sample from a DAW host (feeds ESP SIGNAL IN and EXT IN)
    pub external_audio_input: f32,
}

impl KaloKorgEngine {
    pub fn new(sample_rate: f32) -> Self {
        let max_sr = (sample_rate as usize).max(48000);
        Self {
            sample_rate,
            params: SynthParams::default(),
            pedals_params: PedalboardParams::default(),
            patchbay: Patchbay::default(),
            voice: MonoVoice::default(),
            vco: DualVco::default(),
            noise: NoiseGenerator::default(),
            filter: DualKorg35Filter::default(),
            eg1: Eg1Dar::default(),
            eg2: Eg2Hadsr::default(),
            mg: ModulationGenerator::default(),
            esp: EspProcessor::default(),
            pedals_dsp: PedalboardDsp::new(max_sr),
            scope_buffer: [0.0; 256],
            scope_write_idx: 0,
            scope_decim: 0,
            peak_l: 0.0,
            peak_r: 0.0,
            last_signal_out: 0.0,
            external_audio_input: 0.0,
        }
    }

    pub fn load_preset(&mut self, preset: &Preset) {
        let current_pad_db = self.params.output_pad_db;
        self.params = preset.synth.clone();
        self.params.output_pad_db = current_pad_db;
        self.patchbay.cables = preset.cables.clone();
        self.pedals_params = preset.pedals.clone();
    }

    /// Move a pedal left or right in the effect chain order
    pub fn move_pedal(&mut self, index: usize, direction: isize) {
        let new_idx = (index as isize + direction).clamp(0, 4) as usize;
        if new_idx != index {
            self.pedals_params.chain_order.swap(index, new_idx);
        }
    }

    /// Synthesize one stereo sample pair `(left, right)` following the exact Korg MS-20
    /// block diagram (`KORG-MS-20-Monophonic-Synthesizer-fig-3.jpg`) and pedalboard chain.
    #[inline]
    pub fn render_frame(&mut self) -> (f32, f32) {
        let sr = self.sample_rate;

        // 1. Noise Generator (White & Pink Noise)
        let (white, pink) = self.noise.step();
        self.patchbay.source_values[PatchSource::WhiteNoiseOut as usize] = white;
        self.patchbay.source_values[PatchSource::PinkNoiseOut as usize] = pink;

        // 2. Modulation Generator (MG LFO)
        let (mg_tri, mg_pulse) = self.mg.process(self.params.mg_waveform, self.params.mg_frequency, sr);
        self.patchbay.source_values[PatchSource::MgTriangleOut as usize] = mg_tri;
        self.patchbay.source_values[PatchSource::MgPulseOut as usize] = mg_pulse;

        // 3. Control Wheel & Momentary Switch
        self.patchbay.source_values[PatchSource::ControlWheelOut as usize] =
            (self.params.control_wheel / 5.0).clamp(-1.0, 1.0);
        self.patchbay.source_values[PatchSource::MomentarySwOut as usize] =
            if self.params.momentary_switch { 1.0 } else { 0.0 };

        // 4. Keyboard/MIDI Pitch CV & Portamento
        let (_glided_hz, key_cv) = self.voice.step_portamento(self.params.portamento_time, sr);
        let kbd_gate = self.voice.gate || self.params.drone_gate;
        self.patchbay.source_values[PatchSource::KeyCvOut as usize] = key_cv;
        self.patchbay.source_values[PatchSource::TrigOut as usize] = if kbd_gate { 1.0 } else { 0.0 };

        // 5. External Signal Processor (ESP)
        // Normalled to DAW external audio input (if present) or subtle internal synth feedback
        let esp_default_in = if self.external_audio_input.abs() > 1e-6 {
            self.external_audio_input
        } else {
            self.last_signal_out * 0.35 + pink * 0.02
        };
        let esp_in = self
            .patchbay
            .read_dest(PatchDest::EspSignalIn)
            .unwrap_or(esp_default_in);
        self.esp.process(
            esp_in,
            self.params.esp_signal_level,
            self.params.esp_low_cut,
            self.params.esp_high_cut,
            self.params.esp_cv_adjust,
            self.params.esp_threshold,
            sr,
        );
        self.patchbay.source_values[PatchSource::EspAmpOut as usize] = self.esp.amp_out;
        self.patchbay.source_values[PatchSource::EspBpfOut as usize] = self.esp.bpf_out;
        self.patchbay.source_values[PatchSource::EspFvCvOut as usize] = self.esp.fv_cv_out;
        self.patchbay.source_values[PatchSource::EspEnvOut as usize] = self.esp.env_out;
        self.patchbay.source_values[PatchSource::EspTrigOut as usize] = self.esp.trig_out;

        // 6. Step Patchbay Auxiliary Modules (S&H and Patch MVCA)
        self.patchbay.step_aux_modules();

        // 7. Resolve Triggers for EG 1 and EG 2 (normalled to KBD Gate, overridable via TRIG IN / EG1 TRIG IN)
        let main_trig_ext = self.patchbay.read_dest(PatchDest::MainTrigIn).map(|v| v > 0.25);
        let eg2_gate = main_trig_ext.unwrap_or(kbd_gate);
        let eg1_gate = self
            .patchbay
            .read_dest(PatchDest::Eg1TrigIn)
            .map(|v| v > 0.25)
            .or(main_trig_ext)
            .unwrap_or(kbd_gate);

        // 8. Envelope Generators EG 1 (DAR) & EG 2 (HADSR)
        let (eg1_norm, eg1_rev) = self.eg1.process(
            eg1_gate,
            self.params.eg1_delay,
            self.params.eg1_attack,
            self.params.eg1_release,
            sr,
        );
        let (eg2_norm, eg2_rev) = self.eg2.process(
            eg2_gate,
            self.params.eg2_hold,
            self.params.eg2_attack,
            self.params.eg2_decay,
            self.params.eg2_sustain,
            self.params.eg2_release,
            sr,
        );
        self.patchbay.source_values[PatchSource::Eg1NormalOut as usize] = eg1_norm;
        self.patchbay.source_values[PatchSource::Eg1RevOut as usize] = eg1_rev;
        self.patchbay.source_values[PatchSource::Eg2RevOut as usize] = eg2_rev;

        // 9. Resolve Normalled vs Patched Modulation Sources (per Fig. 3 Block Diagram):
        // - TOTAL jack overrides MG Triangle for all three `MG/T.EXT` knobs (FM, HPF, LPF)!
        let mg_or_total = self.patchbay.read_dest(PatchDest::TotalExt).unwrap_or(mg_tri);

        // - FREQ jack overrides EG1 Normal Out for VCO `EG1/EXT` knob!
        let fm_eg1_or_ext = self.patchbay.read_dest(PatchDest::FreqExt).unwrap_or(eg1_norm);

        // - HPF CUTOFF jack overrides EG2 Normal Out for HPF `EG2/EXT` knob!
        let hpf_eg2_or_ext = self
            .patchbay
            .read_dest(PatchDest::HpfCutoffExt)
            .unwrap_or(eg2_norm);

        // - LPF CUTOFF jack overrides EG2 Normal Out for LPF `EG2/EXT` knob!
        let lpf_eg2_or_ext = self
            .patchbay
            .read_dest(PatchDest::LpfCutoffExt)
            .unwrap_or(eg2_norm);

        // 10. Resolve Pitch CV for VCO 1+2 and VCO 2 Only
        let vco12_cv = self.patchbay.read_dest(PatchDest::Vco1And2CvIn).unwrap_or(key_cv);
        let vco2_cv = self.patchbay.read_dest(PatchDest::Vco2CvIn).unwrap_or(vco12_cv);

        // Frequency Modulation octaves from MG/T.EXT and EG1/EXT knobs
        let fm_octaves = (self.params.fm_mg_text / 10.0).powf(1.6) * mg_or_total * 1.5
            + (self.params.fm_eg1_ext / 10.0).powf(1.5) * fm_eg1_or_ext * 2.0;

        let base_freq_vco1 = 261.62558 * 2.0_f32.powf(vco12_cv * 4.0 + fm_octaves);
        let base_freq_vco2 = 261.62558 * 2.0_f32.powf(vco2_cv * 4.0 + fm_octaves);

        // 11. Synthesize VCO 1 & VCO 2 (PolyBLEP + TC4011BP CMOS XOR Ring Mod)
        let (vco1_sig, vco2_sig) = self.vco.process(
            sr,
            base_freq_vco1,
            base_freq_vco2,
            self.params.vco1_waveform,
            self.params.vco1_pw,
            self.params.vco1_scale,
            self.params.vco2_waveform,
            self.params.vco2_pitch,
            self.params.vco2_scale,
            self.params.master_tune,
            white,
        );

        // 12. VCO Mixer + EXT IN Jack
        let mix_vco1 = vco1_sig * (self.params.vco1_level / 10.0).clamp(0.0, 1.0);
        let mix_vco2 = vco2_sig * (self.params.vco2_level / 10.0).clamp(0.0, 1.0);
        let ext_audio = self.patchbay.read_dest(PatchDest::ExtAudioIn).unwrap_or(0.0);
        let mixer_out = (mix_vco1 + mix_vco2 + ext_audio) * 0.55;

        // 13. Dual KORG35 Series Filter (12dB/oct VC HPF -> 12dB/oct VC LPF)
        let hpf_mod_oct = (self.params.hpf_mg_text / 10.0) * mg_or_total * 3.5
            + (self.params.hpf_eg2_ext / 10.0) * hpf_eg2_or_ext * 5.0;
        let lpf_mod_oct = (self.params.lpf_mg_text / 10.0) * mg_or_total * 3.5
            + (self.params.lpf_eg2_ext / 10.0) * lpf_eg2_or_ext * 5.5;

        let hpf_hz = DualKorg35Filter::hpf_knob_to_hz(self.params.hpf_cutoff, hpf_mod_oct);
        let lpf_hz = DualKorg35Filter::lpf_knob_to_hz(self.params.lpf_cutoff, lpf_mod_oct);

        let vcf_out = self.filter.process(
            mixer_out,
            hpf_hz,
            self.params.hpf_peak,
            lpf_hz,
            self.params.lpf_peak,
            sr,
            self.params.filter_revision,
        );

        // 14. Main VCA (controlled by EG2 + INIT GAIN patch jack)
        let init_gain_cv = self.patchbay.read_dest(PatchDest::VcaInitGain).unwrap_or(0.0);
        let vca_gain = (eg2_norm * self.voice.velocity + init_gain_cv.max(0.0)).clamp(0.0, 1.4);
        let vol_norm = (self.params.master_volume / 10.0).clamp(0.0, 1.0);

        // Keep internal patchbay SIGNAL OUT / PHONES OUT & ESP feedback at true circuit voltage
        let circuit_signal_out = vcf_out * vca_gain * vol_norm;
        self.last_signal_out = circuit_signal_out;
        self.patchbay.source_values[PatchSource::SignalOut as usize] = circuit_signal_out;
        self.patchbay.source_values[PatchSource::PhonesOut as usize] = circuit_signal_out;

        // Apply logarithmic A-taper potentiometer curve for the audio output path
        let audio_taper_vol = vol_norm.powf(1.85) * 0.65;
        let synth_mono = vcf_out * vca_gain * audio_taper_vol;

        // 15. Process through the 5 Ambient Guitar Pedals in user-defined chain order
        let (ped_l, ped_r) =
            self.pedals_dsp
                .process_stereo(synth_mono, synth_mono, &self.pedals_params, sr);

        // 16. Master Output Signal Pad / Trim (-30 dB .. +6 dB) + Soft-Knee Headphone Limiter
        let pad_gain = 10.0_f32.powf(self.params.output_pad_db.clamp(-30.0, 6.0) / 20.0);
        let out_l = Self::soft_limit(ped_l * pad_gain);
        let out_r = Self::soft_limit(ped_r * pad_gain);

        // 17. Update Oscilloscope & Peak Meters
        self.scope_decim += 1;
        if self.scope_decim >= 8 {
            self.scope_decim = 0;
            self.scope_buffer[self.scope_write_idx] = (out_l + out_r) * 0.5;
            self.scope_write_idx = (self.scope_write_idx + 1) % self.scope_buffer.len();
        }
        self.peak_l = (self.peak_l * 0.9992).max(out_l.abs().min(1.0));
        self.peak_r = (self.peak_r * 0.9992).max(out_r.abs().min(1.0));

        (out_l, out_r)
    }

    /// Transparent up to ±0.70, then smooth analog soft-knee saturation approaching ±0.98
    /// so hot KORG35 dual-resonance screams never cause harsh digital clipping in headphones.
    #[inline]
    fn soft_limit(x: f32) -> f32 {
        if !x.is_finite() {
            return 0.0;
        }
        let abs_x = x.abs();
        if abs_x <= 0.70 {
            x
        } else {
            let excess = (abs_x - 0.70) / 0.28;
            let limited = 0.70 + 0.28 * excess.tanh();
            limited.copysign(x)
        }
    }
}

impl AudioProcessor for KaloKorgEngine {
    fn process(&mut self, output_left: &mut [f32], output_right: &mut [f32]) {
        let frames = output_left.len().min(output_right.len());
        for i in 0..frames {
            let (l, r) = self.render_frame();
            output_left[i] = l;
            output_right[i] = r;
        }
    }

    fn handle_midi_event(&mut self, message: [u8; 3]) {
        let status = message[0] & 0xF0;
        let data1 = message[1] & 0x7F;
        let data2 = message[2] & 0x7F;

        match status {
            0x90 => {
                // Note On (velocity 0 = Note Off)
                self.voice.note_on(data1, data2);
            }
            0x80 => {
                // Note Off
                self.voice.note_off(data1);
            }
            0xE0 => {
                // Pitch Bend (14-bit, +/- 2 semitones)
                let bend14 = ((data2 as i32) << 7) | (data1 as i32);
                let norm = ((bend14 - 8192) as f32) / 8192.0;
                self.voice.pitch_bend_semitones = norm * 2.0;
            }
            0xB0 => {
                // Control Change (CC) mapping for physical MIDI controllers
                let val10 = (data2 as f32 / 127.0) * 10.0;
                match data1 {
                    1 => {
                        // Mod Wheel -> Control Wheel & MG FM depth
                        self.params.control_wheel = (data2 as f32 / 127.0) * 10.0 - 5.0;
                        self.params.fm_mg_text = val10 * 0.5;
                    }
                    5 => self.params.portamento_time = val10,
                    7 => self.params.master_volume = val10,
                    71 => self.params.lpf_peak = val10,
                    74 => self.params.lpf_cutoff = val10,
                    75 => self.params.hpf_cutoff = val10,
                    76 => self.params.hpf_peak = val10,
                    73 => self.params.eg2_attack = val10,
                    72 => self.params.eg2_release = val10,
                    120 | 123 => self.voice.all_notes_off(),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn set_parameter(&mut self, param_id: u32, value: f32) {
        match param_id {
            0 => self.params.vco1_waveform = Vco1Waveform::from_index(value as usize),
            1 => self.params.vco1_pw = value.clamp(0.0, 10.0),
            2 => self.params.vco1_scale = Vco1Scale::from_index(value as usize),
            3 => self.params.portamento_time = value.clamp(0.0, 10.0),
            4 => self.params.vco2_waveform = Vco2Waveform::from_index(value as usize),
            5 => self.params.vco2_pitch = value.clamp(-5.0, 5.0),
            6 => self.params.vco2_scale = Vco2Scale::from_index(value as usize),
            7 => self.params.master_tune = value.clamp(-5.0, 5.0),
            8 => self.params.vco1_level = value.clamp(0.0, 10.0),
            9 => self.params.vco2_level = value.clamp(0.0, 10.0),
            10 => self.params.fm_mg_text = value.clamp(0.0, 10.0),
            11 => self.params.fm_eg1_ext = value.clamp(0.0, 10.0),
            12 => self.params.hpf_cutoff = value.clamp(0.0, 10.0),
            13 => self.params.hpf_peak = value.clamp(0.0, 10.0),
            14 => self.params.hpf_mg_text = value.clamp(0.0, 10.0),
            15 => self.params.hpf_eg2_ext = value.clamp(0.0, 10.0),
            16 => self.params.lpf_cutoff = value.clamp(0.0, 10.0),
            17 => self.params.lpf_peak = value.clamp(0.0, 10.0),
            18 => self.params.lpf_mg_text = value.clamp(0.0, 10.0),
            19 => self.params.lpf_eg2_ext = value.clamp(0.0, 10.0),
            20 => self.params.mg_waveform = value.clamp(0.0, 10.0),
            21 => self.params.mg_frequency = value.clamp(0.0, 10.0),
            22 => self.params.eg1_delay = value.clamp(0.0, 10.0),
            23 => self.params.eg1_attack = value.clamp(0.0, 10.0),
            24 => self.params.eg1_release = value.clamp(0.0, 10.0),
            25 => self.params.eg2_hold = value.clamp(0.0, 10.0),
            26 => self.params.eg2_attack = value.clamp(0.0, 10.0),
            27 => self.params.eg2_decay = value.clamp(0.0, 10.0),
            28 => self.params.eg2_sustain = value.clamp(0.0, 10.0),
            29 => self.params.eg2_release = value.clamp(0.0, 10.0),
            30 => self.params.master_volume = value.clamp(0.0, 10.0),
            31 => self.params.esp_signal_level = value.clamp(0.0, 10.0),
            32 => self.params.esp_low_cut = value.clamp(0.0, 10.0),
            33 => self.params.esp_high_cut = value.clamp(0.0, 10.0),
            34 => self.params.esp_cv_adjust = value.clamp(0.0, 10.0),
            35 => self.params.esp_threshold = value.clamp(0.0, 10.0),
            36 => {
                self.params.filter_revision = if value >= 0.5 {
                    FilterRevision::Rev2Ota
                } else {
                    FilterRevision::Rev1Korg35
                };
            }
            37 => self.params.drone_gate = value >= 0.5,
            38 => self.params.output_pad_db = value.clamp(-30.0, 6.0),
            // Pedals
            100 => self.pedals_params.zoom.enabled = value >= 0.5,
            101 => self.pedals_params.zoom.mode = ZoomMode::from_index(value as usize),
            102 => self.pedals_params.zoom.depth = value.clamp(0.0, 10.0),
            103 => self.pedals_params.zoom.rate = value.clamp(0.0, 10.0),
            104 => self.pedals_params.zoom.tone = value.clamp(0.0, 10.0),
            105 => self.pedals_params.zoom.mix = value.clamp(0.0, 10.0),

            110 => self.pedals_params.dd8.enabled = value >= 0.5,
            111 => self.pedals_params.dd8.mode = BossDd8Mode::from_index(value as usize),
            112 => self.pedals_params.dd8.effect_level = value.clamp(0.0, 10.0),
            113 => self.pedals_params.dd8.feedback = value.clamp(0.0, 10.0),
            114 => self.pedals_params.dd8.delay_time = value.clamp(0.0, 10.0),
            115 => self.pedals_params.dd8.mod_depth = value.clamp(0.0, 10.0),

            120 => self.pedals_params.particle.enabled = value >= 0.5,
            121 => self.pedals_params.particle.mode = ParticleMode::from_index(value as usize),
            122 => self.pedals_params.particle.blend = value.clamp(0.0, 10.0),
            123 => self.pedals_params.particle.chop = value.clamp(0.0, 10.0),
            124 => self.pedals_params.particle.pitch_detune = value.clamp(0.0, 10.0),
            125 => self.pedals_params.particle.grain_size = value.clamp(0.0, 10.0),
            126 => self.pedals_params.particle.feedback = value.clamp(0.0, 10.0),
            127 => self.pedals_params.particle.freeze = value >= 0.5,

            130 => self.pedals_params.cloudburst.enabled = value >= 0.5,
            131 => {
                self.pedals_params.cloudburst.ensemble_mode =
                    CloudburstEnsembleMode::from_index(value as usize)
            }
            132 => self.pedals_params.cloudburst.decay = value.clamp(0.0, 10.0),
            133 => self.pedals_params.cloudburst.ensemble = value.clamp(0.0, 10.0),
            134 => self.pedals_params.cloudburst.mix = value.clamp(0.0, 10.0),
            135 => self.pedals_params.cloudburst.tone = value.clamp(0.0, 10.0),
            136 => self.pedals_params.cloudburst.pre_delay = value.clamp(0.0, 10.0),
            137 => self.pedals_params.cloudburst.mod_speed = value.clamp(0.0, 10.0),

            140 => self.pedals_params.chorus.enabled = value >= 0.5,
            141 => self.pedals_params.chorus.mode = ChorusMode::from_index(value as usize),
            142 => self.pedals_params.chorus.rate = value.clamp(0.0, 10.0),
            143 => self.pedals_params.chorus.depth = value.clamp(0.0, 10.0),
            144 => self.pedals_params.chorus.stereo_width = value.clamp(0.0, 10.0),
            145 => self.pedals_params.chorus.warmth = value.clamp(0.0, 10.0),
            146 => self.pedals_params.chorus.mix = value.clamp(0.0, 10.0),
            _ => {}
        }
    }
}

// Re-export PedalId for convenience
pub use crate::params::PedalId as EnginePedalId;
