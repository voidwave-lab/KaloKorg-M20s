use nih_plug::prelude::*;
use serde::{Deserialize, Serialize};

use ms20_core::{
    engine::{KaloKorgEngine, Preset},
    params::{
        BossDd8Mode, ChorusMode, CloudburstEnsembleMode, FilterRevision, ParticleMode, PedalId,
        PedalboardParams, SynthParams, Vco1Scale, Vco1Waveform, Vco2Scale, Vco2Waveform, ZoomMode,
    },
    patchbay::PatchCable,
};

// ============================================================================
// NIH-PLUG ENUM WRAPPERS FOR MS-20 & 5 AMBIENT GUITAR PEDALS
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginPreset {
    ClassicMs20Lead = 0,
    Korg35DualPeakScream = 1,
    Tc4011CmosRingMod = 2,
    CloudburstCathedralEnsemble = 3,
    Particle2GranularCosmos = 4,
    SubBassKorg35Growl = 5,
}

impl Enum for PluginPreset {
    fn variants() -> &'static [&'static str] {
        &[
            "01. Classic MS-20 Lead (Fig.5)",
            "02. KORG35 Dual-Peak Scream",
            "03. TC4011 CMOS XOR Ring Mod",
            "04. Cloudburst Cathedral Ensemble",
            "05. Particle 2 Granular Cosmos",
            "06. Sub-Bass KORG35 Growl",
        ]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 6 {
            0 => Self::ClassicMs20Lead,
            1 => Self::Korg35DualPeakScream,
            2 => Self::Tc4011CmosRingMod,
            3 => Self::CloudburstCathedralEnsemble,
            4 => Self::Particle2GranularCosmos,
            _ => Self::SubBassKorg35Growl,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginVco1Wave {
    Triangle = 0,
    Sawtooth = 1,
    Pulse = 2,
    WhiteNoise = 3,
}

impl Enum for PluginVco1Wave {
    fn variants() -> &'static [&'static str] {
        &["TRI /\\", "SAW |\\", "PW |_|", "WHITE NOISE"]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::Triangle,
            1 => Self::Sawtooth,
            2 => Self::Pulse,
            _ => Self::WhiteNoise,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginVco1Scale {
    Feet32 = 0,
    Feet16 = 1,
    Feet8 = 2,
    Feet4 = 3,
}

impl Enum for PluginVco1Scale {
    fn variants() -> &'static [&'static str] {
        &["32'", "16'", "8'", "4'"]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::Feet32,
            1 => Self::Feet16,
            2 => Self::Feet8,
            _ => Self::Feet4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginVco2Wave {
    Sawtooth = 0,
    Square = 1,
    NarrowPulse = 2,
    RingMod = 3,
}

impl Enum for PluginVco2Wave {
    fn variants() -> &'static [&'static str] {
        &["SAW |\\", "SQR |_|", "NARROW PULSE |_", "RING MOD (TC4011 XOR)"]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::Sawtooth,
            1 => Self::Square,
            2 => Self::NarrowPulse,
            _ => Self::RingMod,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginVco2Scale {
    Feet16 = 0,
    Feet8 = 1,
    Feet4 = 2,
    Feet2 = 3,
}

impl Enum for PluginVco2Scale {
    fn variants() -> &'static [&'static str] {
        &["16'", "8'", "4'", "2'"]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::Feet16,
            1 => Self::Feet8,
            2 => Self::Feet4,
            _ => Self::Feet2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginFilterRev {
    Rev1Korg35 = 0,
    Rev2Ota = 1,
}

impl Enum for PluginFilterRev {
    fn variants() -> &'static [&'static str] {
        &["Rev.1 KORG35 (KLM-128)", "Rev.2 LM13600 OTA (KLM-307)"]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        if index % 2 == 0 {
            Self::Rev1Korg35
        } else {
            Self::Rev2Ota
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginPedalChainOrder {
    StandardChorusFirst = 0,
    ShoegazeEarlyReverb = 1,
    GranularIntoChorus = 2,
    PostDelayChorusToReverb = 3,
}

impl Enum for PluginPedalChainOrder {
    fn variants() -> &'static [&'static str] {
        &[
            "1: CE-2W -> MS-70CDR -> Particle 2 -> DD-8 -> Cloudburst",
            "2: CE-2W -> Cloudburst -> Particle 2 -> DD-8 -> MS-70CDR",
            "3: Particle 2 -> CE-2W -> DD-8 -> MS-70CDR -> Cloudburst",
            "4: MS-70CDR -> Particle 2 -> DD-8 -> CE-2W -> Cloudburst",
        ]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::StandardChorusFirst,
            1 => Self::ShoegazeEarlyReverb,
            2 => Self::GranularIntoChorus,
            _ => Self::PostDelayChorusToReverb,
        }
    }
}

impl PluginPedalChainOrder {
    pub fn to_chain_array(self) -> [PedalId; 5] {
        match self {
            Self::StandardChorusFirst => [
                PedalId::BossCe2wChorus,
                PedalId::ZoomMs70Cdr,
                PedalId::RedPandaParticle2,
                PedalId::BossDd8,
                PedalId::StrymonCloudburst,
            ],
            Self::ShoegazeEarlyReverb => [
                PedalId::BossCe2wChorus,
                PedalId::StrymonCloudburst,
                PedalId::RedPandaParticle2,
                PedalId::BossDd8,
                PedalId::ZoomMs70Cdr,
            ],
            Self::GranularIntoChorus => [
                PedalId::RedPandaParticle2,
                PedalId::BossCe2wChorus,
                PedalId::BossDd8,
                PedalId::ZoomMs70Cdr,
                PedalId::StrymonCloudburst,
            ],
            Self::PostDelayChorusToReverb => [
                PedalId::ZoomMs70Cdr,
                PedalId::RedPandaParticle2,
                PedalId::BossDd8,
                PedalId::BossCe2wChorus,
                PedalId::StrymonCloudburst,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginChorusMode {
    Ce2AnalogStereo = 0,
    DimensionCDc2w = 1,
    TriStereo3Voice = 2,
    JunoSwirl = 3,
}

impl Enum for PluginChorusMode {
    fn variants() -> &'static [&'static str] {
        &[
            "CE-2W Stereo BBD",
            "DC-2W Dimension D",
            "Tri-Stereo 3-Voice",
            "Juno-60 I+II Swirl",
        ]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::Ce2AnalogStereo,
            1 => Self::DimensionCDc2w,
            2 => Self::TriStereo3Voice,
            _ => Self::JunoSwirl,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginZoomMode {
    CoronaTriChorus = 0,
    IceShimmerDelay = 1,
    SpaceHoleReverb = 2,
    LoFiTapeVibrato = 3,
}

impl Enum for PluginZoomMode {
    fn variants() -> &'static [&'static str] {
        &[
            "Stereo Tri-Chorus",
            "Ice Shimmer +12st",
            "SpaceHole Nebula",
            "Lo-Fi Tape Drift",
        ]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 4 {
            0 => Self::CoronaTriChorus,
            1 => Self::IceShimmerDelay,
            2 => Self::SpaceHoleReverb,
            _ => Self::LoFiTapeVibrato,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginDd8Mode {
    Warm = 0,
    Reverse = 1,
    Warp = 2,
    Glitch = 3,
    Tape = 4,
}

impl Enum for PluginDd8Mode {
    fn variants() -> &'static [&'static str] {
        &[
            "WARM (Analog)",
            "REVERSE",
            "WARP (Swell)",
            "GLT (Glitch)",
            "TAPE (Space Echo)",
        ]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 5 {
            0 => Self::Warm,
            1 => Self::Reverse,
            2 => Self::Warp,
            3 => Self::Glitch,
            _ => Self::Tape,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginParticleMode {
    Density = 0,
    LfoTime = 1,
    RandomPitch = 2,
    PitchShift = 3,
    ReverseGrains = 4,
}

impl Enum for PluginParticleMode {
    fn variants() -> &'static [&'static str] {
        &[
            "DENS (Cloud)",
            "LFO (Scrub)",
            "RND (Scatter)",
            "PITCH (+7/+12)",
            "REV (Reverse)",
        ]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 5 {
            0 => Self::Density,
            1 => Self::LfoTime,
            2 => Self::RandomPitch,
            3 => Self::PitchShift,
            _ => Self::ReverseGrains,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginCloudburstMode {
    Off = 0,
    MezzoPiano = 1,
    Forte = 2,
}

impl Enum for PluginCloudburstMode {
    fn variants() -> &'static [&'static str] {
        &["OFF", "MP (Mezzo-Piano)", "FORTE (String Synth)"]
    }
    fn to_index(self) -> usize {
        self as usize
    }
    fn from_index(index: usize) -> Self {
        match index % 3 {
            0 => Self::Off,
            1 => Self::MezzoPiano,
            _ => Self::Forte,
        }
    }
}

// ============================================================================
// DAW PLUGIN PARAMETERS (NO INTERNAL WAV RECORDER — DAW HANDLES RECORDING)
// ============================================================================

pub struct KaloKorgPluginParams {
    // Factory Preset & Global
    pub preset: EnumParam<PluginPreset>,
    pub master_volume: FloatParam,
    pub output_pad_db: FloatParam,
    pub master_tune: FloatParam,
    pub portamento_time: FloatParam,
    pub filter_revision: EnumParam<PluginFilterRev>,
    pub drone_gate: BoolParam,
    pub route_daw_input_to_ext: BoolParam,

    // VCO 1
    pub vco1_waveform: EnumParam<PluginVco1Wave>,
    pub vco1_pw: FloatParam,
    pub vco1_scale: EnumParam<PluginVco1Scale>,

    // VCO 2
    pub vco2_waveform: EnumParam<PluginVco2Wave>,
    pub vco2_pitch: FloatParam,
    pub vco2_scale: EnumParam<PluginVco2Scale>,

    // VCO Mixer & FM
    pub vco1_level: FloatParam,
    pub vco2_level: FloatParam,
    pub fm_mg_text: FloatParam,
    pub fm_eg1_ext: FloatParam,

    // Dual KORG35 Filters (HPF & LPF)
    pub hpf_cutoff: FloatParam,
    pub hpf_peak: FloatParam,
    pub hpf_mg_text: FloatParam,
    pub hpf_eg2_ext: FloatParam,
    pub lpf_cutoff: FloatParam,
    pub lpf_peak: FloatParam,
    pub lpf_mg_text: FloatParam,
    pub lpf_eg2_ext: FloatParam,

    // Modulation Generator (MG LFO)
    pub mg_waveform: FloatParam,
    pub mg_frequency: FloatParam,

    // Envelopes EG 1 (DAR) & EG 2 (HADSR)
    pub eg1_delay: FloatParam,
    pub eg1_attack: FloatParam,
    pub eg1_release: FloatParam,
    pub eg2_hold: FloatParam,
    pub eg2_attack: FloatParam,
    pub eg2_decay: FloatParam,
    pub eg2_sustain: FloatParam,
    pub eg2_release: FloatParam,

    // External Signal Processor (ESP)
    pub esp_signal_level: FloatParam,
    pub esp_low_cut: FloatParam,
    pub esp_high_cut: FloatParam,
    pub esp_cv_adjust: FloatParam,
    pub esp_threshold: FloatParam,

    // Pedalboard Signal Chain Order
    pub pedal_chain_order: EnumParam<PluginPedalChainOrder>,

    // Pedal 1: BOSS CE-2W / DC-2W Stereo Chorus & Dimension Expander
    pub chorus_enabled: BoolParam,
    pub chorus_mode: EnumParam<PluginChorusMode>,
    pub chorus_rate: FloatParam,
    pub chorus_depth: FloatParam,
    pub chorus_stereo_width: FloatParam,
    pub chorus_warmth: FloatParam,
    pub chorus_mix: FloatParam,

    // Pedal 2: Zoom MS-70CDR
    pub zoom_enabled: BoolParam,
    pub zoom_mode: EnumParam<PluginZoomMode>,
    pub zoom_depth: FloatParam,
    pub zoom_rate: FloatParam,
    pub zoom_tone: FloatParam,
    pub zoom_mix: FloatParam,

    // Pedal 3: Red Panda Particle 2
    pub particle_enabled: BoolParam,
    pub particle_mode: EnumParam<PluginParticleMode>,
    pub particle_blend: FloatParam,
    pub particle_chop: FloatParam,
    pub particle_pitch_detune: FloatParam,
    pub particle_grain_size: FloatParam,
    pub particle_feedback: FloatParam,
    pub particle_freeze: BoolParam,

    // Pedal 4: Boss DD-8 Digital Delay
    pub dd8_enabled: BoolParam,
    pub dd8_mode: EnumParam<PluginDd8Mode>,
    pub dd8_effect_level: FloatParam,
    pub dd8_feedback: FloatParam,
    pub dd8_delay_time: FloatParam,
    pub dd8_mod_depth: FloatParam,

    // Pedal 5: Strymon Cloudburst Ambient Reverb
    pub cloudburst_enabled: BoolParam,
    pub cloudburst_ensemble_mode: EnumParam<PluginCloudburstMode>,
    pub cloudburst_decay: FloatParam,
    pub cloudburst_ensemble: FloatParam,
    pub cloudburst_mix: FloatParam,
    pub cloudburst_tone: FloatParam,
    pub cloudburst_pre_delay: FloatParam,
    pub cloudburst_mod_speed: FloatParam,
}

impl Default for KaloKorgPluginParams {
    fn default() -> Self {
        let s = SynthParams::default();
        let p = PedalboardParams::default();
        let r0_10 = FloatRange::Linear { min: 0.0, max: 10.0 };
        let r_bi5 = FloatRange::Linear { min: -5.0, max: 5.0 };
        let smooth = SmoothingStyle::Linear(5.0);

        Self {
            preset: EnumParam::new("Factory Preset", PluginPreset::ClassicMs20Lead)
                .with_id("preset"),
            master_volume: FloatParam::new("Master Volume", s.master_volume, r0_10)
                .with_id("master_vol")
                .with_smoother(smooth),
            output_pad_db: FloatParam::new(
                "Output Signal Pad (Trim)",
                s.output_pad_db,
                FloatRange::Linear {
                    min: -30.0,
                    max: 6.0,
                },
            )
            .with_id("output_pad_db")
            .with_unit(" dB")
            .with_smoother(smooth),
            master_tune: FloatParam::new("Master Tune", s.master_tune, r_bi5)
                .with_id("master_tune")
                .with_smoother(smooth),
            portamento_time: FloatParam::new("Portamento Time", s.portamento_time, r0_10)
                .with_id("portamento")
                .with_smoother(smooth),
            filter_revision: EnumParam::new("VCF Filter IC", PluginFilterRev::Rev1Korg35)
                .with_id("filter_rev"),
            // In DAW plugin mode, default drone_gate to false so playback responds to DAW MIDI notes
            drone_gate: BoolParam::new("Drone Gate Latch", false).with_id("drone_gate"),
            route_daw_input_to_ext: BoolParam::new("Route DAW Input -> EXT/ESP", true)
                .with_id("daw_ext_in"),

            // VCO 1
            vco1_waveform: EnumParam::new("VCO 1 Waveform", PluginVco1Wave::Sawtooth)
                .with_id("vco1_wave"),
            vco1_pw: FloatParam::new("VCO 1 Pulse Width", s.vco1_pw, r0_10)
                .with_id("vco1_pw")
                .with_smoother(smooth),
            vco1_scale: EnumParam::new("VCO 1 Scale", PluginVco1Scale::Feet8)
                .with_id("vco1_scale"),

            // VCO 2
            vco2_waveform: EnumParam::new("VCO 2 Waveform", PluginVco2Wave::Sawtooth)
                .with_id("vco2_wave"),
            vco2_pitch: FloatParam::new("VCO 2 Pitch", s.vco2_pitch, r_bi5)
                .with_id("vco2_pitch")
                .with_smoother(smooth),
            vco2_scale: EnumParam::new("VCO 2 Scale", PluginVco2Scale::Feet8)
                .with_id("vco2_scale"),

            // Mixer & FM
            vco1_level: FloatParam::new("VCO 1 Level", s.vco1_level, r0_10)
                .with_id("vco1_lvl")
                .with_smoother(smooth),
            vco2_level: FloatParam::new("VCO 2 Level", s.vco2_level, r0_10)
                .with_id("vco2_lvl")
                .with_smoother(smooth),
            fm_mg_text: FloatParam::new("FM MG/T.EXT", s.fm_mg_text, r0_10)
                .with_id("fm_mg")
                .with_smoother(smooth),
            fm_eg1_ext: FloatParam::new("FM EG1/EXT", s.fm_eg1_ext, r0_10)
                .with_id("fm_eg1")
                .with_smoother(smooth),

            // Dual KORG35 Filters
            hpf_cutoff: FloatParam::new("HPF Cutoff", s.hpf_cutoff, r0_10)
                .with_id("hpf_cut")
                .with_smoother(smooth),
            hpf_peak: FloatParam::new("HPF Peak (Res)", s.hpf_peak, r0_10)
                .with_id("hpf_peak")
                .with_smoother(smooth),
            hpf_mg_text: FloatParam::new("HPF MG/T.EXT", s.hpf_mg_text, r0_10)
                .with_id("hpf_mg")
                .with_smoother(smooth),
            hpf_eg2_ext: FloatParam::new("HPF EG2/EXT", s.hpf_eg2_ext, r0_10)
                .with_id("hpf_eg2")
                .with_smoother(smooth),
            lpf_cutoff: FloatParam::new("LPF Cutoff", s.lpf_cutoff, r0_10)
                .with_id("lpf_cut")
                .with_smoother(smooth),
            lpf_peak: FloatParam::new("LPF Peak (Res)", s.lpf_peak, r0_10)
                .with_id("lpf_peak")
                .with_smoother(smooth),
            lpf_mg_text: FloatParam::new("LPF MG/T.EXT", s.lpf_mg_text, r0_10)
                .with_id("lpf_mg")
                .with_smoother(smooth),
            lpf_eg2_ext: FloatParam::new("LPF EG2/EXT", s.lpf_eg2_ext, r0_10)
                .with_id("lpf_eg2")
                .with_smoother(smooth),

            // MG
            mg_waveform: FloatParam::new("MG Waveform", s.mg_waveform, r0_10)
                .with_id("mg_wave")
                .with_smoother(smooth),
            mg_frequency: FloatParam::new("MG Frequency", s.mg_frequency, r0_10)
                .with_id("mg_freq")
                .with_smoother(smooth),

            // EG 1 & EG 2
            eg1_delay: FloatParam::new("EG1 Delay", s.eg1_delay, r0_10).with_id("eg1_del"),
            eg1_attack: FloatParam::new("EG1 Attack", s.eg1_attack, r0_10).with_id("eg1_att"),
            eg1_release: FloatParam::new("EG1 Release", s.eg1_release, r0_10).with_id("eg1_rel"),
            eg2_hold: FloatParam::new("EG2 Hold", s.eg2_hold, r0_10).with_id("eg2_hld"),
            eg2_attack: FloatParam::new("EG2 Attack", s.eg2_attack, r0_10).with_id("eg2_att"),
            eg2_decay: FloatParam::new("EG2 Decay", s.eg2_decay, r0_10).with_id("eg2_dec"),
            eg2_sustain: FloatParam::new("EG2 Sustain", s.eg2_sustain, r0_10).with_id("eg2_sus"),
            eg2_release: FloatParam::new("EG2 Release", s.eg2_release, r0_10).with_id("eg2_rel"),

            // ESP
            esp_signal_level: FloatParam::new("ESP Signal Level", s.esp_signal_level, r0_10)
                .with_id("esp_lvl"),
            esp_low_cut: FloatParam::new("ESP Low Cut", s.esp_low_cut, r0_10).with_id("esp_lc"),
            esp_high_cut: FloatParam::new("ESP High Cut", s.esp_high_cut, r0_10).with_id("esp_hc"),
            esp_cv_adjust: FloatParam::new("ESP CV Adjust", s.esp_cv_adjust, r0_10)
                .with_id("esp_cv"),
            esp_threshold: FloatParam::new("ESP Threshold", s.esp_threshold, r0_10)
                .with_id("esp_thr"),

            // Pedalboard Chain
            pedal_chain_order: EnumParam::new(
                "Pedal Chain Order",
                PluginPedalChainOrder::StandardChorusFirst,
            )
            .with_id("pedal_order"),

            // 1. BOSS CE-2W / DC-2W Stereo Chorus
            chorus_enabled: BoolParam::new("CE-2W Chorus Enabled", p.chorus.enabled)
                .with_id("ce2w_on"),
            chorus_mode: EnumParam::new("CE-2W Chorus Mode", PluginChorusMode::Ce2AnalogStereo)
                .with_id("ce2w_mode"),
            chorus_rate: FloatParam::new("CE-2W Rate", p.chorus.rate, r0_10)
                .with_id("ce2w_rate")
                .with_smoother(smooth),
            chorus_depth: FloatParam::new("CE-2W Depth", p.chorus.depth, r0_10)
                .with_id("ce2w_depth")
                .with_smoother(smooth),
            chorus_stereo_width: FloatParam::new("CE-2W Stereo Width", p.chorus.stereo_width, r0_10)
                .with_id("ce2w_width")
                .with_smoother(smooth),
            chorus_warmth: FloatParam::new("CE-2W BBD Warmth", p.chorus.warmth, r0_10)
                .with_id("ce2w_warmth")
                .with_smoother(smooth),
            chorus_mix: FloatParam::new("CE-2W Mix", p.chorus.mix, r0_10)
                .with_id("ce2w_mix")
                .with_smoother(smooth),

            // 2. Zoom MS-70CDR
            zoom_enabled: BoolParam::new("MS-70CDR Enabled", p.zoom.enabled).with_id("zoom_on"),
            zoom_mode: EnumParam::new("MS-70CDR Mode", PluginZoomMode::CoronaTriChorus)
                .with_id("zoom_mode"),
            zoom_depth: FloatParam::new("MS-70CDR Depth", p.zoom.depth, r0_10)
                .with_id("zoom_depth")
                .with_smoother(smooth),
            zoom_rate: FloatParam::new("MS-70CDR Rate", p.zoom.rate, r0_10)
                .with_id("zoom_rate")
                .with_smoother(smooth),
            zoom_tone: FloatParam::new("MS-70CDR Tone", p.zoom.tone, r0_10)
                .with_id("zoom_tone")
                .with_smoother(smooth),
            zoom_mix: FloatParam::new("MS-70CDR Mix", p.zoom.mix, r0_10)
                .with_id("zoom_mix")
                .with_smoother(smooth),

            // 3. Red Panda Particle 2
            particle_enabled: BoolParam::new("Particle 2 Enabled", p.particle.enabled)
                .with_id("part_on"),
            particle_mode: EnumParam::new("Particle 2 Mode", PluginParticleMode::Density)
                .with_id("part_mode"),
            particle_blend: FloatParam::new("Particle 2 Blend", p.particle.blend, r0_10)
                .with_id("part_blend")
                .with_smoother(smooth),
            particle_chop: FloatParam::new("Particle 2 Chop/Density", p.particle.chop, r0_10)
                .with_id("part_chop")
                .with_smoother(smooth),
            particle_pitch_detune: FloatParam::new(
                "Particle 2 Delay/Pitch",
                p.particle.pitch_detune,
                r0_10,
            )
            .with_id("part_pitch")
            .with_smoother(smooth),
            particle_grain_size: FloatParam::new(
                "Particle 2 Grain Size",
                p.particle.grain_size,
                r0_10,
            )
            .with_id("part_grain")
            .with_smoother(smooth),
            particle_feedback: FloatParam::new("Particle 2 Feedback", p.particle.feedback, r0_10)
                .with_id("part_fb")
                .with_smoother(smooth),
            particle_freeze: BoolParam::new("Particle 2 Freeze", p.particle.freeze)
                .with_id("part_freeze"),

            // 4. Boss DD-8
            dd8_enabled: BoolParam::new("BOSS DD-8 Enabled", p.dd8.enabled).with_id("dd8_on"),
            dd8_mode: EnumParam::new("BOSS DD-8 Mode", PluginDd8Mode::Warm).with_id("dd8_mode"),
            dd8_effect_level: FloatParam::new("BOSS DD-8 E.Level", p.dd8.effect_level, r0_10)
                .with_id("dd8_lvl")
                .with_smoother(smooth),
            dd8_feedback: FloatParam::new("BOSS DD-8 Feedback", p.dd8.feedback, r0_10)
                .with_id("dd8_fb")
                .with_smoother(smooth),
            dd8_delay_time: FloatParam::new("BOSS DD-8 Delay Time", p.dd8.delay_time, r0_10)
                .with_id("dd8_time")
                .with_smoother(smooth),
            dd8_mod_depth: FloatParam::new("BOSS DD-8 Mod/Tone", p.dd8.mod_depth, r0_10)
                .with_id("dd8_mod")
                .with_smoother(smooth),

            // 5. Strymon Cloudburst
            cloudburst_enabled: BoolParam::new("Cloudburst Enabled", p.cloudburst.enabled)
                .with_id("cb_on"),
            cloudburst_ensemble_mode: EnumParam::new(
                "Cloudburst Ensemble Mode",
                PluginCloudburstMode::Forte,
            )
            .with_id("cb_mode"),
            cloudburst_decay: FloatParam::new("Cloudburst Decay", p.cloudburst.decay, r0_10)
                .with_id("cb_decay")
                .with_smoother(smooth),
            cloudburst_ensemble: FloatParam::new(
                "Cloudburst Ensemble",
                p.cloudburst.ensemble,
                r0_10,
            )
            .with_id("cb_ens")
            .with_smoother(smooth),
            cloudburst_mix: FloatParam::new("Cloudburst Mix", p.cloudburst.mix, r0_10)
                .with_id("cb_mix")
                .with_smoother(smooth),
            cloudburst_tone: FloatParam::new("Cloudburst Tone", p.cloudburst.tone, r0_10)
                .with_id("cb_tone")
                .with_smoother(smooth),
            cloudburst_pre_delay: FloatParam::new(
                "Cloudburst Pre-Delay",
                p.cloudburst.pre_delay,
                r0_10,
            )
            .with_id("cb_predel")
            .with_smoother(smooth),
            cloudburst_mod_speed: FloatParam::new(
                "Cloudburst Mod Speed",
                p.cloudburst.mod_speed,
                r0_10,
            )
            .with_id("cb_mod")
            .with_smoother(smooth),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PluginSerializedState {
    synth: SynthParams,
    pedals: PedalboardParams,
    cables: Vec<PatchCable>,
}

impl Params for KaloKorgPluginParams {
    fn serialize_fields(&self) -> String {
        let mut engine = KaloKorgEngine::new(48000.0);
        let presets = Preset::factory_presets();
        let mut idx = self.preset.value().to_index();
        self.sync_to_engine(&mut engine, &presets, &mut idx);
        let state = PluginSerializedState {
            synth: engine.params,
            pedals: engine.pedals_params,
            cables: engine.patchbay.cables,
        };
        serde_json::to_string(&state).unwrap_or_default()
    }

    fn deserialize_fields(&self, serialized: &str) {
        if let Ok(state) = serde_json::from_str::<PluginSerializedState>(serialized) {
            let preset = Preset {
                name: "DAW Saved State".to_string(),
                category: "User".to_string(),
                description: String::new(),
                synth: state.synth,
                cables: state.cables,
                pedals: state.pedals,
            };
            self.load_from_preset(&preset);
        }
    }
}

impl KaloKorgPluginParams {
    /// Populate all DAW parameters from a `Preset` snapshot
    pub fn load_from_preset(&self, preset: &Preset) {
        let s = &preset.synth;
        let p = &preset.pedals;

        self.master_volume.set_plain_value(s.master_volume);
        self.master_tune.set_plain_value(s.master_tune);
        self.portamento_time.set_plain_value(s.portamento_time);
        self.filter_revision
            .set_index(s.filter_revision as usize);

        self.vco1_waveform.set_index(s.vco1_waveform as usize);
        self.vco1_pw.set_plain_value(s.vco1_pw);
        self.vco1_scale.set_index(s.vco1_scale as usize);

        self.vco2_waveform.set_index(s.vco2_waveform as usize);
        self.vco2_pitch.set_plain_value(s.vco2_pitch);
        self.vco2_scale.set_index(s.vco2_scale as usize);

        self.vco1_level.set_plain_value(s.vco1_level);
        self.vco2_level.set_plain_value(s.vco2_level);
        self.fm_mg_text.set_plain_value(s.fm_mg_text);
        self.fm_eg1_ext.set_plain_value(s.fm_eg1_ext);

        self.hpf_cutoff.set_plain_value(s.hpf_cutoff);
        self.hpf_peak.set_plain_value(s.hpf_peak);
        self.hpf_mg_text.set_plain_value(s.hpf_mg_text);
        self.hpf_eg2_ext.set_plain_value(s.hpf_eg2_ext);
        self.lpf_cutoff.set_plain_value(s.lpf_cutoff);
        self.lpf_peak.set_plain_value(s.lpf_peak);
        self.lpf_mg_text.set_plain_value(s.lpf_mg_text);
        self.lpf_eg2_ext.set_plain_value(s.lpf_eg2_ext);

        self.mg_waveform.set_plain_value(s.mg_waveform);
        self.mg_frequency.set_plain_value(s.mg_frequency);

        self.eg1_delay.set_plain_value(s.eg1_delay);
        self.eg1_attack.set_plain_value(s.eg1_attack);
        self.eg1_release.set_plain_value(s.eg1_release);
        self.eg2_hold.set_plain_value(s.eg2_hold);
        self.eg2_attack.set_plain_value(s.eg2_attack);
        self.eg2_decay.set_plain_value(s.eg2_decay);
        self.eg2_sustain.set_plain_value(s.eg2_sustain);
        self.eg2_release.set_plain_value(s.eg2_release);

        self.esp_signal_level.set_plain_value(s.esp_signal_level);
        self.esp_low_cut.set_plain_value(s.esp_low_cut);
        self.esp_high_cut.set_plain_value(s.esp_high_cut);
        self.esp_cv_adjust.set_plain_value(s.esp_cv_adjust);
        self.esp_threshold.set_plain_value(s.esp_threshold);

        // Pedals
        self.chorus_enabled.set_plain_value(p.chorus.enabled);
        self.chorus_mode.set_index(p.chorus.mode as usize);
        self.chorus_rate.set_plain_value(p.chorus.rate);
        self.chorus_depth.set_plain_value(p.chorus.depth);
        self.chorus_stereo_width
            .set_plain_value(p.chorus.stereo_width);
        self.chorus_warmth.set_plain_value(p.chorus.warmth);
        self.chorus_mix.set_plain_value(p.chorus.mix);

        self.zoom_enabled.set_plain_value(p.zoom.enabled);
        self.zoom_mode.set_index(p.zoom.mode as usize);
        self.zoom_depth.set_plain_value(p.zoom.depth);
        self.zoom_rate.set_plain_value(p.zoom.rate);
        self.zoom_tone.set_plain_value(p.zoom.tone);
        self.zoom_mix.set_plain_value(p.zoom.mix);

        self.particle_enabled.set_plain_value(p.particle.enabled);
        self.particle_mode.set_index(p.particle.mode as usize);
        self.particle_blend.set_plain_value(p.particle.blend);
        self.particle_chop.set_plain_value(p.particle.chop);
        self.particle_pitch_detune
            .set_plain_value(p.particle.pitch_detune);
        self.particle_grain_size
            .set_plain_value(p.particle.grain_size);
        self.particle_feedback.set_plain_value(p.particle.feedback);
        self.particle_freeze.set_plain_value(p.particle.freeze);

        self.dd8_enabled.set_plain_value(p.dd8.enabled);
        self.dd8_mode.set_index(p.dd8.mode as usize);
        self.dd8_effect_level.set_plain_value(p.dd8.effect_level);
        self.dd8_feedback.set_plain_value(p.dd8.feedback);
        self.dd8_delay_time.set_plain_value(p.dd8.delay_time);
        self.dd8_mod_depth.set_plain_value(p.dd8.mod_depth);

        self.cloudburst_enabled
            .set_plain_value(p.cloudburst.enabled);
        self.cloudburst_ensemble_mode
            .set_index(p.cloudburst.ensemble_mode as usize);
        self.cloudburst_decay.set_plain_value(p.cloudburst.decay);
        self.cloudburst_ensemble
            .set_plain_value(p.cloudburst.ensemble);
        self.cloudburst_mix.set_plain_value(p.cloudburst.mix);
        self.cloudburst_tone.set_plain_value(p.cloudburst.tone);
        self.cloudburst_pre_delay
            .set_plain_value(p.cloudburst.pre_delay);
        self.cloudburst_mod_speed
            .set_plain_value(p.cloudburst.mod_speed);
    }

    /// Synchronize current DAW parameter values into `KaloKorgEngine`
    #[inline]
    pub fn sync_to_engine(
        &self,
        engine: &mut KaloKorgEngine,
        presets: &[Preset],
        last_preset_idx: &mut usize,
    ) {
        let cur_preset_idx = self.preset.value().to_index();
        if cur_preset_idx != *last_preset_idx {
            *last_preset_idx = cur_preset_idx;
            if let Some(p) = presets.get(cur_preset_idx) {
                engine.load_preset(p);
                self.load_from_preset(p);
            }
        }

        let s = &mut engine.params;
        s.master_volume = self.master_volume.value();
        s.output_pad_db = self.output_pad_db.value();
        s.master_tune = self.master_tune.value();
        s.portamento_time = self.portamento_time.value();
        s.filter_revision = match self.filter_revision.value() {
            PluginFilterRev::Rev1Korg35 => FilterRevision::Rev1Korg35,
            PluginFilterRev::Rev2Ota => FilterRevision::Rev2Ota,
        };
        s.drone_gate = self.drone_gate.value();

        s.vco1_waveform = Vco1Waveform::from_index(self.vco1_waveform.value().to_index());
        s.vco1_pw = self.vco1_pw.value();
        s.vco1_scale = Vco1Scale::from_index(self.vco1_scale.value().to_index());

        s.vco2_waveform = Vco2Waveform::from_index(self.vco2_waveform.value().to_index());
        s.vco2_pitch = self.vco2_pitch.value();
        s.vco2_scale = Vco2Scale::from_index(self.vco2_scale.value().to_index());

        s.vco1_level = self.vco1_level.value();
        s.vco2_level = self.vco2_level.value();
        s.fm_mg_text = self.fm_mg_text.value();
        s.fm_eg1_ext = self.fm_eg1_ext.value();

        s.hpf_cutoff = self.hpf_cutoff.value();
        s.hpf_peak = self.hpf_peak.value();
        s.hpf_mg_text = self.hpf_mg_text.value();
        s.hpf_eg2_ext = self.hpf_eg2_ext.value();
        s.lpf_cutoff = self.lpf_cutoff.value();
        s.lpf_peak = self.lpf_peak.value();
        s.lpf_mg_text = self.lpf_mg_text.value();
        s.lpf_eg2_ext = self.lpf_eg2_ext.value();

        s.mg_waveform = self.mg_waveform.value();
        s.mg_frequency = self.mg_frequency.value();

        s.eg1_delay = self.eg1_delay.value();
        s.eg1_attack = self.eg1_attack.value();
        s.eg1_release = self.eg1_release.value();
        s.eg2_hold = self.eg2_hold.value();
        s.eg2_attack = self.eg2_attack.value();
        s.eg2_decay = self.eg2_decay.value();
        s.eg2_sustain = self.eg2_sustain.value();
        s.eg2_release = self.eg2_release.value();

        s.esp_signal_level = self.esp_signal_level.value();
        s.esp_low_cut = self.esp_low_cut.value();
        s.esp_high_cut = self.esp_high_cut.value();
        s.esp_cv_adjust = self.esp_cv_adjust.value();
        s.esp_threshold = self.esp_threshold.value();

        // Pedals & Chain Order
        let pp = &mut engine.pedals_params;
        pp.chain_order = self.pedal_chain_order.value().to_chain_array();

        pp.chorus.enabled = self.chorus_enabled.value();
        pp.chorus.mode = ChorusMode::from_index(self.chorus_mode.value().to_index());
        pp.chorus.rate = self.chorus_rate.value();
        pp.chorus.depth = self.chorus_depth.value();
        pp.chorus.stereo_width = self.chorus_stereo_width.value();
        pp.chorus.warmth = self.chorus_warmth.value();
        pp.chorus.mix = self.chorus_mix.value();

        pp.zoom.enabled = self.zoom_enabled.value();
        pp.zoom.mode = ZoomMode::from_index(self.zoom_mode.value().to_index());
        pp.zoom.depth = self.zoom_depth.value();
        pp.zoom.rate = self.zoom_rate.value();
        pp.zoom.tone = self.zoom_tone.value();
        pp.zoom.mix = self.zoom_mix.value();

        pp.particle.enabled = self.particle_enabled.value();
        pp.particle.mode = ParticleMode::from_index(self.particle_mode.value().to_index());
        pp.particle.blend = self.particle_blend.value();
        pp.particle.chop = self.particle_chop.value();
        pp.particle.pitch_detune = self.particle_pitch_detune.value();
        pp.particle.grain_size = self.particle_grain_size.value();
        pp.particle.feedback = self.particle_feedback.value();
        pp.particle.freeze = self.particle_freeze.value();

        pp.dd8.enabled = self.dd8_enabled.value();
        pp.dd8.mode = BossDd8Mode::from_index(self.dd8_mode.value().to_index());
        pp.dd8.effect_level = self.dd8_effect_level.value();
        pp.dd8.feedback = self.dd8_feedback.value();
        pp.dd8.delay_time = self.dd8_delay_time.value();
        pp.dd8.mod_depth = self.dd8_mod_depth.value();

        pp.cloudburst.enabled = self.cloudburst_enabled.value();
        pp.cloudburst.ensemble_mode =
            CloudburstEnsembleMode::from_index(self.cloudburst_ensemble_mode.value().to_index());
        pp.cloudburst.decay = self.cloudburst_decay.value();
        pp.cloudburst.ensemble = self.cloudburst_ensemble.value();
        pp.cloudburst.mix = self.cloudburst_mix.value();
        pp.cloudburst.tone = self.cloudburst_tone.value();
        pp.cloudburst.pre_delay = self.cloudburst_pre_delay.value();
        pp.cloudburst.mod_speed = self.cloudburst_mod_speed.value();
    }
}
