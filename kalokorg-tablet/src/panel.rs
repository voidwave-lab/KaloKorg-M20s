//! Korg MS-20s Front Panel, Patchbay & Effects Section (`kalokorg-tablet/src/panel.rs`).
//!
//! Renders the top 65% of the tablet viewport in landscape orientation:
//! - Column 1: VCO 1 & VCO 2 (Waveform, Scale/Frequency, Pulse Width, Pitch Detune, Mixer Levels, FM)
//! - Column 2: Dual Series KORG35 VCF (HPF Cutoff & Peak, LPF Cutoff & Peak, Filter Mod, Rev.1/Rev.2 IC)
//! - Column 3: Envelope Sliders (EG1: Delay, Attack, Release + MG LFO; EG2: Hold, Attack, Decay, Sustain, Release)
//! - Column 4: Semi-Modular Patchbay & Effects Chain (Delay, Distortion/Saturator, Chorus, Cloudburst & Output Volume)

use crossbeam_channel::Sender;
use egui::{Button, Color32, Pos2, RichText, Sense, Slider, Stroke, Ui, Vec2};
use korg_ms20::{
    engine::Preset,
    params::{
        BossDd8Mode, ChorusMode, FilterRevision, PedalboardParams, SynthParams, Vco1Scale,
        Vco1Waveform, Vco2Scale, Vco2Waveform,
    },
    patchbay::{PatchCable, PatchDest, PatchSource},
};

use crate::audio::{param_ids, ControlEvent};

/// Complete UI-side mirror of the MS-20 synthesizer, patchbay, and effects parameters.
pub struct SynthPanel {
    pub synth: SynthParams,
    pub pedals: PedalboardParams,
    pub saturator_drive: f32,
    pub active_preset_idx: usize,
    pub presets: Vec<Preset>,
    pub cables: Vec<PatchCable>,
    pub selected_patch_src: usize,
    pub selected_patch_dst: usize,
}

impl Default for SynthPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthPanel {
    pub fn new() -> Self {
        let presets = Preset::factory_presets();
        let mut synth = SynthParams::default();
        synth.drone_gate = false;
        Self {
            synth,
            pedals: PedalboardParams::default(),
            saturator_drive: 2.5,
            active_preset_idx: 0,
            presets,
            cables: Vec::new(),
            selected_patch_src: PatchSource::SampleHoldOut as usize,
            selected_patch_dst: PatchDest::TotalExt as usize,
        }
    }

    /// Load a factory preset into the UI state and dispatch the lock-free preset event to the audio thread.
    pub fn select_preset(&mut self, idx: usize, tx: &Sender<ControlEvent>) {
        if let Some(p) = self.presets.get(idx) {
            let pad = self.synth.output_pad_db;
            let drone = self.synth.drone_gate;
            self.synth = p.synth.clone();
            self.synth.output_pad_db = pad;
            self.synth.drone_gate = drone;
            self.pedals = p.pedals.clone();
            self.cables = p.cables.clone();
            self.active_preset_idx = idx;
            let _ = tx.try_send(ControlEvent::ParameterChange {
                id: param_ids::LOAD_FACTORY_PRESET,
                value: idx as f32,
            });
        }
    }

    /// Connect a 3.5mm patch cable between `src` and `dst` and notify the audio thread.
    pub fn connect_patch(
        &mut self,
        src: PatchSource,
        dst: PatchDest,
        tx: &Sender<ControlEvent>,
    ) {
        self.cables.retain(|c| c.dest != dst);
        let color_idx = (self.cables.len() as u8) % 6;
        self.cables.push(PatchCable {
            source: src,
            dest: dst,
            color_idx,
        });
        let encoded = (src as usize) * 100 + (dst as usize);
        let _ = tx.try_send(ControlEvent::ParameterChange {
            id: param_ids::PATCH_CONNECT_ENCODED,
            value: encoded as f32,
        });
    }

    /// Clear all 3.5mm patchbay cables and notify the audio thread.
    pub fn clear_patches(&mut self, tx: &Sender<ControlEvent>) {
        self.cables.clear();
        let _ = tx.try_send(ControlEvent::ParameterChange {
            id: param_ids::PATCH_CLEAR_ALL,
            value: 1.0,
        });
    }

    /// Render a touch-friendly MS-20 hardware rotary knob + interactive drag control.
    fn ms20_knob(
        ui: &mut Ui,
        label: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        param_id: u32,
        tx: &Sender<ControlEvent>,
    ) -> bool {
        let min_v = *range.start();
        let max_v = *range.end();
        let span = (max_v - min_v).max(f32::EPSILON);
        let mut changed = false;

        ui.horizontal(|row| {
            // Paint rotary dial indicator
            let (dial_rect, dial_resp) =
                row.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::click_and_drag());
            if dial_resp.dragged() {
                let delta = -dial_resp.drag_delta().y * 0.04 * span;
                let new_v = (*value + delta).clamp(min_v, max_v);
                if (new_v - *value).abs() > 1e-4 {
                    *value = new_v;
                    changed = true;
                }
            }

            let norm = ((*value - min_v) / span).clamp(0.0, 1.0);
            // Sweep from -135 deg to +135 deg (in radians, -2.356 .. +2.356, where 0 is straight up)
            let angle = -std::f32::consts::FRAC_PI_2
                + (-0.75 * std::f32::consts::PI + norm * 1.5 * std::f32::consts::PI);
            let center = dial_rect.center();
            let radius = 11.5;

            row.painter()
                .circle_filled(center, radius, Color32::from_rgb(28, 30, 35));
            row.painter().circle_stroke(
                center,
                radius,
                Stroke::new(1.5, Color32::from_rgb(185, 190, 198)),
            );
            let tip = Pos2::new(
                center.x + angle.cos() * (radius - 2.0),
                center.y + angle.sin() * (radius - 2.0),
            );
            row.painter().line_segment(
                [center, tip],
                Stroke::new(2.0, Color32::from_rgb(245, 155, 40)),
            );

            // Touch slider alongside dial for fast single-tap / drag precision on tablets
            let slider_label = format!("{}: {:.1}", label, *value);
            if row
                .add(Slider::new(value, range).text(slider_label).show_value(false))
                .changed()
            {
                changed = true;
            }
        });

        if changed {
            let _ = tx.try_send(ControlEvent::ParameterChange {
                id: param_id,
                value: *value,
            });
        }
        changed
    }

    /// Render the complete MS-20 Top Section (65% viewport height).
    pub fn ui(&mut self, ui: &mut Ui, tx: &Sender<ControlEvent>, audio_status: &str) {
        // 1. Top Header Bar: Title, Preset Selector, Filter IC Revision, Drone Gate, Audio Latency Badge
        ui.horizontal(|bar| {
            bar.label(
                RichText::new("KORG MS-20s TABLET SYNTHESIZER")
                    .size(15.0)
                    .strong()
                    .color(Color32::from_rgb(245, 165, 50)),
            );
            bar.separator();

            // Preset cycle button
            let preset_name = self
                .presets
                .get(self.active_preset_idx)
                .map(|p| p.name.as_str())
                .unwrap_or("01. Classic MS-20 Lead");
            if bar
                .add(
                    Button::new(RichText::new(format!("Preset: {}", preset_name)).size(12.0))
                        .fill(Color32::from_rgb(42, 48, 58)),
                )
                .clicked()
            {
                let next_idx = (self.active_preset_idx + 1) % self.presets.len().max(1);
                self.select_preset(next_idx, tx);
            }

            // Filter IC Revision toggle (Rev.1 KORG35 vs Rev.2 LM13600 OTA)
            let rev_text = match self.synth.filter_revision {
                FilterRevision::Rev1Korg35 => "IC: Rev.1 KORG35",
                FilterRevision::Rev2Ota => "IC: Rev.2 OTA",
            };
            if bar
                .add(
                    Button::new(RichText::new(rev_text).size(12.0))
                        .fill(Color32::from_rgb(55, 42, 32)),
                )
                .clicked()
            {
                self.synth.filter_revision = match self.synth.filter_revision {
                    FilterRevision::Rev1Korg35 => FilterRevision::Rev2Ota,
                    FilterRevision::Rev2Ota => FilterRevision::Rev1Korg35,
                };
                let val = match self.synth.filter_revision {
                    FilterRevision::Rev1Korg35 => 0.0,
                    FilterRevision::Rev2Ota => 1.0,
                };
                let _ = tx.try_send(ControlEvent::ParameterChange {
                    id: param_ids::FILTER_REVISION,
                    value: val,
                });
            }

            // Hands-Free Drone Gate toggle
            let drone_label = if self.synth.drone_gate {
                "DRONE GATE: ON"
            } else {
                "DRONE GATE: OFF"
            };
            if bar
                .selectable_label(
                    self.synth.drone_gate,
                    RichText::new(drone_label).size(12.0).strong(),
                )
                .clicked()
            {
                self.synth.drone_gate = !self.synth.drone_gate;
                let _ = tx.try_send(ControlEvent::ParameterChange {
                    id: param_ids::DRONE_GATE,
                    value: if self.synth.drone_gate { 1.0 } else { 0.0 },
                });
            }

            bar.separator();
            bar.label(
                RichText::new(audio_status)
                    .size(11.0)
                    .monospace()
                    .color(Color32::from_rgb(125, 215, 145)),
            );
        });

        ui.separator();

        // 2. Main 4-Column MS-20 Module Grid
        ui.columns(4, |cols| {
            // ================================================================
            // COLUMN 1: VCO 1 & VCO 2 (Oscillators, Mixer & Frequency Mod)
            // ================================================================
            cols[0].group(|col| {
                col.label(
                    RichText::new("VCO 1 & VCO 2 / MIXER")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(240, 165, 55)),
                );

                // VCO 1 Waveform & Scale selectors
                col.horizontal(|row| {
                    if row
                        .button(
                            RichText::new(format!("VCO1: {}", self.synth.vco1_waveform.label()))
                                .size(11.0),
                        )
                        .clicked()
                    {
                        let next = Vco1Waveform::from_index(self.synth.vco1_waveform as usize + 1);
                        self.synth.vco1_waveform = next;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::VCO1_WAVEFORM,
                            value: (next as usize) as f32,
                        });
                    }
                    if row
                        .button(
                            RichText::new(format!("Oct: {}", self.synth.vco1_scale.label()))
                                .size(11.0),
                        )
                        .clicked()
                    {
                        let next = Vco1Scale::from_index(self.synth.vco1_scale as usize + 1);
                        self.synth.vco1_scale = next;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::VCO1_SCALE,
                            value: (next as usize) as f32,
                        });
                    }
                });

                Self::ms20_knob(
                    col,
                    "VCO1 PW",
                    &mut self.synth.vco1_pw,
                    0.0..=10.0,
                    param_ids::VCO1_PW,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "VCO1 Level",
                    &mut self.synth.vco1_level,
                    0.0..=10.0,
                    param_ids::VCO1_LEVEL,
                    tx,
                );

                // VCO 2 Waveform & Scale selectors
                col.horizontal(|row| {
                    if row
                        .button(
                            RichText::new(format!("VCO2: {}", self.synth.vco2_waveform.label()))
                                .size(11.0),
                        )
                        .clicked()
                    {
                        let next = Vco2Waveform::from_index(self.synth.vco2_waveform as usize + 1);
                        self.synth.vco2_waveform = next;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::VCO2_WAVEFORM,
                            value: (next as usize) as f32,
                        });
                    }
                    if row
                        .button(
                            RichText::new(format!("Oct: {}", self.synth.vco2_scale.label()))
                                .size(11.0),
                        )
                        .clicked()
                    {
                        let next = Vco2Scale::from_index(self.synth.vco2_scale as usize + 1);
                        self.synth.vco2_scale = next;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::VCO2_SCALE,
                            value: (next as usize) as f32,
                        });
                    }
                });

                Self::ms20_knob(
                    col,
                    "VCO2 Pitch",
                    &mut self.synth.vco2_pitch,
                    -5.0..=5.0,
                    param_ids::VCO2_PITCH,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "VCO2 Level",
                    &mut self.synth.vco2_level,
                    0.0..=10.0,
                    param_ids::VCO2_LEVEL,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "Portamento",
                    &mut self.synth.portamento_time,
                    0.0..=10.0,
                    param_ids::PORTAMENTO_TIME,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "FM MG/T.EXT",
                    &mut self.synth.fm_mg_text,
                    0.0..=10.0,
                    param_ids::FM_MG_TEXT,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "FM EG1/EXT",
                    &mut self.synth.fm_eg1_ext,
                    0.0..=10.0,
                    param_ids::FM_EG1_EXT,
                    tx,
                );
            });

            // ================================================================
            // COLUMN 2: DUAL KORG35 VCF (VC HPF & VC LPF Cutoff + Peak)
            // ================================================================
            cols[1].group(|col| {
                col.label(
                    RichText::new("VCF: KORG35 HPF & LPF")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(240, 165, 55)),
                );

                col.label(
                    RichText::new("12dB/oct High-Pass Filter (VC HPF)")
                        .size(11.0)
                        .color(Color32::GRAY),
                );
                Self::ms20_knob(
                    col,
                    "HPF Cutoff",
                    &mut self.synth.hpf_cutoff,
                    0.0..=10.0,
                    param_ids::HPF_CUTOFF,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "HPF Peak",
                    &mut self.synth.hpf_peak,
                    0.0..=10.0,
                    param_ids::HPF_PEAK,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "HPF MG/EXT",
                    &mut self.synth.hpf_mg_text,
                    0.0..=10.0,
                    param_ids::HPF_MG_TEXT,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "HPF EG2/EXT",
                    &mut self.synth.hpf_eg2_ext,
                    0.0..=10.0,
                    param_ids::HPF_EG2_EXT,
                    tx,
                );

                col.separator();
                col.label(
                    RichText::new("12dB/oct Low-Pass Filter (VC LPF)")
                        .size(11.0)
                        .color(Color32::GRAY),
                );
                Self::ms20_knob(
                    col,
                    "LPF Cutoff",
                    &mut self.synth.lpf_cutoff,
                    0.0..=10.0,
                    param_ids::LPF_CUTOFF,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "LPF Peak",
                    &mut self.synth.lpf_peak,
                    0.0..=10.0,
                    param_ids::LPF_PEAK,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "LPF MG/EXT",
                    &mut self.synth.lpf_mg_text,
                    0.0..=10.0,
                    param_ids::LPF_MG_TEXT,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "LPF EG2/EXT",
                    &mut self.synth.lpf_eg2_ext,
                    0.0..=10.0,
                    param_ids::LPF_EG2_EXT,
                    tx,
                );
            });

            // ================================================================
            // COLUMN 3: ENVELOPE SLIDERS (EG1 DAR, EG2 HADSR & MG LFO)
            // ================================================================
            cols[2].group(|col| {
                col.label(
                    RichText::new("ENVELOPES (EG1 / EG2) & MG LFO")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(240, 165, 55)),
                );

                col.label(
                    RichText::new("EG 1 (Delay / Attack / Release)")
                        .size(11.0)
                        .color(Color32::GRAY),
                );
                let eg1_d_label = format!("EG1 Delay: {:.1}", self.synth.eg1_delay);
                if col
                    .add(Slider::new(&mut self.synth.eg1_delay, 0.0..=10.0).text(eg1_d_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG1_DELAY,
                        value: self.synth.eg1_delay,
                    });
                }
                let eg1_a_label = format!("EG1 Attack: {:.1}", self.synth.eg1_attack);
                if col
                    .add(Slider::new(&mut self.synth.eg1_attack, 0.0..=10.0).text(eg1_a_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG1_ATTACK,
                        value: self.synth.eg1_attack,
                    });
                }
                let eg1_r_label = format!("EG1 Release: {:.1}", self.synth.eg1_release);
                if col
                    .add(Slider::new(&mut self.synth.eg1_release, 0.0..=10.0).text(eg1_r_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG1_RELEASE,
                        value: self.synth.eg1_release,
                    });
                }

                col.separator();
                col.label(
                    RichText::new("EG 2 (Hold / Attack / Decay / Sustain / Release)")
                        .size(11.0)
                        .color(Color32::GRAY),
                );
                let eg2_h_label = format!("EG2 Hold: {:.1}", self.synth.eg2_hold);
                if col
                    .add(Slider::new(&mut self.synth.eg2_hold, 0.0..=10.0).text(eg2_h_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG2_HOLD,
                        value: self.synth.eg2_hold,
                    });
                }
                let eg2_a_label = format!("EG2 Attack: {:.1}", self.synth.eg2_attack);
                if col
                    .add(Slider::new(&mut self.synth.eg2_attack, 0.0..=10.0).text(eg2_a_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG2_ATTACK,
                        value: self.synth.eg2_attack,
                    });
                }
                let eg2_d_label = format!("EG2 Decay: {:.1}", self.synth.eg2_decay);
                if col
                    .add(Slider::new(&mut self.synth.eg2_decay, 0.0..=10.0).text(eg2_d_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG2_DECAY,
                        value: self.synth.eg2_decay,
                    });
                }
                let eg2_s_label = format!("EG2 Sustain: {:.1}", self.synth.eg2_sustain);
                if col
                    .add(Slider::new(&mut self.synth.eg2_sustain, 0.0..=10.0).text(eg2_s_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG2_SUSTAIN,
                        value: self.synth.eg2_sustain,
                    });
                }
                let eg2_r_label = format!("EG2 Release: {:.1}", self.synth.eg2_release);
                if col
                    .add(Slider::new(&mut self.synth.eg2_release, 0.0..=10.0).text(eg2_r_label))
                    .changed()
                {
                    let _ = tx.try_send(ControlEvent::ParameterChange {
                        id: param_ids::EG2_RELEASE,
                        value: self.synth.eg2_release,
                    });
                }

                // MG LFO Rate & Shape
                Self::ms20_knob(
                    col,
                    "MG Rate",
                    &mut self.synth.mg_frequency,
                    0.0..=10.0,
                    param_ids::MG_FREQUENCY,
                    tx,
                );
            });

            // ================================================================
            // COLUMN 4: EFFECTS (Delay, Distortion/Saturator, Chorus), PATCHBAY & OUTPUT VOLUME
            // ================================================================
            cols[3].group(|col| {
                col.label(
                    RichText::new("FX (DELAY / SAT / CHORUS) & PATCHBAY")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(240, 165, 55)),
                );

                // Effects toggles row
                col.horizontal(|row| {
                    if row
                        .selectable_label(
                            self.pedals.chorus.enabled,
                            RichText::new("CHORUS").size(11.0),
                        )
                        .clicked()
                    {
                        self.pedals.chorus.enabled = !self.pedals.chorus.enabled;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::CHORUS_ENABLED,
                            value: if self.pedals.chorus.enabled { 1.0 } else { 0.0 },
                        });
                    }
                    if row
                        .selectable_label(
                            self.pedals.dd8.enabled,
                            RichText::new("DELAY").size(11.0),
                        )
                        .clicked()
                    {
                        self.pedals.dd8.enabled = !self.pedals.dd8.enabled;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::DD8_ENABLED,
                            value: if self.pedals.dd8.enabled { 1.0 } else { 0.0 },
                        });
                    }
                    if row
                        .selectable_label(
                            self.pedals.cloudburst.enabled,
                            RichText::new("REVERB").size(11.0),
                        )
                        .clicked()
                    {
                        self.pedals.cloudburst.enabled = !self.pedals.cloudburst.enabled;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::CLOUDBURST_ENABLED,
                            value: if self.pedals.cloudburst.enabled {
                                1.0
                            } else {
                                0.0
                            },
                        });
                    }
                    if row
                        .selectable_label(
                            self.pedals.particle.freeze,
                            RichText::new("FREEZE").size(11.0),
                        )
                        .clicked()
                    {
                        self.pedals.particle.freeze = !self.pedals.particle.freeze;
                        let _ = tx.try_send(ControlEvent::ParameterChange {
                            id: param_ids::PARTICLE_FREEZE,
                            value: if self.pedals.particle.freeze { 1.0 } else { 0.0 },
                        });
                    }
                });

                // 1. Distortion / Saturator Drive
                Self::ms20_knob(
                    col,
                    "Saturator Drive",
                    &mut self.saturator_drive,
                    0.0..=10.0,
                    param_ids::SATURATOR_DRIVE,
                    tx,
                );

                // 2. Stereo Chorus (BOSS CE-2W / DC-2W)
                Self::ms20_knob(
                    col,
                    "Chorus Depth",
                    &mut self.pedals.chorus.depth,
                    0.0..=10.0,
                    param_ids::CHORUS_DEPTH,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "Chorus Mix",
                    &mut self.pedals.chorus.mix,
                    0.0..=10.0,
                    param_ids::CHORUS_MIX,
                    tx,
                );

                // 3. Delay (BOSS DD-8)
                Self::ms20_knob(
                    col,
                    "Delay Time",
                    &mut self.pedals.dd8.delay_time,
                    0.0..=10.0,
                    param_ids::DD8_TIME,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "Delay Fdbk",
                    &mut self.pedals.dd8.feedback,
                    0.0..=10.0,
                    param_ids::DD8_FEEDBACK,
                    tx,
                );
                Self::ms20_knob(
                    col,
                    "Delay Level",
                    &mut self.pedals.dd8.effect_level,
                    0.0..=10.0,
                    param_ids::DD8_LEVEL,
                    tx,
                );

                // 4. Master Output Volume
                Self::ms20_knob(
                    col,
                    "Output Volume",
                    &mut self.synth.master_volume,
                    0.0..=10.0,
                    param_ids::MASTER_VOLUME,
                    tx,
                );

                // 5. Semi-Modular 35-Jack Patchbay Quick Router
                col.separator();
                let src = PatchSource::from_index(self.selected_patch_src);
                let dst = PatchDest::from_index(self.selected_patch_dst);
                col.horizontal(|row| {
                    if row
                        .button(RichText::new(format!("Src: {}", src.label())).size(10.5))
                        .clicked()
                    {
                        self.selected_patch_src =
                            (self.selected_patch_src + 1) % PatchSource::ALL.len();
                    }
                    if row
                        .button(RichText::new(format!("Dst: {}", dst.label())).size(10.5))
                        .clicked()
                    {
                        self.selected_patch_dst =
                            (self.selected_patch_dst + 1) % PatchDest::ALL.len();
                    }
                });
                col.horizontal(|row| {
                    if row
                        .button(RichText::new("Patch Cable").size(11.0))
                        .clicked()
                    {
                        self.connect_patch(src, dst, tx);
                    }
                    if row
                        .button(
                            RichText::new(format!("Clear ({})", self.cables.len())).size(11.0),
                        )
                        .clicked()
                    {
                        self.clear_patches(tx);
                    }
                });
            });
        });

        // Paint a subtle bottom divider line between the 65% MS-20 panel and 35% keyboard
        let rect = ui.max_rect();
        ui.painter().line_segment(
            [
                Pos2::new(rect.min.x, rect.max.y),
                Pos2::new(rect.max.x, rect.max.y),
            ],
            Stroke::new(1.5, Color32::from_rgb(235, 145, 35)),
        );
    }

    #[allow(dead_code)]
    pub fn cycle_chorus_mode(&mut self, tx: &Sender<ControlEvent>) {
        let next = ChorusMode::from_index(self.pedals.chorus.mode as usize + 1);
        self.pedals.chorus.mode = next;
        let _ = tx.try_send(ControlEvent::ParameterChange {
            id: param_ids::CHORUS_MODE,
            value: (next as usize) as f32,
        });
    }

    #[allow(dead_code)]
    pub fn cycle_delay_mode(&mut self, tx: &Sender<ControlEvent>) {
        let next = BossDd8Mode::from_index(self.pedals.dd8.mode as usize + 1);
        self.pedals.dd8.mode = next;
        let _ = tx.try_send(ControlEvent::ParameterChange {
            id: param_ids::DD8_MODE,
            value: (next as usize) as f32,
        });
    }
}
