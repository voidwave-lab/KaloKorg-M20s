//! KaloKorg M20s Dedicated Tablet Application (`kalokorg-tablet`).
//!
//! Targets exclusively iPadOS (`UIDeviceFamily = 2`, Landscape) and Android Tablets
//! (`requiresSmallestWidthDp = "600"`, `largeScreens`/`xlargeScreens`, `sensorLandscape`),
//! rendering a pure Rust `egui`/`eframe` interface with:
//! - Top 65% MS-20 Synthesizer, 35-Jack Patchbay & Effects Panel
//! - Bottom 35% Velocity-Aware Multi-Touch Keyboard (25 or 37 keys, polyphonic touch tracking & glissando)
//! - Lock-Free Audio Pipeline (`crossbeam_channel::bounded(256)` -> `oboe` on Android / `cpal` on iPadOS & Desktop)

pub mod app;
pub mod audio;
pub mod keyboard;
pub mod panel;

pub use app::{
    KaloKorgTabletApp, BOTTOM_KEYBOARD_HEIGHT_RATIO, MIN_TABLET_SMALLEST_WIDTH_DP,
    TOP_PANEL_HEIGHT_RATIO,
};
pub use audio::{
    param_ids, ControlEvent, RealtimeTabletProcessor, TabletAudioEngine, TARGET_BUFFER_FRAMES,
};
pub use keyboard::{KeyGeometry, KeyboardSpan, MultiTouchKeyboard};
pub use panel::SynthPanel;

/// Default tablet landscape viewport dimensions (`1280x800` dp).
pub const DEFAULT_TABLET_WIDTH: f32 = 1280.0;
pub const DEFAULT_TABLET_HEIGHT: f32 = 800.0;

/// Build the standard `eframe::NativeOptions` configured for a landscape tablet viewport.
pub fn tablet_native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::ViewportBuilder::default()
            .with_title("KaloKorg M20s — Tablet Synthesizer (iPadOS & Android Tablet)")
            .with_inner_size([DEFAULT_TABLET_WIDTH, DEFAULT_TABLET_HEIGHT])
            .with_min_inner_size([960.0, MIN_TABLET_SMALLEST_WIDTH_DP])
            .with_resizable(true),
    }
}

/// Launch the KaloKorg M20s Tablet application via `eframe::run_native`.
pub fn run_tablet_app() -> eframe::Result {
    let options = tablet_native_options();
    eframe::run_native(
        "KaloKorg M20s",
        options,
        Box::new(|cc| Box::new(KaloKorgTabletApp::new(cc))),
    )
}

/// Android Tablet (`NativeActivity` / `GameActivity`) entry point.
#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(_app: android_activity::AndroidApp) {
    let _ = run_tablet_app();
}

/// iPadOS C-ABI entry point for Xcode (`main.m` / `UIApplicationMain` bridge).
#[cfg(any(target_os = "ios", target_os = "macos"))]
#[no_mangle]
pub extern "C" fn kalokorg_tablet_ios_main() -> i32 {
    match run_tablet_app() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::App;
    use egui::{pos2, vec2, Context, Event, Pos2, RawInput, Rect, TouchDeviceId, TouchId, TouchPhase};

    #[test]
    fn test_step1_tablet_manifest_restrictions() {
        let info_plist = include_str!("../Info.plist");
        assert!(
            info_plist.contains("<key>UIDeviceFamily</key>"),
            "Info.plist must specify UIDeviceFamily"
        );
        assert!(
            info_plist.contains("<integer>2</integer>"),
            "Info.plist must include UIDeviceFamily 2 (iPad)"
        );
        assert!(
            !info_plist.contains("<integer>1</integer>"),
            "Info.plist must strictly omit UIDeviceFamily 1 (iPhone)"
        );
        assert!(
            info_plist.contains("UIInterfaceOrientationLandscapeLeft")
                && info_plist.contains("UIInterfaceOrientationLandscapeRight"),
            "Info.plist must restrict orientation to Landscape"
        );
        assert!(
            info_plist.contains("<key>UIRequiresFullScreen</key>"),
            "Info.plist must require fullscreen on iPadOS"
        );

        let android_manifest = include_str!("../AndroidManifest.xml");
        assert!(
            android_manifest.contains("android:smallScreens=\"false\"")
                && android_manifest.contains("android:normalScreens=\"false\""),
            "AndroidManifest.xml must disable small and normal phone screens"
        );
        assert!(
            android_manifest.contains("android:largeScreens=\"true\"")
                && android_manifest.contains("android:xlargeScreens=\"true\"")
                && android_manifest.contains("android:requiresSmallestWidthDp=\"600\""),
            "AndroidManifest.xml must restrict to large/xlarge screens with smallestWidth >= 600dp"
        );
        assert!(
            android_manifest.contains("android:screenOrientation=\"sensorLandscape\""),
            "AndroidManifest.xml must lock orientation to sensorLandscape"
        );

        // Verify Xcode project (`KaloKorgTablet.xcodeproj/project.pbxproj`) & iOS Info.plist
        let pbxproj = include_str!("../ios/KaloKorgTablet.xcodeproj/project.pbxproj");
        assert!(
            pbxproj.contains("TARGETED_DEVICE_FAMILY = 2;"),
            "Xcode project must set TARGETED_DEVICE_FAMILY = 2 (iPad only)"
        );
        assert!(
            pbxproj.contains("-lkalokorg_tablet"),
            "Xcode project must link libkalokorg_tablet.a"
        );
        let ios_plist = include_str!("../ios/KaloKorgTablet/Info.plist");
        assert!(ios_plist.contains("<integer>2</integer>") && !ios_plist.contains("<integer>1</integer>"));

        // Verify Android Studio Gradle project (`settings.gradle.kts`, `app/build.gradle.kts`, `app/src/main/AndroidManifest.xml`)
        let gradle_settings = include_str!("../android/settings.gradle.kts");
        assert!(gradle_settings.contains("KaloKorgTablet") && gradle_settings.contains(":app"));
        let app_gradle = include_str!("../android/app/build.gradle.kts");
        assert!(
            app_gradle.contains("com.voidwavelab.kalokorg.tablet")
                && app_gradle.contains("arm64-v8a")
                && app_gradle.contains("kalokorg-tablet")
        );
        let studio_manifest = include_str!("../android/app/src/main/AndroidManifest.xml");
        assert!(
            studio_manifest.contains("android:requiresSmallestWidthDp=\"600\"")
                && studio_manifest.contains("android:screenOrientation=\"sensorLandscape\"")
                && studio_manifest.contains("kalokorg_tablet")
        );
    }

    #[test]
    fn test_step3_split_viewport_65_35_geometry() {
        let ctx = Context::default();
        let (mut app, _rx) = KaloKorgTabletApp::new_for_testing();
        let mut frame = eframe::Frame::default();

        let screen_rect = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));
        assert!(KaloKorgTabletApp::is_valid_tablet_landscape_viewport(
            screen_rect
        ));

        // Phone viewport (e.g. 390x844 portrait or 844x390 landscape) must be rejected as non-tablet
        let phone_rect = Rect::from_min_size(Pos2::ZERO, vec2(844.0, 390.0));
        assert!(!KaloKorgTabletApp::is_valid_tablet_landscape_viewport(
            phone_rect
        ));

        let raw = RawInput {
            screen_rect: Some(screen_rect),
            events: Vec::new(),
        };
        let shapes = ctx.run(raw, |egui_ctx| {
            app.update(egui_ctx, &mut frame);
        });

        assert!(!shapes.is_empty(), "Expected painted MS-20 panel & keyboard shapes");
        assert_eq!(app.last_panel_rect.height(), 520.0); // 65% of 800.0
        assert_eq!(app.last_keyboard_rect.height(), 280.0); // 35% of 800.0
        assert_eq!(
            app.last_keyboard_rect.min.y,
            app.last_panel_rect.max.y,
            "Keyboard must sit directly below the 65% synth panel"
        );
    }

    #[test]
    fn test_step4_multitouch_polyphonic_and_glissando_keyboard() {
        let ctx = Context::default();
        let (mut app, rx) = KaloKorgTabletApp::new_for_testing();
        let mut frame = eframe::Frame::default();
        let screen_rect = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));

        // 1. Initial frame to compute key geometries
        ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                events: Vec::new(),
            },
            |egui_ctx| app.update(egui_ctx, &mut frame),
        );

        let geoms = app.keyboard.last_key_geometries.clone();
        assert_eq!(geoms.len(), 37, "Default 3-octave span should have 37 keys (C2..=C5)");

        // Locate white key C2 (MIDI 36), white key D2 (MIDI 38), and black key C#2 (MIDI 37)
        let c2_key = geoms.iter().find(|k| k.note == 36).unwrap();
        let cs2_key = geoms.iter().find(|k| k.note == 37).unwrap();
        let d2_key = geoms.iter().find(|k| k.note == 38).unwrap();
        let e2_key = geoms.iter().find(|k| k.note == 40).unwrap();

        // Pick a point near the bottom of white keys (below the 60% black key overlay)
        let c2_pos = pos2(c2_key.rect.center().x, c2_key.rect.max.y - 20.0);
        let d2_pos = pos2(d2_key.rect.center().x, d2_key.rect.max.y - 20.0);
        let e2_pos = pos2(e2_key.rect.center().x, e2_key.rect.max.y - 20.0);
        // Pick a point inside the black key C#2 (top 60% layer)
        let cs2_pos = cs2_key.rect.center();

        // Verify black key dimensions (~60% height of white key)
        let height_ratio = cs2_key.rect.height() / c2_key.rect.height();
        assert!(
            (height_ratio - 0.60).abs() < 0.02,
            "Black key height ratio should be ~60%, got {}",
            height_ratio
        );

        // 2. Multi-touch: Finger 1 touches C2 (36) AND Finger 2 simultaneously touches E2 (40)
        ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                events: vec![
                    Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(1),
                        phase: TouchPhase::Start,
                        pos: c2_pos,
                        force: Some(0.8),
                    },
                    Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(2),
                        phase: TouchPhase::Start,
                        pos: e2_pos,
                        force: Some(0.9),
                    },
                ],
            },
            |egui_ctx| app.update(egui_ctx, &mut frame),
        );

        assert_eq!(app.keyboard.active_touches.get(&TouchId(1)), Some(&36));
        assert_eq!(app.keyboard.active_touches.get(&TouchId(2)), Some(&40));

        let ev1 = rx.try_recv().expect("Expected NoteOn for Finger 1 (C2 = 36)");
        let ev2 = rx.try_recv().expect("Expected NoteOn for Finger 2 (E2 = 40)");
        assert!(matches!(ev1, ControlEvent::NoteOn { note: 36, .. }));
        assert!(matches!(ev2, ControlEvent::NoteOn { note: 40, .. }));

        // 3. Glissando: Finger 1 slides from C2 (36) onto black key C#2 (37), then onto D2 (38)
        ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                events: vec![Event::Touch {
                    device_id: TouchDeviceId(0),
                    id: TouchId(1),
                    phase: TouchPhase::Move,
                    pos: cs2_pos,
                    force: Some(0.85),
                }],
            },
            |egui_ctx| app.update(egui_ctx, &mut frame),
        );

        assert_eq!(app.keyboard.active_touches.get(&TouchId(1)), Some(&37));
        assert_eq!(
            rx.try_recv().unwrap(),
            ControlEvent::NoteOff { note: 36 },
            "Glissando must emit NoteOff for exited key 36"
        );
        assert!(
            matches!(rx.try_recv().unwrap(), ControlEvent::NoteOn { note: 37, .. }),
            "Glissando must emit NoteOn for entered black key 37"
        );

        // Slide Finger 1 from C#2 (37) to D2 (38)
        ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                events: vec![Event::Touch {
                    device_id: TouchDeviceId(0),
                    id: TouchId(1),
                    phase: TouchPhase::Move,
                    pos: d2_pos,
                    force: Some(0.85),
                }],
            },
            |egui_ctx| app.update(egui_ctx, &mut frame),
        );

        assert_eq!(app.keyboard.active_touches.get(&TouchId(1)), Some(&38));
        assert_eq!(rx.try_recv().unwrap(), ControlEvent::NoteOff { note: 37 });
        assert!(matches!(
            rx.try_recv().unwrap(),
            ControlEvent::NoteOn { note: 38, .. }
        ));

        // 4. Lift both fingers
        ctx.run(
            RawInput {
                screen_rect: Some(screen_rect),
                events: vec![
                    Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(1),
                        phase: TouchPhase::End,
                        pos: d2_pos,
                        force: None,
                    },
                    Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(2),
                        phase: TouchPhase::End,
                        pos: e2_pos,
                        force: None,
                    },
                ],
            },
            |egui_ctx| app.update(egui_ctx, &mut frame),
        );

        assert!(app.keyboard.active_touches.is_empty());
        assert_eq!(rx.try_recv().unwrap(), ControlEvent::NoteOff { note: 38 });
        assert_eq!(rx.try_recv().unwrap(), ControlEvent::NoteOff { note: 40 });
    }

    #[test]
    fn test_step5_lock_free_audio_pipeline_sub10ms_block_processing() {
        let (audio, rx) = TabletAudioEngine::new_test_pair();
        let mut processor = RealtimeTabletProcessor::new(48_000.0);

        // Send NoteOn and parameter tweaks over the bounded(256) lock-free channel
        audio.send_param(param_ids::EG2_ATTACK, 0.0);
        audio.send_param(param_ids::LPF_CUTOFF, 7.5);
        audio.send_param(param_ids::LPF_PEAK, 6.5);
        audio.send_param(param_ids::SATURATOR_DRIVE, 4.0);
        audio.note_on(60, 0.9);

        // Process a 256-frame stereo block (256 * 2 = 512 interleaved floats, 5.33ms @ 48kHz)
        let mut stereo_buf = [0.0f32; (TARGET_BUFFER_FRAMES as usize) * 2];
        processor.process_interleaved(&rx, &mut stereo_buf, 2);

        let energy: f32 = stereo_buf.iter().map(|s| s * s).sum();
        assert!(
            energy > 0.04,
            "Expected audible synthesized MS-20 output from 256-frame callback, got energy={}",
            energy
        );
        assert!(stereo_buf.iter().all(|s| s.is_finite() && s.abs() <= 1.0));

        // Send NoteOff and verify clean release
        audio.note_off(60);
        processor.process_interleaved(&rx, &mut stereo_buf, 2);
        assert!(!processor.engine.voice.gate);
    }

    #[test]
    fn test_keyboard_octave_navigation_and_25_37_key_spans() {
        let (mut app, rx) = KaloKorgTabletApp::new_for_testing();
        assert_eq!(app.keyboard.base_octave, 2);
        assert_eq!(app.keyboard.start_midi_note(), 36); // C2
        assert_eq!(app.keyboard.end_midi_note(), 72); // C5 (37 keys)

        // Switch to 25-key 2-octave mode (C2..=C4)
        app.keyboard.span = KeyboardSpan::Keys25TwoOctaves;
        assert_eq!(app.keyboard.start_midi_note(), 36); // C2
        assert_eq!(app.keyboard.end_midi_note(), 60); // C4 (25 keys)
        let geoms_25 = app
            .keyboard
            .compute_key_geometries(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 250.0)));
        assert_eq!(geoms_25.len(), 25);

        // Octave + -> C3..=C5 (MIDI 48..=72)
        app.keyboard.octave_up(&app.audio.tx);
        assert_eq!(app.keyboard.base_octave, 3);
        assert_eq!(app.keyboard.start_midi_note(), 48);

        // Octave - twice -> C1..=C3 (MIDI 24..=48)
        app.keyboard.octave_down(&app.audio.tx);
        app.keyboard.octave_down(&app.audio.tx);
        assert_eq!(app.keyboard.base_octave, 1);
        assert_eq!(app.keyboard.start_midi_note(), 24);
        assert!(rx.try_recv().is_err());
    }
}
