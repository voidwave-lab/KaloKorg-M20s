use serde::{Deserialize, Serialize};

/// VCO 1 Waveform selector (4 positions on front panel)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vco1Waveform {
    Triangle = 0,
    Sawtooth = 1,
    Pulse = 2,
    WhiteNoise = 3,
}

impl Vco1Waveform {
    pub fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => Self::Triangle,
            1 => Self::Sawtooth,
            2 => Self::Pulse,
            _ => Self::WhiteNoise,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Triangle => "TRI /\\",
            Self::Sawtooth => "SAW |\\",
            Self::Pulse => "PW  |_|",
            Self::WhiteNoise => "NOISE",
        }
    }
}

/// VCO 2 Waveform selector (4 positions on front panel, including TC4011BP CMOS XOR Ring Mod)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vco2Waveform {
    Sawtooth = 0,
    Square = 1,
    NarrowPulse = 2,
    RingMod = 3,
}

impl Vco2Waveform {
    pub fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => Self::Sawtooth,
            1 => Self::Square,
            2 => Self::NarrowPulse,
            _ => Self::RingMod,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sawtooth => "SAW  |\\",
            Self::Square => "SQR  |_|",
            Self::NarrowPulse => "PLS  |_",
            Self::RingMod => "RING XOR",
        }
    }
}

/// VCO 1 Scale (32', 16', 8', 4')
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vco1Scale {
    Feet32 = 0,
    Feet16 = 1,
    Feet8 = 2,
    Feet4 = 3,
}

impl Vco1Scale {
    pub fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => Self::Feet32,
            1 => Self::Feet16,
            2 => Self::Feet8,
            _ => Self::Feet4,
        }
    }

    pub fn octave_multiplier(&self) -> f32 {
        match self {
            Self::Feet32 => 0.25,
            Self::Feet16 => 0.5,
            Self::Feet8 => 1.0,
            Self::Feet4 => 2.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Feet32 => "32'",
            Self::Feet16 => "16'",
            Self::Feet8 => "8'",
            Self::Feet4 => "4'",
        }
    }
}

/// VCO 2 Scale (16', 8', 4', 2')
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vco2Scale {
    Feet16 = 0,
    Feet8 = 1,
    Feet4 = 2,
    Feet2 = 3,
}

impl Vco2Scale {
    pub fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => Self::Feet16,
            1 => Self::Feet8,
            2 => Self::Feet4,
            _ => Self::Feet2,
        }
    }

    pub fn octave_multiplier(&self) -> f32 {
        match self {
            Self::Feet16 => 0.5,
            Self::Feet8 => 1.0,
            Self::Feet4 => 2.0,
            Self::Feet2 => 4.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Feet16 => "16'",
            Self::Feet8 => "8'",
            Self::Feet4 => "4'",
            Self::Feet2 => "2'",
        }
    }
}

/// Filter IC revision model:
/// - Rev1Korg35: KLM-128 board discrete KORG35 SIP-8 hybrid IC with asymmetric diode/transistor clipping
/// - Rev2Ota: KLM-307/3162 board LM13600/NJM13700 dual monolithic OTA with linearizing diodes & symmetric saturation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterRevision {
    Rev1Korg35 = 0,
    Rev2Ota = 1,
}

impl FilterRevision {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rev1Korg35 => "Rev.1 KORG35 (KLM-128)",
            Self::Rev2Ota => "Rev.2 LM13600 OTA (KLM-307)",
        }
    }
}

/// Complete MS-20 Front Panel Parameters (all continuous knobs scaled 0.0..=10.0 like physical panel)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthParams {
    // Column 1: VCO 1 & Portamento
    pub vco1_waveform: Vco1Waveform,
    pub vco1_pw: f32,        // 0.0 .. 10.0 (maps to 50%..95% pulse width)
    pub vco1_scale: Vco1Scale,
    pub portamento_time: f32, // 0.0 .. 10.0

    // Column 2: VCO 2 & Master Tune
    pub vco2_waveform: Vco2Waveform,
    pub vco2_pitch: f32,     // -5.0 .. +5.0 (maps to -12 .. +12 semitones)
    pub vco2_scale: Vco2Scale,
    pub master_tune: f32,    // -5.0 .. +5.0 (maps to -100 .. +100 cents)

    // Column 3: VCO Mixer & Freq Mod
    pub vco1_level: f32,     // 0.0 .. 10.0
    pub vco2_level: f32,     // 0.0 .. 10.0
    pub fm_mg_text: f32,     // 0.0 .. 10.0 (MG/T.EXT frequency modulation depth)
    pub fm_eg1_ext: f32,     // 0.0 .. 10.0 (EG1/EXT frequency modulation depth)

    // Column 4: VC HPF & HPF Mod
    pub hpf_cutoff: f32,     // 0.0 .. 10.0
    pub hpf_peak: f32,       // 0.0 .. 10.0
    pub hpf_mg_text: f32,    // 0.0 .. 10.0
    pub hpf_eg2_ext: f32,    // 0.0 .. 10.0

    // Column 5: VC LPF & LPF Mod
    pub lpf_cutoff: f32,     // 0.0 .. 10.0
    pub lpf_peak: f32,       // 0.0 .. 10.0
    pub lpf_mg_text: f32,    // 0.0 .. 10.0
    pub lpf_eg2_ext: f32,    // 0.0 .. 10.0

    // Column 6: MG & EG 1 (DAR)
    pub mg_waveform: f32,    // 0.0 .. 10.0 (0=Down Saw, 5=Triangle/Square, 10=Up Saw)
    pub mg_frequency: f32,   // 0.0 .. 10.0 (0.1 Hz .. 32 Hz)
    pub eg1_delay: f32,      // 0.0 .. 10.0
    pub eg1_attack: f32,     // 0.0 .. 10.0
    pub eg1_release: f32,    // 0.0 .. 10.0

    // Column 7: EG 2 (HADSR)
    pub eg2_hold: f32,       // 0.0 .. 10.0
    pub eg2_attack: f32,     // 0.0 .. 10.0
    pub eg2_decay: f32,      // 0.0 .. 10.0
    pub eg2_sustain: f32,    // 0.0 .. 10.0
    pub eg2_release: f32,    // 0.0 .. 10.0

    // Right Panel: Master Volume & ESP (External Signal Processor)
    pub master_volume: f32,  // 0.0 .. 10.0
    pub esp_signal_level: f32,   // 0.0 .. 10.0
    pub esp_low_cut: f32,        // 0.0 .. 10.0
    pub esp_high_cut: f32,       // 0.0 .. 10.0
    pub esp_cv_adjust: f32,      // 0.0 .. 10.0
    pub esp_threshold: f32,      // 0.0 .. 10.0

    // Control Wheel & Momentary Switch (for patchbay sources)
    pub control_wheel: f32,      // -5.0 .. +5.0
    pub momentary_switch: bool,

    // Hardware Circuit Revision Switch (KORG35 vs LM13600 OTA)
    pub filter_revision: FilterRevision,

    // Drone / Hold Latched Gate for hands-free ambient exploration
    pub drone_gate: bool,

    // Master Output Signal Pad / Trim in dB (-30.0 dB .. +6.0 dB, default -6.0 dB)
    // Protects headphones/speakers while preserving hot internal KORG35 & pedal saturation.
    #[serde(default = "default_output_pad_db")]
    pub output_pad_db: f32,
}

fn default_output_pad_db() -> f32 {
    -6.0
}

impl Default for SynthParams {
    fn default() -> Self {
        // Matches Figure 5 standard setting from Korg MS-20 manual
        Self {
            vco1_waveform: Vco1Waveform::Sawtooth,
            vco1_pw: 0.0,
            vco1_scale: Vco1Scale::Feet8,
            portamento_time: 0.0,

            vco2_waveform: Vco2Waveform::Sawtooth,
            vco2_pitch: 0.05, // Slight warm analog detune
            vco2_scale: Vco2Scale::Feet8,
            master_tune: 0.0,

            vco1_level: 10.0,
            vco2_level: 8.5,
            fm_mg_text: 0.0,
            fm_eg1_ext: 0.0,

            hpf_cutoff: 0.5,
            hpf_peak: 1.5,
            hpf_mg_text: 0.0,
            hpf_eg2_ext: 0.0,

            lpf_cutoff: 6.8,
            lpf_peak: 3.5,
            lpf_mg_text: 0.0,
            lpf_eg2_ext: 4.5,

            mg_waveform: 5.0,
            mg_frequency: 4.5,
            eg1_delay: 0.0,
            eg1_attack: 2.2,
            eg1_release: 4.2,

            eg2_hold: 0.0,
            eg2_attack: 0.3,
            eg2_decay: 4.5,
            eg2_sustain: 6.0,
            eg2_release: 3.5,

            master_volume: 6.5,
            esp_signal_level: 5.0,
            esp_low_cut: 2.0,
            esp_high_cut: 8.0,
            esp_cv_adjust: 5.0,
            esp_threshold: 4.0,

            control_wheel: 0.0,
            momentary_switch: false,
            filter_revision: FilterRevision::Rev1Korg35,
            drone_gate: false,
            output_pad_db: default_output_pad_db(),
        }
    }
}

/// Pedal IDs for ordering the 5 guitar pedals in the effect chain
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PedalId {
    ZoomMs70Cdr = 0,
    BossDd8 = 1,
    RedPandaParticle2 = 2,
    StrymonCloudburst = 3,
    BossCe2wChorus = 4,
}

impl PedalId {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ZoomMs70Cdr => "ZOOM MS-70CDR",
            Self::BossDd8 => "BOSS DD-8",
            Self::RedPandaParticle2 => "RED PANDA PARTICLE 2",
            Self::StrymonCloudburst => "STRYMON CLOUDBURST",
            Self::BossCe2wChorus => "BOSS CE-2W / DC-2W",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::ZoomMs70Cdr => "MultiStomp Chorus / Delay / Reverb",
            Self::BossDd8 => "Digital Delay",
            Self::RedPandaParticle2 => "Granular Delay / Pitch Shifter",
            Self::StrymonCloudburst => "Ambient Reverb + Ensemble",
            Self::BossCe2wChorus => "Waza Stereo Chorus & Dimension Expander",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChorusMode {
    Ce2AnalogStereo = 0,
    DimensionCDc2w = 1,
    TriStereo3Voice = 2,
    JunoSwirl = 3,
}

impl ChorusMode {
    pub fn from_index(i: usize) -> Self {
        match i % 4 {
            0 => Self::Ce2AnalogStereo,
            1 => Self::DimensionCDc2w,
            2 => Self::TriStereo3Voice,
            _ => Self::JunoSwirl,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Ce2AnalogStereo => "CE-1/CE-2 Stereo BBD",
            Self::DimensionCDc2w => "DC-2W Dimension D (3D)",
            Self::TriStereo3Voice => "Tri-Stereo 3-Voice",
            Self::JunoSwirl => "Juno I+II BBD Swirl",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoomMode {
    CoronaTriChorus = 0,
    IceShimmerDelay = 1,
    SpaceHoleReverb = 2,
    LoFiTapeVibrato = 3,
}

impl ZoomMode {
    pub fn from_index(i: usize) -> Self {
        match i % 4 {
            0 => Self::CoronaTriChorus,
            1 => Self::IceShimmerDelay,
            2 => Self::SpaceHoleReverb,
            _ => Self::LoFiTapeVibrato,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::CoronaTriChorus => "Stereo Tri-Chorus",
            Self::IceShimmerDelay => "Ice Shimmer +12st",
            Self::SpaceHoleReverb => "SpaceHole Nebula",
            Self::LoFiTapeVibrato => "Lo-Fi Tape Drift",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossDd8Mode {
    Warm = 0,
    Reverse = 1,
    Warp = 2,
    Glitch = 3,
    Tape = 4,
}

impl BossDd8Mode {
    pub fn from_index(i: usize) -> Self {
        match i % 5 {
            0 => Self::Warm,
            1 => Self::Reverse,
            2 => Self::Warp,
            3 => Self::Glitch,
            _ => Self::Tape,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Warm => "WARM (Analog)",
            Self::Reverse => "REVERSE",
            Self::Warp => "WARP (Swell)",
            Self::Glitch => "GLT (Glitch)",
            Self::Tape => "TAPE (Echo)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleMode {
    Density = 0,
    LfoTime = 1,
    RandomPitch = 2,
    PitchShift = 3,
    ReverseGrains = 4,
}

impl ParticleMode {
    pub fn from_index(i: usize) -> Self {
        match i % 5 {
            0 => Self::Density,
            1 => Self::LfoTime,
            2 => Self::RandomPitch,
            3 => Self::PitchShift,
            _ => Self::ReverseGrains,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Density => "DENS (Cloud)",
            Self::LfoTime => "LFO (Scrub)",
            Self::RandomPitch => "RND (Scatter)",
            Self::PitchShift => "PITCH (+7/+12)",
            Self::ReverseGrains => "REV (Backwards)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudburstEnsembleMode {
    Off = 0,
    MezzoPiano = 1,
    Forte = 2,
}

impl CloudburstEnsembleMode {
    pub fn from_index(i: usize) -> Self {
        match i % 3 {
            0 => Self::Off,
            1 => Self::MezzoPiano,
            _ => Self::Forte,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::MezzoPiano => "MP (Subtle Pad)",
            Self::Forte => "FORTE (Lush Strings)",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BossCe2wParams {
    pub enabled: bool,
    pub mode: ChorusMode,
    pub rate: f32,         // 0..10 (LFO Rate)
    pub depth: f32,        // 0..10 (Modulation Depth)
    pub stereo_width: f32, // 0..10 (Mono-to-Stereo Haas & Phase Decorrelation)
    pub warmth: f32,       // 0..10 (MN3007 BBD Analog Saturation & Filter)
    pub mix: f32,          // 0..10 (E.Level)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZoomMs70Params {
    pub enabled: bool,
    pub mode: ZoomMode,
    pub depth: f32, // 0..10
    pub rate: f32,  // 0..10
    pub tone: f32,  // 0..10
    pub mix: f32,   // 0..10
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BossDd8Params {
    pub enabled: bool,
    pub mode: BossDd8Mode,
    pub effect_level: f32, // 0..10
    pub feedback: f32,     // 0..10
    pub delay_time: f32,   // 0..10 (maps to 25ms .. 1500ms)
    pub mod_depth: f32,    // 0..10
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Particle2Params {
    pub enabled: bool,
    pub mode: ParticleMode,
    pub blend: f32,      // 0..10
    pub chop: f32,       // 0..10 (grain density / freeze threshold)
    pub pitch_detune: f32, // 0..10 (pitch shift / delay)
    pub grain_size: f32, // 0..10 (10ms .. 450ms)
    pub feedback: f32,   // 0..10
    pub freeze: bool,    // Dedicated FREEZE footswitch
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudburstParams {
    pub enabled: bool,
    pub ensemble_mode: CloudburstEnsembleMode,
    pub decay: f32,      // 0..10 (0.5s .. 40s)
    pub ensemble: f32,   // 0..10 (harmonic string resonator drive)
    pub mix: f32,        // 0..10
    pub tone: f32,       // 0..10
    pub pre_delay: f32,  // 0..10
    pub mod_speed: f32,  // 0..10
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PedalboardParams {
    pub chain_order: [PedalId; 5],
    pub chorus: BossCe2wParams,
    pub zoom: ZoomMs70Params,
    pub dd8: BossDd8Params,
    pub particle: Particle2Params,
    pub cloudburst: CloudburstParams,
}

impl Default for PedalboardParams {
    fn default() -> Self {
        Self {
            chain_order: [
                PedalId::BossCe2wChorus,
                PedalId::ZoomMs70Cdr,
                PedalId::RedPandaParticle2,
                PedalId::BossDd8,
                PedalId::StrymonCloudburst,
            ],
            chorus: BossCe2wParams {
                enabled: true,
                mode: ChorusMode::DimensionCDc2w,
                rate: 4.2,
                depth: 7.5,
                stereo_width: 9.0,
                warmth: 6.5,
                mix: 8.0,
            },
            zoom: ZoomMs70Params {
                enabled: true,
                mode: ZoomMode::IceShimmerDelay,
                depth: 7.5,
                rate: 4.5,
                tone: 7.5,
                mix: 6.2,
            },
            dd8: BossDd8Params {
                enabled: true,
                mode: BossDd8Mode::Tape,
                effect_level: 6.5,
                feedback: 6.8,
                delay_time: 5.5,
                mod_depth: 5.5,
            },
            particle: Particle2Params {
                enabled: true,
                mode: ParticleMode::RandomPitch,
                blend: 5.8,
                chop: 6.8,
                pitch_detune: 7.5,
                grain_size: 5.5,
                feedback: 6.2,
                freeze: false,
            },
            cloudburst: CloudburstParams {
                enabled: true,
                ensemble_mode: CloudburstEnsembleMode::Forte,
                decay: 8.4,
                ensemble: 7.8,
                mix: 6.8,
                tone: 7.2,
                pre_delay: 3.0,
                mod_speed: 5.5,
            },
        }
    }
}
