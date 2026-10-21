/// Convert a 0..10 MS-20 envelope knob to RC time constant tau in seconds (1.5 ms .. 8.0 s)
#[inline]
fn knob_to_tau(knob: f32) -> f32 {
    let norm = (knob.clamp(0.0, 10.0) / 10.0).powf(2.2);
    0.0015 + norm * 4.5
}

/// Convert a 0..10 MS-20 Delay/Hold knob to duration in seconds (0.0 s .. 4.0 s)
#[inline]
fn knob_to_time_sec(knob: f32) -> f32 {
    if knob <= 0.05 {
        0.0
    } else {
        let norm = (knob.clamp(0.0, 10.0) / 10.0).powf(2.0);
        norm * 3.8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eg1Stage {
    Idle,
    Delay,
    Attack,
    SustainHold,
    Release,
}

/// EG 1: DAR (Delay, Attack, Release) Envelope Generator (LM324 / TC4001BP)
/// Provides both Normal (0..+1) and Reverse (0..-1) outputs for the MS-20 Patchbay.
#[derive(Clone, Debug)]
pub struct Eg1Dar {
    pub stage: Eg1Stage,
    pub level: f32,
    delay_remaining_samples: u32,
    prev_gate: bool,
}

impl Default for Eg1Dar {
    fn default() -> Self {
        Self {
            stage: Eg1Stage::Idle,
            level: 0.0,
            delay_remaining_samples: 0,
            prev_gate: false,
        }
    }
}

impl Eg1Dar {
    #[inline]
    pub fn process(
        &mut self,
        gate: bool,
        delay_knob: f32,
        attack_knob: f32,
        release_knob: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        if gate && !self.prev_gate {
            let delay_sec = knob_to_time_sec(delay_knob);
            self.delay_remaining_samples = (delay_sec * sample_rate) as u32;
            if self.delay_remaining_samples > 0 {
                self.stage = Eg1Stage::Delay;
            } else {
                self.stage = Eg1Stage::Attack;
            }
        } else if !gate && self.prev_gate {
            self.stage = Eg1Stage::Release;
        }
        self.prev_gate = gate;

        match self.stage {
            Eg1Stage::Idle => {
                self.level = 0.0;
            }
            Eg1Stage::Delay => {
                if !gate {
                    self.stage = Eg1Stage::Release;
                } else if self.delay_remaining_samples > 0 {
                    self.delay_remaining_samples -= 1;
                } else {
                    self.stage = Eg1Stage::Attack;
                }
            }
            Eg1Stage::Attack => {
                let tau = knob_to_tau(attack_knob);
                let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
                // Analog RC comparator charges toward 1.25x rail and clips at 1.0
                self.level += alpha * (1.22 - self.level);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Eg1Stage::SustainHold;
                }
            }
            Eg1Stage::SustainHold => {
                self.level = 1.0;
                if !gate {
                    self.stage = Eg1Stage::Release;
                }
            }
            Eg1Stage::Release => {
                let tau = knob_to_tau(release_knob);
                let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
                self.level += alpha * (0.0 - self.level);
                if self.level < 0.0005 {
                    self.level = 0.0;
                    self.stage = Eg1Stage::Idle;
                }
            }
        }

        let normal_out = self.level.clamp(0.0, 1.0);
        // MS-20 EG1 Reverse Out: 0V at rest, goes down to -5V (-1.0) during envelope
        let reverse_out = -normal_out;
        (normal_out, reverse_out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eg2Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// EG 2: HADSR (Hold, Attack, Decay, Sustain, Release) Envelope Generator
/// Per Korg MS-20 schematic `1.jpeg` (bottom-left EG-2 section), the HOLD timer extends
/// the effective gate duration after key release (or sustains the gate before release begins).
#[derive(Clone, Debug)]
pub struct Eg2Hadsr {
    pub stage: Eg2Stage,
    pub level: f32,
    hold_remaining_samples: u32,
    prev_gate: bool,
}

impl Default for Eg2Hadsr {
    fn default() -> Self {
        Self {
            stage: Eg2Stage::Idle,
            level: 0.0,
            hold_remaining_samples: 0,
            prev_gate: false,
        }
    }
}

impl Eg2Hadsr {
    #[inline]
    pub fn process(
        &mut self,
        gate: bool,
        hold_knob: f32,
        attack_knob: f32,
        decay_knob: f32,
        sustain_knob: f32,
        release_knob: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        let sustain_level = (sustain_knob / 10.0).clamp(0.0, 1.0);

        if gate && !self.prev_gate {
            let hold_sec = knob_to_time_sec(hold_knob);
            self.hold_remaining_samples = (hold_sec * sample_rate) as u32;
            self.stage = Eg2Stage::Attack;
        }
        self.prev_gate = gate;

        // Effective gate stays high while physical gate is high OR while Hold timer is counting down
        let effective_gate = if gate {
            true
        } else if self.hold_remaining_samples > 0 {
            self.hold_remaining_samples -= 1;
            true
        } else {
            false
        };

        if !effective_gate && self.stage != Eg2Stage::Idle && self.stage != Eg2Stage::Release {
            self.stage = Eg2Stage::Release;
        }

        match self.stage {
            Eg2Stage::Idle => {
                self.level = 0.0;
            }
            Eg2Stage::Attack => {
                let tau = knob_to_tau(attack_knob);
                let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
                self.level += alpha * (1.22 - self.level);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Eg2Stage::Decay;
                }
            }
            Eg2Stage::Decay => {
                let tau = knob_to_tau(decay_knob);
                let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
                self.level += alpha * (sustain_level - self.level);
                if (self.level - sustain_level).abs() < 0.001 {
                    self.level = sustain_level;
                    self.stage = Eg2Stage::Sustain;
                }
            }
            Eg2Stage::Sustain => {
                // Smoothly follow sustain knob movements while note is held
                let alpha = 1.0 - (-1.0 / (0.008 * sample_rate)).exp();
                self.level += alpha * (sustain_level - self.level);
            }
            Eg2Stage::Release => {
                let tau = knob_to_tau(release_knob);
                let alpha = 1.0 - (-1.0 / (tau * sample_rate)).exp();
                self.level += alpha * (0.0 - self.level);
                if self.level < 0.0004 {
                    self.level = 0.0;
                    self.stage = Eg2Stage::Idle;
                }
            }
        }

        let normal_out = self.level.clamp(0.0, 1.0);
        // EG2 REV OUT: 0V to -5V inverted envelope output
        let rev_out = -normal_out;
        (normal_out, rev_out)
    }
}
