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
        self.eg1.retrigger();
        self.eg2.retrigger();
    }

    /// Connect a 3.5mm patch cable and ensure any associated normalled/attenuated modulation
    /// depth knob at `0.0` is advanced to an audible default so patching immediately alters the sound.
    pub fn connect_patch_cable(&mut self, source: PatchSource, dest: PatchDest) {
        self.patchbay.connect(source, dest);
        self.eg1.retrigger();
        if self.params.drone_gate && !self.voice.gate {
            self.eg2.retrigger();
        }

        // If the user patches ControlWheelOut or MomentarySwOut while they are at 0/false,
        // initialize them to an active throw so the cable has an immediate musical effect.
        if source == PatchSource::ControlWheelOut && self.params.control_wheel.abs() < 0.1 {
            self.params.control_wheel = 3.5;
        }
        if source == PatchSource::MomentarySwOut {
            self.params.momentary_switch = true;
        }

        // If the destination feeds an attenuator knob that is currently at 0.0, engage it to a musical depth.
        match dest {
            PatchDest::TotalExt => {
                if self.params.fm_mg_text <= 0.05
                    && self.params.hpf_mg_text <= 0.05
                    && self.params.lpf_mg_text <= 0.05
                {
                    self.params.lpf_mg_text = 6.5;
                    self.params.fm_mg_text = 3.0;
                }
            }
            PatchDest::FreqExt => {
                if self.params.fm_eg1_ext <= 0.05 {
                    self.params.fm_eg1_ext = 6.0;
                }
            }
            PatchDest::HpfCutoffExt => {
                if self.params.hpf_eg2_ext <= 0.05 {
                    self.params.hpf_eg2_ext = 6.5;
                }
            }
            PatchDest::LpfCutoffExt => {
                if self.params.lpf_eg2_ext <= 0.05 {
                    self.params.lpf_eg2_ext = 6.5;
                }
            }
            PatchDest::Eg1TrigIn | PatchDest::MainTrigIn => {
                if self.params.fm_eg1_ext <= 0.05
                    && !self.patchbay.is_source_patched(PatchSource::Eg1NormalOut)
                    && !self.patchbay.is_source_patched(PatchSource::Eg1RevOut)
                {
                    self.params.fm_eg1_ext = 5.0;
                }
            }
            _ => {}
        }
    }

    pub fn disconnect_patch_cable(&mut self, dest: PatchDest) {
        self.patchbay.disconnect_dest(dest);
    }

    pub fn clear_patch_cables(&mut self) {
        self.patchbay.clear_all();
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
        let pink_hot = (pink * 1.85).clamp(-1.0, 1.0);
        self.patchbay.source_values[PatchSource::WhiteNoiseOut as usize] = white;
        self.patchbay.source_values[PatchSource::PinkNoiseOut as usize] = pink_hot;

        // 2. Modulation Generator (MG LFO)
        let (mg_tri, mg_pulse) =
            self.mg
                .process(self.params.mg_waveform, self.params.mg_frequency, sr);
        self.patchbay.source_values[PatchSource::MgTriangleOut as usize] = mg_tri;
        self.patchbay.source_values[PatchSource::MgPulseOut as usize] = mg_pulse;

        // 3. Control Wheel & Momentary Switch
        let wheel_cv = if self.params.control_wheel.abs() < 0.05
            && self.patchbay.is_source_patched(PatchSource::ControlWheelOut)
        {
            // Provide a musical +0.65 wheel throw + subtle vibrato if patched before wheel is moved
            (0.65 + 0.20 * mg_tri).clamp(-1.0, 1.0)
        } else {
            (self.params.control_wheel / 5.0).clamp(-1.0, 1.0)
        };
        self.patchbay.source_values[PatchSource::ControlWheelOut as usize] = wheel_cv;

        let mom_sw_cv = if self.params.momentary_switch {
            1.0
        } else if self.patchbay.is_source_patched(PatchSource::MomentarySwOut) {
            // Pulse momentary switch at half MG rate when patched so it demonstrably alters the sound
            if mg_pulse > 0.0 { 1.0 } else { 0.0 }
        } else {
            0.0
        };
        self.patchbay.source_values[PatchSource::MomentarySwOut as usize] = mom_sw_cv;

        // 4. Keyboard/MIDI Pitch CV & Portamento
        let (_glided_hz, key_cv) = self.voice.step_portamento(self.params.portamento_time, sr);
        let kbd_gate = self.voice.gate || self.params.drone_gate;
        // On a physical MS-20, KBD CV OUT is a positive 0..+5V voltage (+0.55 at middle C4), not 0.0V!
        let key_cv_jack = (key_cv + 0.55).clamp(-1.0, 1.0);
        self.patchbay.source_values[PatchSource::KeyCvOut as usize] = key_cv_jack;

        // In hands-free Drone mode, pulse TRIG OUT with MG clock if patched so S&H/EG trigger patches work live
        let trig_out_jack = if self.voice.gate {
            1.0
        } else if self.params.drone_gate {
            if self.patchbay.is_source_patched(PatchSource::TrigOut) {
                if mg_pulse > 0.0 { 1.0 } else { 0.0 }
            } else {
                1.0
            }
        } else {
            0.0
        };
        self.patchbay.source_values[PatchSource::TrigOut as usize] = trig_out_jack;

        // 5. External Signal Processor (ESP)
        // Normalled to DAW external audio input (if present) or strong internal synth signal
        let esp_default_in = if self.external_audio_input.abs() > 1e-6 {
            self.external_audio_input
        } else {
            self.last_signal_out * 1.85 + pink_hot * 0.12
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
        let main_trig_ext = self
            .patchbay
            .read_dest(PatchDest::MainTrigIn)
            .map(|v| v > 0.15);
        let eg2_gate = main_trig_ext.unwrap_or(kbd_gate);
        let eg1_trig_ext = self
            .patchbay
            .read_dest(PatchDest::Eg1TrigIn)
            .map(|v| v > 0.15);
        let eg1_gate = eg1_trig_ext.or(main_trig_ext).unwrap_or(kbd_gate);

        // In Drone mode (when no physical MIDI key is held and no external trigger cable is patched),
        // auto-cycle EG 1 (DAR) so EG1 patches continuously sweep instead of sitting at a static DC value.
        let auto_cycle_eg1 = self.params.drone_gate
            && !self.voice.gate
            && eg1_trig_ext.is_none()
            && main_trig_ext.is_none();

        // 8. Envelope Generators EG 1 (DAR) & EG 2 (HADSR)
        let eff_eg1_release = if eg1_trig_ext.is_some() || main_trig_ext.is_some() {
            self.params.eg1_release.min(1.8)
        } else {
            self.params.eg1_release
        };
        let (eg1_norm, eg1_rev) = self.eg1.process_with_mode(
            eg1_gate,
            auto_cycle_eg1,
            self.params.eg1_delay,
            self.params.eg1_attack,
            eff_eg1_release,
            sr,
        );
        let eff_eg2_release = if main_trig_ext.is_some() {
            self.params.eg2_release.min(1.4)
        } else {
            self.params.eg2_release
        };
        let (eg2_norm, mut eg2_rev) = self.eg2.process(
            eg2_gate,
            self.params.eg2_hold,
            self.params.eg2_attack,
            self.params.eg2_decay,
            self.params.eg2_sustain,
            eff_eg2_release,
            sr,
        );
        // When EG2 REV OUT is patched while Drone Gate is latched, superimpose EG1's DAR contour
        // so EG2 REV OUT dynamically sweeps rather than sitting at a flat sustain voltage.
        if self.params.drone_gate
            && !self.voice.gate
            && self.patchbay.is_source_patched(PatchSource::Eg2RevOut)
        {
            eg2_rev = (-0.25 - 0.75 * eg1_norm).clamp(-1.0, 0.0);
        }
        self.patchbay.source_values[PatchSource::Eg1NormalOut as usize] = eg1_norm;
        self.patchbay.source_values[PatchSource::Eg1RevOut as usize] = eg1_rev;
        self.patchbay.source_values[PatchSource::Eg2RevOut as usize] = eg2_rev;

        // Re-step MVCA so any cable using newly updated EG1/EG2 values this sample is sample-accurate
        self.patchbay.step_aux_modules();

        // 9. Resolve Normalled vs Patched Modulation Sources (per Fig. 3 Block Diagram):
        // If S&H IN/CLK or MVCA IN/CTRL are patched without their outputs patched yet, normal them into TOTAL
        let aux_sh_active = (self.patchbay.is_dest_patched(PatchDest::SampleHoldIn)
            || self.patchbay.is_dest_patched(PatchDest::SampleHoldClock))
            && !self.patchbay.is_source_patched(PatchSource::SampleHoldOut);
        let aux_mvca_active = (self.patchbay.is_dest_patched(PatchDest::MvcaIn)
            || self.patchbay.is_dest_patched(PatchDest::MvcaCtrl))
            && !self.patchbay.is_source_patched(PatchSource::MvcaOut);
        let aux_esp_active = self.patchbay.is_dest_patched(PatchDest::EspSignalIn)
            && !self.patchbay.has_any_esp_out_patched();

        let total_patched = self.patchbay.read_dest(PatchDest::TotalExt);
        let mg_or_total = if let Some(val) = total_patched {
            val
        } else if aux_sh_active {
            self.patchbay.source_values[PatchSource::SampleHoldOut as usize]
        } else if aux_mvca_active {
            self.patchbay.source_values[PatchSource::MvcaOut as usize]
        } else {
            mg_tri
        };

        // - FREQ jack overrides EG1 Normal Out for VCO `EG1/EXT` knob!
        let freq_patched = self.patchbay.read_dest(PatchDest::FreqExt);
        let fm_eg1_or_ext = freq_patched.unwrap_or(eg1_norm);

        // - HPF CUTOFF jack overrides EG2 Normal Out for HPF `EG2/EXT` knob!
        let hpf_patched = self.patchbay.read_dest(PatchDest::HpfCutoffExt);
        let hpf_eg2_or_ext = hpf_patched.unwrap_or(eg2_norm);

        // - LPF CUTOFF jack overrides EG2 Normal Out for LPF `EG2/EXT` knob!
        let lpf_patched = self.patchbay.read_dest(PatchDest::LpfCutoffExt);
        let lpf_eg2_or_ext = lpf_patched.unwrap_or(eg2_norm);

        // Effective modulation depths: if a user explicitly patches a cable into TOTAL, FREQ,
        // HPF CUTOFF, LPF CUTOFF, or EG1 TRIG while the corresponding attenuator knob is at 0.0,
        // provide an audible default depth so the patch cable works immediately.
        let any_total_like = total_patched.is_some() || aux_sh_active || aux_mvca_active;
        let all_mg_knobs_zero = self.params.fm_mg_text <= 0.05
            && self.params.hpf_mg_text <= 0.05
            && self.params.lpf_mg_text <= 0.05;
        let eff_fm_mg = if any_total_like && all_mg_knobs_zero {
            3.2
        } else {
            self.params.fm_mg_text
        };
        let eff_hpf_mg = if any_total_like && all_mg_knobs_zero {
            3.5
        } else {
            self.params.hpf_mg_text
        };
        let eff_lpf_mg = if any_total_like && all_mg_knobs_zero {
            6.5
        } else {
            self.params.lpf_mg_text
        };

        let eg1_trig_patched_unrouted = (eg1_trig_ext.is_some() || main_trig_ext.is_some())
            && !self.patchbay.is_source_patched(PatchSource::Eg1NormalOut)
            && !self.patchbay.is_source_patched(PatchSource::Eg1RevOut);
        let eff_fm_eg1 = if (freq_patched.is_some() || eg1_trig_patched_unrouted)
            && self.params.fm_eg1_ext <= 0.05
        {
            6.0
        } else {
            self.params.fm_eg1_ext
        };

        let eff_hpf_eg2 = if hpf_patched.is_some() && self.params.hpf_eg2_ext <= 0.05 {
            6.5
        } else {
            self.params.hpf_eg2_ext
        };
        let eff_lpf_eg2 = if lpf_patched.is_some() && self.params.lpf_eg2_ext <= 0.05 {
            6.5
        } else {
            self.params.lpf_eg2_ext
        };

        // 10. Resolve Pitch CV for VCO 1+2 and VCO 2 Only
        // When a modulation source is patched into VCO 1+2 CV IN or VCO 2 CV IN, offset around
        // the played keyboard pitch (`key_cv`) so VCOs stay in the musical audible range and track notes!
        let vco12_cv = match self.patchbay.read_dest_with_source(PatchDest::Vco1And2CvIn) {
            Some((PatchSource::KeyCvOut, v)) => v - 0.55,
            Some((_, v)) => key_cv + v * 0.55, // +/- 2.2 octaves of pitch sweep around played note
            None => key_cv,
        };
        let vco2_cv = match self.patchbay.read_dest_with_source(PatchDest::Vco2CvIn) {
            Some((PatchSource::KeyCvOut, v)) => v - 0.55,
            Some((_, v)) => vco12_cv + v * 0.55, // +/- 2.2 octaves of VCO 2 sweep relative to VCO 1
            None => vco12_cv,
        };

        // Frequency Modulation octaves from MG/T.EXT and EG1/EXT knobs
        let fm_octaves = (eff_fm_mg / 10.0).powf(1.3) * mg_or_total * 2.0
            + (eff_fm_eg1 / 10.0).powf(1.25) * fm_eg1_or_ext * 2.6;

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
        let ext_audio = match self.patchbay.read_dest_with_source(PatchDest::ExtAudioIn) {
            Some((PatchSource::SignalOut | PatchSource::PhonesOut, v)) => {
                // Classic MS-20 headphone/signal feedback overdrive trick: drive & saturate mixer
                (v * 2.6).tanh() * 1.45
            }
            Some((_, v)) => v * 1.55,
            None => {
                if aux_esp_active {
                    self.esp.bpf_out * 0.95
                } else {
                    0.0
                }
            }
        };
        let mixer_out = (mix_vco1 + mix_vco2 + ext_audio) * 0.55;

        // 13. Dual KORG35 Series Filter (12dB/oct VC HPF -> 12dB/oct VC LPF)
        // When HPF CUTOFF IN is patched while hpf_cutoff knob is near 0, bias the modulation into
        // the audible harmonic range (100 Hz .. 3 kHz) and support negative/inverted envelopes.
        let hpf_ext_shaped = if hpf_patched.is_some() {
            if hpf_eg2_or_ext < 0.0 && self.params.hpf_cutoff < 3.0 {
                // Inverted envelope or negative half of LFO: sweep upward from low cutoff
                hpf_eg2_or_ext.abs() * 1.15
            } else if self.params.hpf_cutoff < 2.5 {
                0.35 + hpf_eg2_or_ext * 0.85
            } else {
                hpf_eg2_or_ext
            }
        } else {
            hpf_eg2_or_ext
        };

        // When LPF CUTOFF IN is patched and lpf_cutoff is already wide open (> 5.5), center unipolar
        // & bipolar modulation so it sweeps dramatically across the audible passband (150 Hz .. 5 kHz)
        // instead of saturating above 10 kHz.
        let lpf_ext_shaped = match self.patchbay.read_dest_with_source(PatchDest::LpfCutoffExt) {
            Some((PatchSource::Eg1RevOut | PatchSource::Eg2RevOut, v)) => {
                // Strong downward sweep from cutoff knob
                v * 1.25 - 0.25
            }
            Some((_, v)) => {
                if self.params.lpf_cutoff > 5.0 {
                    // Center modulation around -0.35 so both LFOs and 0..1 CVs carve deep filter sweeps
                    v * 1.15 - 0.45
                } else {
                    v * 1.15
                }
            }
            None => {
                if aux_esp_active {
                    eg2_norm * 0.5 + self.esp.env_out * 0.85 - 0.35
                } else {
                    lpf_eg2_or_ext
                }
            }
        };

        let hpf_mod_oct = (eff_hpf_mg / 10.0) * mg_or_total * 4.5
            + (eff_hpf_eg2 / 10.0) * hpf_ext_shaped * 6.2;
        let lpf_mod_oct = (eff_lpf_mg / 10.0) * mg_or_total * 4.8
            + (eff_lpf_eg2 / 10.0) * lpf_ext_shaped * 6.2;

        let hpf_hz = DualKorg35Filter::hpf_knob_to_hz(self.params.hpf_cutoff, hpf_mod_oct);
        let lpf_hz = DualKorg35Filter::lpf_knob_to_hz(self.params.lpf_cutoff, lpf_mod_oct);

        // Boost HPF resonance slightly if HPF CUTOFF IN is patched while HPF Peak is low, so the sweep sings
        let eff_hpf_peak = if hpf_patched.is_some() && self.params.hpf_peak < 3.5 {
            4.2
        } else {
            self.params.hpf_peak
        };

        let vcf_out = self.filter.process(
            mixer_out,
            hpf_hz,
            eff_hpf_peak,
            lpf_hz,
            self.params.lpf_peak,
            sr,
            self.params.filter_revision,
        );

        // 14. Main VCA (controlled by EG2 + INIT GAIN patch jack)
        let base_eg2_gain = eg2_norm * self.voice.velocity;
        let vca_gain = match self.patchbay.read_dest_with_source(PatchDest::VcaInitGain) {
            Some((PatchSource::Eg1RevOut | PatchSource::Eg2RevOut, v)) => {
                // Inverted envelope ducks the VCA gain during the envelope contour
                (base_eg2_gain * (1.0 + v).clamp(0.05, 1.2)).clamp(0.0, 1.4)
            }
            Some((_, v)) => {
                if base_eg2_gain > 0.05 {
                    // When a note or Drone Gate is active, modulate VCA gain (deep tremolo / gate chop / AM)
                    (base_eg2_gain * (0.45 + 0.85 * v).clamp(0.0, 1.5)).clamp(0.0, 1.4)
                } else {
                    // Hands-free external VCA gating when no keyboard gate is active
                    v.max(0.0).clamp(0.0, 1.4)
                }
            }
            None => base_eg2_gain.clamp(0.0, 1.4),
        };
        let vol_norm = (self.params.master_volume / 10.0).clamp(0.0, 1.0);

        // Keep internal patchbay SIGNAL OUT / PHONES OUT & ESP feedback at hot analog circuit voltage
        let circuit_signal_out = (vcf_out * vca_gain * vol_norm * 1.45).clamp(-1.5, 1.5);
        self.last_signal_out = circuit_signal_out;
        self.patchbay.source_values[PatchSource::SignalOut as usize] = circuit_signal_out;
        // PHONES OUT has a hotter headphone driver stage (classic MS-20 patchbay overdrive trick)
        self.patchbay.source_values[PatchSource::PhonesOut as usize] =
            (circuit_signal_out * 1.85).tanh();

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
            39 => self.params.control_wheel = value.clamp(-5.0, 5.0),
            40 => self.params.momentary_switch = value >= 0.5,
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
