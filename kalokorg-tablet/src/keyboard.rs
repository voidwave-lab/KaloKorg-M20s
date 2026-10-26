//! Multi-Touch On-Screen Synthesizer Keyboard (`kalokorg-tablet/src/keyboard.rs`).
//!
//! Renders the bottom 35% of the tablet viewport as a 25-key (2 octaves, C2–C4) or
//! 37-key (3 octaves, C2–C5) velocity-aware multi-touch piano keyboard with:
//! - Independent multi-finger tracking via `HashMap<TouchId, u8>`
//! - Smooth glissando transitions (`NoteOff` on exited key -> `NoteOn` on entered key)
//! - `Octave -` and `Octave +` transpose controls
//! - Standard chromatic black-key geometry (60% height, 60% white-key width) layered on top
//! - Amber/orange active key illumination matching vintage Korg MS-20 styling

use std::collections::HashMap;

use crossbeam_channel::Sender;
use egui::{
    Align2, Button, Color32, FontId, Pos2, Rect, RichText, Rounding, Sense, Stroke, TouchId,
    TouchPhase, Ui, Vec2,
};

use crate::audio::{param_ids, ControlEvent};

/// Reserved pseudo-`TouchId` for single-pointer desktop mouse preview testing.
const DESKTOP_MOUSE_TOUCH_ID: TouchId = TouchId(u64::MAX);

/// Selectable keyboard span: 25 keys (2 octaves, C2–C4) or 37 keys (3 octaves, C2–C5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardSpan {
    /// 25 keys (2 full octaves + top C, e.g. C2..=C4 = MIDI 36..=60)
    Keys25TwoOctaves,
    /// 37 keys (3 full octaves + top C, e.g. C2..=C5 = MIDI 36..=72)
    Keys37ThreeOctaves,
}

impl KeyboardSpan {
    #[inline]
    pub fn total_semitones(self) -> u8 {
        match self {
            Self::Keys25TwoOctaves => 25,
            Self::Keys37ThreeOctaves => 37,
        }
    }

    #[inline]
    pub fn white_key_count(self) -> usize {
        match self {
            Self::Keys25TwoOctaves => 15, // 2 * 7 + 1
            Self::Keys37ThreeOctaves => 22, // 3 * 7 + 1
        }
    }

    #[inline]
    pub fn label(self) -> &'static str {
        match self {
            Self::Keys25TwoOctaves => "25 Keys (2 Oct)",
            Self::Keys37ThreeOctaves => "37 Keys (3 Oct)",
        }
    }
}

/// Computed screen rectangle and metadata for a single piano key.
#[derive(Clone, Copy, Debug)]
pub struct KeyGeometry {
    pub note: u8,
    pub is_black: bool,
    pub rect: Rect,
}

/// Multi-touch on-screen keyboard state and geometry engine.
pub struct MultiTouchKeyboard {
    /// Active touch-to-MIDI-note state tracker (`TouchId -> MIDI note u8`).
    pub active_touches: HashMap<TouchId, u8>,
    /// Base octave number (`2` means leftmost key is `C2` = MIDI note `36`).
    pub base_octave: i8,
    /// Keyboard range mode (`25` keys C2–C4 or `37` keys C2–C5).
    pub span: KeyboardSpan,
    /// Left-hand MS-20 Control Wheel (`-5.0 .. +5.0`).
    pub control_wheel: f32,
    /// Left-hand MS-20 Momentary Switch (`false` / `true`).
    pub momentary_switch: bool,
    /// Cached key geometries from the most recent layout pass (used for hit testing & tests).
    pub last_key_geometries: Vec<KeyGeometry>,
    /// Last computed keyboard bed rectangle.
    pub last_bed_rect: Rect,
}

impl Default for MultiTouchKeyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiTouchKeyboard {
    pub fn new() -> Self {
        Self {
            active_touches: HashMap::with_capacity(16),
            base_octave: 2, // C2 (MIDI 36)
            span: KeyboardSpan::Keys37ThreeOctaves,
            control_wheel: 0.0,
            momentary_switch: false,
            last_key_geometries: Vec::with_capacity(37),
            last_bed_rect: Rect::NOTHING,
        }
    }

    /// Lowest MIDI note currently visible on the keyboard (`C2` = `36` when `base_octave == 2`).
    #[inline]
    pub fn start_midi_note(&self) -> u8 {
        ((self.base_octave as i16 + 1) * 12).clamp(12, 84) as u8
    }

    /// Highest MIDI note currently visible on the keyboard.
    #[inline]
    pub fn end_midi_note(&self) -> u8 {
        (self.start_midi_note() + self.span.total_semitones() - 1).min(127)
    }

    /// Shift the keyboard down by one octave (`Octave -`), releasing any held notes cleanly.
    pub fn octave_down(&mut self, tx: &Sender<ControlEvent>) {
        if self.base_octave > 0 {
            self.release_all_touches(tx);
            self.base_octave -= 1;
        }
    }

    /// Shift the keyboard up by one octave (`Octave +`), releasing any held notes cleanly.
    pub fn octave_up(&mut self, tx: &Sender<ControlEvent>) {
        if self.base_octave < 5 {
            self.release_all_touches(tx);
            self.base_octave += 1;
        }
    }

    /// Release all currently active touches and emit `NoteOff` for each unique held note.
    pub fn release_all_touches(&mut self, tx: &Sender<ControlEvent>) {
        let held_notes: Vec<u8> = self.active_touches.values().copied().collect();
        self.active_touches.clear();
        for note in held_notes {
            let _ = tx.try_send(ControlEvent::NoteOff { note });
        }
    }

    /// Returns `true` if any active finger touch is currently pressing `note`.
    #[inline]
    pub fn is_note_pressed(&self, note: u8) -> bool {
        self.active_touches.values().any(|&n| n == note)
    }

    /// Returns `true` if `note % 12` is a chromatic sharp/flat (black key: C#, D#, F#, G#, A#).
    #[inline]
    pub const fn is_black_key(note: u8) -> bool {
        matches!(note % 12, 1 | 3 | 6 | 8 | 10)
    }

    /// Compute exact rectangles for all white and black keys within `bed_rect`.
    ///
    /// - White keys are distributed uniformly across `bed_rect.width()`.
    /// - Black keys are placed at standard chromatic intervals (`C#`, `D#`, `F#`, `G#`, `A#`),
    ///   occupying `60%` of `bed_rect.height()` and `60%` of `white_key_width`.
    pub fn compute_key_geometries(&self, bed_rect: Rect) -> Vec<KeyGeometry> {
        let start_note = self.start_midi_note();
        let total_keys = self.span.total_semitones();
        let white_count = self.span.white_key_count().max(1);
        let white_w = bed_rect.width() / (white_count as f32);
        let white_h = bed_rect.height();
        let black_w = white_w * 0.60;
        let black_h = white_h * 0.60;

        let mut geometries = Vec::with_capacity(total_keys as usize);
        let mut white_idx = 0usize;

        for offset in 0..total_keys {
            let note = start_note + offset;
            if !Self::is_black_key(note) {
                let x0 = bed_rect.min.x + (white_idx as f32) * white_w;
                let x1 = x0 + white_w;
                let rect = Rect::from_min_max(
                    Pos2::new(x0, bed_rect.min.y),
                    Pos2::new(x1, bed_rect.max.y),
                );
                geometries.push(KeyGeometry {
                    note,
                    is_black: false,
                    rect,
                });
                white_idx += 1;
            } else {
                // Black key sits centered near the boundary between the preceding white key (`white_idx - 1`)
                // and the following white key (`white_idx`).
                let boundary_x = bed_rect.min.x + (white_idx as f32) * white_w;
                let center_x = match note % 12 {
                    1 => boundary_x - black_w * 0.08, // C# slightly left of center
                    3 => boundary_x + black_w * 0.08, // D# slightly right of center
                    6 => boundary_x - black_w * 0.10, // F#
                    8 => boundary_x,                  // G# centered
                    10 => boundary_x + black_w * 0.10, // A#
                    _ => boundary_x,
                };
                let rect = Rect::from_min_max(
                    Pos2::new(center_x - black_w * 0.5, bed_rect.min.y),
                    Pos2::new(center_x + black_w * 0.5, bed_rect.min.y + black_h),
                );
                geometries.push(KeyGeometry {
                    note,
                    is_black: true,
                    rect,
                });
            }
        }

        geometries
    }

    /// Hit-test a touch/pointer position against the keyboard geometry.
    /// Checks black keys first (since they are layered on top of white keys), then white keys.
    pub fn hit_test_note(&self, pos: Pos2, geometries: &[KeyGeometry]) -> Option<u8> {
        // 1. Check black keys first (top layer)
        for key in geometries.iter().filter(|k| k.is_black) {
            if key.rect.contains(pos) {
                return Some(key.note);
            }
        }
        // 2. Check white keys (base layer)
        for key in geometries.iter().filter(|k| !k.is_black) {
            if key.rect.contains(pos) {
                return Some(key.note);
            }
        }
        None
    }

    /// Compute normalized MIDI velocity (`0.25 .. 1.0`) from vertical touch position on the key
    /// or hardware touch pressure/force if reported by the tablet digitizer.
    #[inline]
    fn compute_touch_velocity(pos: Pos2, force: Option<f32>, bed_rect: Rect) -> f32 {
        if let Some(f) = force {
            if f > 0.05 {
                return f.clamp(0.25, 1.0);
            }
        }
        let rel_y = ((pos.y - bed_rect.min.y) / bed_rect.height().max(1.0)).clamp(0.0, 1.0);
        (0.55 + 0.45 * rel_y).clamp(0.25, 1.0)
    }

    /// Process a single touch contact transition (`Start`, `Move`, `End`, `Cancel`).
    /// Handles multi-finger independence and glissando (`NoteOff` on exited key, `NoteOn` on entered key).
    pub fn process_touch_point(
        &mut self,
        touch_id: TouchId,
        phase: TouchPhase,
        pos: Pos2,
        force: Option<f32>,
        geometries: &[KeyGeometry],
        bed_rect: Rect,
        tx: &Sender<ControlEvent>,
    ) {
        match phase {
            TouchPhase::Start | TouchPhase::Move => {
                let hit_note = self.hit_test_note(pos, geometries);
                let prev_note = self.active_touches.get(&touch_id).copied();

                match (prev_note, hit_note) {
                    (None, Some(new_note)) => {
                        // Finger touched down onto a key
                        self.active_touches.insert(touch_id, new_note);
                        let velocity = Self::compute_touch_velocity(pos, force, bed_rect);
                        let _ = tx.try_send(ControlEvent::NoteOn {
                            note: new_note,
                            velocity,
                        });
                    }
                    (Some(old_note), Some(new_note)) if old_note != new_note => {
                        // Glissando: finger slid from `old_note` onto `new_note`
                        self.active_touches.insert(touch_id, new_note);
                        let velocity = Self::compute_touch_velocity(pos, force, bed_rect);
                        // Trigger the new note first or release old note cleanly
                        let still_held_by_other = self
                            .active_touches
                            .iter()
                            .any(|(&id, &n)| id != touch_id && n == old_note);
                        if !still_held_by_other {
                            let _ = tx.try_send(ControlEvent::NoteOff { note: old_note });
                        }
                        let _ = tx.try_send(ControlEvent::NoteOn {
                            note: new_note,
                            velocity,
                        });
                    }
                    (Some(old_note), None) => {
                        // Finger dragged outside the keyboard bed
                        self.active_touches.remove(&touch_id);
                        let still_held_by_other =
                            self.active_touches.values().any(|&n| n == old_note);
                        if !still_held_by_other {
                            let _ = tx.try_send(ControlEvent::NoteOff { note: old_note });
                        }
                    }
                    _ => {}
                }
            }
            TouchPhase::End | TouchPhase::Cancel => {
                if let Some(old_note) = self.active_touches.remove(&touch_id) {
                    let still_held_by_other = self.active_touches.values().any(|&n| n == old_note);
                    if !still_held_by_other {
                        let _ = tx.try_send(ControlEvent::NoteOff { note: old_note });
                    }
                }
            }
        }
    }

    /// Render the complete bottom keyboard section (left sidebar with Octave -/+ & Wheel + multi-touch keybed).
    pub fn ui(&mut self, ui: &mut Ui, tx: &Sender<ControlEvent>) {
        let full_rect = ui.available_rect_before_wrap();
        let sidebar_w = (full_rect.width() * 0.13).clamp(115.0, 165.0);

        let sidebar_rect = Rect::from_min_max(
            full_rect.min,
            Pos2::new(full_rect.min.x + sidebar_w, full_rect.max.y),
        );
        let bed_rect = Rect::from_min_max(
            Pos2::new(full_rect.min.x + sidebar_w + 6.0, full_rect.min.y + 2.0),
            Pos2::new(full_rect.max.x - 4.0, full_rect.max.y - 2.0),
        );
        self.last_bed_rect = bed_rect;

        // 1. Render Left-Hand Octave Navigation & MS-20 Control Wheel / Momentary Switch Sidebar
        ui.allocate_ui_at_rect(sidebar_rect, |side_ui| {
            side_ui.painter().rect_filled(
                sidebar_rect,
                Rounding::same(6.0),
                Color32::from_rgb(24, 26, 31),
            );
            side_ui.painter().rect_stroke(
                sidebar_rect,
                Rounding::same(6.0),
                Stroke::new(1.0, Color32::from_rgb(70, 75, 84)),
            );

            let start_oct = self.base_octave;
            let end_oct = start_oct
                + match self.span {
                    KeyboardSpan::Keys25TwoOctaves => 2,
                    KeyboardSpan::Keys37ThreeOctaves => 3,
                };
            side_ui.label(
                RichText::new(format!("RANGE: C{} - C{}", start_oct, end_oct))
                    .size(12.0)
                    .strong()
                    .color(Color32::from_rgb(240, 165, 55)),
            );

            side_ui.horizontal(|row| {
                if row
                    .add(
                        Button::new(RichText::new("Octave -").size(12.0).strong())
                            .fill(Color32::from_rgb(45, 50, 58))
                            .min_size(Vec2::new(58.0, 28.0)),
                    )
                    .clicked()
                {
                    self.octave_down(tx);
                }
                if row
                    .add(
                        Button::new(RichText::new("Octave +").size(12.0).strong())
                            .fill(Color32::from_rgb(45, 50, 58))
                            .min_size(Vec2::new(58.0, 28.0)),
                    )
                    .clicked()
                {
                    self.octave_up(tx);
                }
            });

            // Toggle between 25 Keys (2 Octaves, C2-C4) and 37 Keys (3 Octaves, C2-C5)
            let next_span_label = self.span.label();
            if side_ui
                .add(
                    Button::new(RichText::new(next_span_label).size(11.0))
                        .fill(Color32::from_rgb(38, 42, 49))
                        .min_size(Vec2::new(sidebar_w - 16.0, 24.0)),
                )
                .clicked()
            {
                self.release_all_touches(tx);
                self.span = match self.span {
                    KeyboardSpan::Keys25TwoOctaves => KeyboardSpan::Keys37ThreeOctaves,
                    KeyboardSpan::Keys37ThreeOctaves => KeyboardSpan::Keys25TwoOctaves,
                };
            }

            // Control Wheel slider & Momentary Switch
            side_ui.label(
                RichText::new(format!("WHEEL: {:+.1}V", self.control_wheel))
                    .size(11.0)
                    .color(Color32::LIGHT_GRAY),
            );
            if side_ui
                .add(
                    egui::Slider::new(&mut self.control_wheel, -5.0..=5.0)
                        .show_value(false),
                )
                .changed()
            {
                let _ = tx.try_send(ControlEvent::ParameterChange {
                    id: param_ids::CONTROL_WHEEL,
                    value: self.control_wheel,
                });
            }

            let mom_label = if self.momentary_switch {
                "MOMENT SW: ON"
            } else {
                "MOMENT SW: OFF"
            };
            if side_ui
                .selectable_label(self.momentary_switch, RichText::new(mom_label).size(11.0))
                .clicked()
            {
                self.momentary_switch = !self.momentary_switch;
                let _ = tx.try_send(ControlEvent::ParameterChange {
                    id: param_ids::MOMENTARY_SWITCH,
                    value: if self.momentary_switch { 1.0 } else { 0.0 },
                });
            }
        });

        // 2. Compute Piano Key Geometries & Inspect Multi-Touch / Pointer Input
        let geometries = self.compute_key_geometries(bed_rect);

        let (touches, primary_down, interact_pos) = ui.input(|i| {
            (
                i.pointer.touches().to_vec(),
                i.pointer.primary_down(),
                i.pointer.interact_pos(),
            )
        });

        if !touches.is_empty() {
            for t in touches {
                self.process_touch_point(
                    t.id,
                    t.phase,
                    t.pos,
                    t.force,
                    &geometries,
                    bed_rect,
                    tx,
                );
            }
        } else if primary_down {
            // Desktop single-pointer mouse fallback for `cargo run -p kalokorg-tablet`
            if let Some(pos) = interact_pos {
                let phase = if self.active_touches.contains_key(&DESKTOP_MOUSE_TOUCH_ID) {
                    TouchPhase::Move
                } else {
                    TouchPhase::Start
                };
                self.process_touch_point(
                    DESKTOP_MOUSE_TOUCH_ID,
                    phase,
                    pos,
                    Some(0.85),
                    &geometries,
                    bed_rect,
                    tx,
                );
            }
        } else if self.active_touches.contains_key(&DESKTOP_MOUSE_TOUCH_ID) {
            self.process_touch_point(
                DESKTOP_MOUSE_TOUCH_ID,
                TouchPhase::End,
                Pos2::ZERO,
                None,
                &geometries,
                bed_rect,
                tx,
            );
        }

        // 3. Allocate Keybed Region & Paint White Keys first, then Black Keys on top
        let _bed_response = ui.allocate_rect(bed_rect, Sense::click_and_drag());
        let painter = ui.painter();

        // Keybed frame background
        painter.rect_filled(bed_rect, Rounding::same(4.0), Color32::from_rgb(14, 15, 18));

        // 3a. Paint White Keys uniformly across the keyboard width
        let amber_white = Color32::from_rgb(240, 150, 35);
        let default_white = Color32::from_rgb(236, 234, 226);

        for key in geometries.iter().filter(|k| !k.is_black) {
            let active = self.is_note_pressed(key.note);
            let fill = if active { amber_white } else { default_white };
            let key_draw_rect = key.rect.shrink2(Vec2::new(0.8, 0.5));

            painter.rect_filled(key_draw_rect, Rounding::same(3.0), fill);
            painter.rect_stroke(
                key_draw_rect,
                Rounding::same(3.0),
                Stroke::new(1.0, Color32::from_rgb(35, 38, 44)),
            );

            // Label C keys (`C2`, `C3`, `C4`, `C5`) or currently pressed white keys
            if key.note % 12 == 0 || active {
                let octave = (key.note as i16 / 12) - 1;
                let note_name = Self::note_name(key.note);
                let label = format!("{}{}", note_name, octave);
                let text_col = if active {
                    Color32::BLACK
                } else {
                    Color32::from_rgb(70, 72, 78)
                };
                painter.text(
                    Pos2::new(key.rect.center().x, key.rect.max.y - 14.0),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::monospace(10.5),
                    text_col,
                );
            }
        }

        // 3b. Paint Black Keys (~60% height, 60% white key width) layered on top
        let amber_black = Color32::from_rgb(220, 120, 20);
        let default_black = Color32::from_rgb(26, 28, 32);

        for key in geometries.iter().filter(|k| k.is_black) {
            let active = self.is_note_pressed(key.note);
            let fill = if active { amber_black } else { default_black };

            painter.rect_filled(key.rect, Rounding::same(3.0), fill);
            painter.rect_stroke(
                key.rect,
                Rounding::same(3.0),
                Stroke::new(
                    1.2,
                    if active {
                        Color32::from_rgb(255, 200, 110)
                    } else {
                        Color32::from_rgb(68, 72, 80)
                    },
                ),
            );
        }

        self.last_key_geometries = geometries;
    }

    #[inline]
    pub const fn note_name(note: u8) -> &'static str {
        match note % 12 {
            0 => "C",
            1 => "C#",
            2 => "D",
            3 => "D#",
            4 => "E",
            5 => "F",
            6 => "F#",
            7 => "G",
            8 => "G#",
            9 => "A",
            10 => "A#",
            _ => "B",
        }
    }
}
