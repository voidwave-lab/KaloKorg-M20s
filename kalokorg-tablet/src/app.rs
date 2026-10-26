//! Main `eframe::App` Tablet Viewport Layout (`kalokorg-tablet/src/app.rs`).
//!
//! Enforces a landscape tablet split-screen geometry:
//! - **Top Section (65% screen height)**: MS-20 VCO 1 & 2, KORG35 HPF/LPF filters, EG1/EG2 envelope
//!   sliders, 35-jack semi-modular patchbay, and Effects (Delay, Distortion/Saturator, Chorus, Volume).
//! - **Bottom Section (35% screen height)**: 25-key (2 octaves) or 37-key (3 octaves) multi-touch
//!   on-screen keyboard with independent polyphonic touch tracking, glissando, and `Octave -/+` buttons.

use crossbeam_channel::Receiver;
use egui::{CentralPanel, Color32, Context, Frame, Margin, Rect, TopBottomPanel, Visuals};

use crate::audio::{ControlEvent, TabletAudioEngine};
use crate::keyboard::MultiTouchKeyboard;
use crate::panel::SynthPanel;

/// Top MS-20 synth & effects panel height fraction (`65%` of screen height).
pub const TOP_PANEL_HEIGHT_RATIO: f32 = 0.65;

/// Bottom multi-touch keyboard height fraction (`35%` of screen height).
pub const BOTTOM_KEYBOARD_HEIGHT_RATIO: f32 = 0.35;

/// Minimum tablet smallest-width threshold in density-independent pixels (`600dp`),
/// matching `android:requiresSmallestWidthDp="600"` and iPad landscape bounds.
pub const MIN_TABLET_SMALLEST_WIDTH_DP: f32 = 600.0;

/// Main KaloKorg M20s Tablet Application (`eframe::App`).
pub struct KaloKorgTabletApp {
    pub panel: SynthPanel,
    pub keyboard: MultiTouchKeyboard,
    pub audio: TabletAudioEngine,
    pub last_panel_rect: Rect,
    pub last_keyboard_rect: Rect,
}

impl KaloKorgTabletApp {
    /// Initialize the tablet application and start the low-latency lock-free audio engine.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_tablet_visuals(&cc.egui_ctx);
        let audio = TabletAudioEngine::start();
        Self {
            panel: SynthPanel::new(),
            keyboard: MultiTouchKeyboard::new(),
            audio,
            last_panel_rect: Rect::NOTHING,
            last_keyboard_rect: Rect::NOTHING,
        }
    }

    /// Construct a deterministic test instance backed by an inspectable lock-free `Receiver<ControlEvent>`.
    pub fn new_for_testing() -> (Self, Receiver<ControlEvent>) {
        let (audio, rx) = TabletAudioEngine::new_test_pair();
        let app = Self {
            panel: SynthPanel::new(),
            keyboard: MultiTouchKeyboard::new(),
            audio,
            last_panel_rect: Rect::NOTHING,
            last_keyboard_rect: Rect::NOTHING,
        };
        (app, rx)
    }

    /// Apply dark hardware-inspired MS-20 styling to the `egui::Context`.
    pub fn configure_tablet_visuals(ctx: &Context) {
        let mut visuals = Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(20, 22, 26);
        visuals.window_fill = Color32::from_rgb(20, 22, 26);
        visuals.extreme_bg_color = Color32::from_rgb(12, 13, 16);
        ctx.set_visuals(visuals);
    }

    /// Verify whether a viewport rectangle satisfies tablet landscape requirements (`>= 600dp` smallest width
    /// and landscape aspect `width >= height`).
    #[inline]
    pub fn is_valid_tablet_landscape_viewport(screen_rect: Rect) -> bool {
        let w = screen_rect.width();
        let h = screen_rect.height();
        let smallest_width = w.min(h);
        w >= h && smallest_width >= MIN_TABLET_SMALLEST_WIDTH_DP
    }
}

impl eframe::App for KaloKorgTabletApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let screen_rect = ctx.screen_rect();
        let total_h = screen_rect.height().max(1.0);
        let keyboard_h = (total_h * BOTTOM_KEYBOARD_HEIGHT_RATIO).round();
        let panel_h = total_h - keyboard_h;

        let tx = self.audio.tx.clone();
        let status = self.audio.backend_label.clone();

        // 1. Bottom Section (35% screen height): Multi-Touch Keyboard (25 or 37 keys + Octave -/+)
        TopBottomPanel::bottom("ms20_tablet_keyboard_panel")
            .exact_height(keyboard_h)
            .resizable(false)
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(16, 18, 22))
                    .inner_margin(Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                self.keyboard.ui(ui, &tx);
            });

        self.last_keyboard_rect = Rect::from_min_max(
            egui::pos2(screen_rect.min.x, screen_rect.max.y - keyboard_h),
            screen_rect.max,
        );

        // 2. Top Section (65% screen height): MS-20 Synth Controls, Patchbay & Effects Panel
        CentralPanel::default()
            .frame(
                Frame::none()
                    .fill(Color32::from_rgb(22, 24, 29))
                    .inner_margin(Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                self.panel.ui(ui, &tx, &status);
            });

        self.last_panel_rect = Rect::from_min_max(
            screen_rect.min,
            egui::pos2(screen_rect.max.x, screen_rect.min.y + panel_h),
        );

        // Keep requesting continuous frames when touches are held for responsive real-time performance
        if !self.keyboard.active_touches.is_empty() {
            ctx.request_repaint();
        }
    }
}
