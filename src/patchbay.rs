use serde::{Deserialize, Serialize};

/// All 20 Output Source Jacks on the Korg MS-20 Right-Hand Patchbay & ESP Panel
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatchSource {
    SignalOut = 0,
    PhonesOut = 1,
    MgTriangleOut = 2,
    MgPulseOut = 3,
    Eg1NormalOut = 4,
    Eg1RevOut = 5,
    Eg2RevOut = 6,
    KeyCvOut = 7,
    TrigOut = 8,
    SampleHoldOut = 9,
    MvcaOut = 10,
    PinkNoiseOut = 11,
    WhiteNoiseOut = 12,
    ControlWheelOut = 13,
    MomentarySwOut = 14,
    EspAmpOut = 15,
    EspBpfOut = 16,
    EspFvCvOut = 17,
    EspEnvOut = 18,
    EspTrigOut = 19,
}

impl PatchSource {
    pub const ALL: [PatchSource; 20] = [
        Self::SignalOut,
        Self::PhonesOut,
        Self::MgTriangleOut,
        Self::MgPulseOut,
        Self::Eg1NormalOut,
        Self::Eg1RevOut,
        Self::Eg2RevOut,
        Self::KeyCvOut,
        Self::TrigOut,
        Self::SampleHoldOut,
        Self::MvcaOut,
        Self::PinkNoiseOut,
        Self::WhiteNoiseOut,
        Self::ControlWheelOut,
        Self::MomentarySwOut,
        Self::EspAmpOut,
        Self::EspBpfOut,
        Self::EspFvCvOut,
        Self::EspEnvOut,
        Self::EspTrigOut,
    ];

    pub fn from_index(idx: usize) -> Self {
        Self::ALL[idx % Self::ALL.len()]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::SignalOut => "SIGNAL OUT",
            Self::PhonesOut => "PHONES OUT",
            Self::MgTriangleOut => "MG TRI /\\",
            Self::MgPulseOut => "MG PLS |_|",
            Self::Eg1NormalOut => "EG1 OUT (+)",
            Self::Eg1RevOut => "EG1 REV (-)",
            Self::Eg2RevOut => "EG2 REV OUT",
            Self::KeyCvOut => "KEY CV OUT",
            Self::TrigOut => "KBD TRIG OUT",
            Self::SampleHoldOut => "S&H OUT",
            Self::MvcaOut => "VCA (PATCH) OUT",
            Self::PinkNoiseOut => "PINK NOISE",
            Self::WhiteNoiseOut => "WHITE NOISE",
            Self::ControlWheelOut => "WHEEL (^)",
            Self::MomentarySwOut => "MOMENTARY SW",
            Self::EspAmpOut => "ESP AMP OUT",
            Self::EspBpfOut => "ESP BPF OUT",
            Self::EspFvCvOut => "ESP F-V CV",
            Self::EspEnvOut => "ESP ENV OUT",
            Self::EspTrigOut => "ESP TRIG OUT",
        }
    }
}

/// All 15 Input Destination Jacks on the Korg MS-20 Right-Hand Patchbay & ESP Panel
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatchDest {
    TotalExt = 0,
    FreqExt = 1,
    ExtAudioIn = 2,
    HpfCutoffExt = 3,
    LpfCutoffExt = 4,
    VcaInitGain = 5,
    Vco1And2CvIn = 6,
    Vco2CvIn = 7,
    Eg1TrigIn = 8,
    MainTrigIn = 9,
    SampleHoldIn = 10,
    SampleHoldClock = 11,
    MvcaIn = 12,
    MvcaCtrl = 13,
    EspSignalIn = 14,
}

impl PatchDest {
    pub const ALL: [PatchDest; 15] = [
        Self::TotalExt,
        Self::FreqExt,
        Self::ExtAudioIn,
        Self::HpfCutoffExt,
        Self::LpfCutoffExt,
        Self::VcaInitGain,
        Self::Vco1And2CvIn,
        Self::Vco2CvIn,
        Self::Eg1TrigIn,
        Self::MainTrigIn,
        Self::SampleHoldIn,
        Self::SampleHoldClock,
        Self::MvcaIn,
        Self::MvcaCtrl,
        Self::EspSignalIn,
    ];

    pub fn from_index(idx: usize) -> Self {
        Self::ALL[idx % Self::ALL.len()]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::TotalExt => "TOTAL IN",
            Self::FreqExt => "FREQ MOD IN",
            Self::ExtAudioIn => "EXT SIG IN",
            Self::HpfCutoffExt => "HPF CUTOFF IN",
            Self::LpfCutoffExt => "LPF CUTOFF IN",
            Self::VcaInitGain => "INIT GAIN IN",
            Self::Vco1And2CvIn => "VCO 1+2 CV IN",
            Self::Vco2CvIn => "VCO 2 CV IN",
            Self::Eg1TrigIn => "EG1 TRIG IN",
            Self::MainTrigIn => "TRIG IN (EG1+2)",
            Self::SampleHoldIn => "S&H IN",
            Self::SampleHoldClock => "S&H CLOCK IN",
            Self::MvcaIn => "VCA (PATCH) IN",
            Self::MvcaCtrl => "VCA CTRL IN",
            Self::EspSignalIn => "ESP SIGNAL IN",
        }
    }
}

/// Virtual 3.5mm Patch Cable connecting one Source Jack to one Destination Jack
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchCable {
    pub source: PatchSource,
    pub dest: PatchDest,
    pub color_idx: u8,
}

/// MS-20 Semi-Modular Patchbay State & Auxiliary Modules (Sample & Hold + Patch MVCA)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Patchbay {
    pub cables: Vec<PatchCable>,
    #[serde(skip)]
    pub source_values: [f32; 20],
    #[serde(skip)]
    sh_held_value: f32,
    #[serde(skip)]
    sh_prev_clock: bool,
}

impl Patchbay {
    pub fn connect(&mut self, source: PatchSource, dest: PatchDest) {
        // On a physical patchbay, each destination jack can only accept one 3.5mm plug at a time
        self.cables.retain(|c| c.dest != dest);
        let color_idx = (self.cables.len() as u8) % 6;
        self.cables.push(PatchCable {
            source,
            dest,
            color_idx,
        });
    }

    pub fn disconnect_dest(&mut self, dest: PatchDest) {
        self.cables.retain(|c| c.dest != dest);
    }

    pub fn clear_all(&mut self) {
        self.cables.clear();
    }

    #[inline]
    pub fn read_dest(&self, dest: PatchDest) -> Option<f32> {
        for cable in &self.cables {
            if cable.dest == dest {
                return Some(self.source_values[cable.source as usize]);
            }
        }
        None
    }

    #[inline]
    pub fn read_dest_with_source(&self, dest: PatchDest) -> Option<(PatchSource, f32)> {
        for cable in &self.cables {
            if cable.dest == dest {
                return Some((cable.source, self.source_values[cable.source as usize]));
            }
        }
        None
    }

    #[inline]
    pub fn is_source_patched(&self, source: PatchSource) -> bool {
        self.cables.iter().any(|c| c.source == source)
    }

    #[inline]
    pub fn is_dest_patched(&self, dest: PatchDest) -> bool {
        self.cables.iter().any(|c| c.dest == dest)
    }

    #[inline]
    pub fn has_any_esp_out_patched(&self) -> bool {
        self.cables.iter().any(|c| {
            matches!(
                c.source,
                PatchSource::EspAmpOut
                    | PatchSource::EspBpfOut
                    | PatchSource::EspFvCvOut
                    | PatchSource::EspEnvOut
                    | PatchSource::EspTrigOut
            )
        })
    }

    /// Update the Sample & Hold (`S&H`) and Auxiliary Patch VCA (`MVCA`) modules.
    /// Per the MS-20 front-panel normalled guide lines:
    /// - `S&H IN` is normalled to `PINK NOISE` if unplugged, and `S&H CLOCK` is normalled to `MG PULSE` if unplugged,
    ///   so patching `S&H OUT` with a single cable immediately produces stepped random modulation.
    /// - `MVCA IN` is normalled to `MG TRIANGLE` if unplugged, and `MVCA CTRL` is normalled to `EG1 NORMAL OUT` if unplugged,
    ///   so patching `MVCA OUT` with a single cable immediately produces envelope-controlled vibrato/modulation.
    #[inline]
    pub fn step_aux_modules(&mut self) {
        // 1. Sample & Hold (S&H): samples S&H IN on positive clock edge at S&H CLOCK
        let default_sh_in = self.source_values[PatchSource::PinkNoiseOut as usize];
        let default_sh_clk = self.source_values[PatchSource::MgPulseOut as usize];
        let sh_in = self.read_dest(PatchDest::SampleHoldIn).unwrap_or(default_sh_in);
        let sh_clk_val = self
            .read_dest(PatchDest::SampleHoldClock)
            .unwrap_or(default_sh_clk);

        let clk_high = sh_clk_val > 0.15;
        if clk_high && !self.sh_prev_clock {
            self.sh_held_value = (sh_in * 1.35).clamp(-1.0, 1.0);
        }
        self.sh_prev_clock = clk_high;
        self.source_values[PatchSource::SampleHoldOut as usize] = self.sh_held_value;

        // 2. Auxiliary Patch VCA (MVCA): IN scaled by CTRL
        let default_mvca_in = self.source_values[PatchSource::MgTriangleOut as usize];
        let default_mvca_ctrl = self.source_values[PatchSource::Eg1NormalOut as usize];
        let mvca_in = self.read_dest(PatchDest::MvcaIn).unwrap_or(default_mvca_in);
        let mvca_ctrl = match self.read_dest_with_source(PatchDest::MvcaCtrl) {
            Some((PatchSource::Eg1RevOut | PatchSource::Eg2RevOut, v)) => {
                // Inverted envelopes (-1..0) act as +5V -> 0V ducking control on the MVCA
                (1.0 + v).clamp(0.0, 1.2)
            }
            Some((_, v)) if v < 0.0 => {
                // Bipolar LFO/audio control on MVCA: rectify/offset so negative half still modulates
                v.abs().clamp(0.0, 1.5)
            }
            Some((_, v)) => v.clamp(0.0, 1.5),
            None => default_mvca_ctrl.clamp(0.0, 1.5),
        };
        self.source_values[PatchSource::MvcaOut as usize] =
            (mvca_in * mvca_ctrl).clamp(-1.5, 1.5);
    }
}
