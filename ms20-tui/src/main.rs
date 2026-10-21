mod audio_host;
mod midi_host;
mod recorder;
mod ui;
mod web_server;

use std::env;
use std::io::{self, IsTerminal};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use ms20_core::{
    params::{FilterRevision, PedalId},
    AudioProcessor,
};
use crate::audio_host::{AudioHost, SharedSynthState};
use crate::midi_host::MidiHost;
use crate::ui::{ActiveTab, SynthFocusArea, TuiApp};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut web_only = false;
    let mut web_port: u16 = 8080;
    let mut demo_record_secs: Option<f32> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--web" => web_only = true,
            "--tui" => web_only = false,
            "--port" => {
                if i + 1 < args.len() {
                    web_port = args[i + 1].parse().unwrap_or(8080);
                    i += 1;
                }
            }
            "--demo-record" => {
                if i + 1 < args.len() {
                    demo_record_secs = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("KaloKorg M20s — Korg MS-20 Mini Clone & Ambient Guitar Pedalboard");
                println!("Usage:");
                println!("  kalokorg-m20s              Launch TUI + Web Interface (http://127.0.0.1:8080)");
                println!("  kalokorg-m20s --web        Launch in Web Interface server mode only");
                println!("  kalokorg-m20s --port 9000  Specify HTTP port for the Web Interface");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    let sample_rate = 48_000;
    let shared_state = Arc::new(Mutex::new(SharedSynthState::new(sample_rate)));

    // Headless CLI demo recording mode (for verification or batch rendering)
    if let Some(secs) = demo_record_secs {
        if let Ok(mut guard) = shared_state.lock() {
            guard.recorder.start();
            guard.engine.handle_midi_event([0x90, 48, 110]); // C3 Note On
            let total_frames = (secs * (sample_rate as f32)) as usize;
            for f in 0..total_frames {
                if f == total_frames / 2 {
                    guard.engine.handle_midi_event([0x90, 55, 115]); // G3 fifth
                }
                let (l, r) = guard.engine.render_frame();
                guard.recorder.push_stereo(l, r);
            }
            let saved = guard.recorder.stop_and_save()?;
            println!("Recorded demo session to {}", saved);
        }
        return Ok(());
    }

    // 1. Start Native Audio Host (CoreAudio 48kHz Stereo on macOS + fallback)
    let _audio_host = AudioHost::start(shared_state.clone(), sample_rate);

    // 2. Start Native Hardware MIDI Host (CoreMIDI on macOS)
    let mut midi_host = MidiHost::start(shared_state.clone());

    // 3. Start Embedded Web Interface Server
    let actual_port = web_server::spawn_web_server(shared_state.clone(), web_port)
        .unwrap_or(web_port);

    if web_only || !io::stdout().is_terminal() {
        println!("╔══════════════════════════════════════════════════════════════════╗");
        println!("║  KALOKORG M20s — Korg MS-20 Mini & Ambient Guitar Pedalboard     ║");
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!("║  Web Interface active at: http://127.0.0.1:{:<21} ║", actual_port);
        println!("║  Native Audio & MIDI Controller Engine running in background.    ║");
        println!("║  Press Ctrl+C to stop.                                           ║");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    // 4. Launch Interactive Terminal User Interface (TUI)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(actual_port);
    let res = run_tui_loop(&mut terminal, &mut app, &shared_state, &mut midi_host);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("TUI error: {}", e);
    }
    Ok(())
}

fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
    state: &Arc<Mutex<SharedSynthState>>,
    midi_host: &mut MidiHost,
) -> io::Result<()> {
    let mut last_note_time: Option<Instant> = None;

    loop {
        if let Ok(guard) = state.lock() {
            terminal.draw(|f| ui::render_ui(f, app, &guard))?;
        }

        // Auto-release computer-keyboard auditioned notes after 650ms if Drone Gate is off
        if let Some(t) = last_note_time {
            if t.elapsed() > Duration::from_millis(650) {
                if let Ok(mut guard) = state.lock() {
                    if let Some(n) = app.last_audition_note.take() {
                        guard.engine.handle_midi_event([0x80, n, 0]);
                    }
                }
                last_note_time = None;
            }
        }

        if event::poll(Duration::from_millis(45))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Exit
                if key.code == KeyCode::Esc
                    || key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    if let Ok(mut guard) = state.lock() {
                        if guard.recorder.is_recording {
                            let _ = guard.recorder.stop_and_save();
                        }
                    }
                    break;
                }

                // Switch Tabs (Tab / F1 / F2)
                match key.code {
                    KeyCode::Tab => {
                        app.active_tab = match app.active_tab {
                            ActiveTab::SynthPanel => ActiveTab::Pedalboard,
                            ActiveTab::Pedalboard => ActiveTab::SynthPanel,
                        };
                        continue;
                    }
                    KeyCode::F(1) => {
                        app.active_tab = ActiveTab::SynthPanel;
                        continue;
                    }
                    KeyCode::F(2) => {
                        app.active_tab = ActiveTab::Pedalboard;
                        continue;
                    }
                    _ => {}
                }

                // Global Shortcuts: Preset (P), Filter Rev (V), Drone (D), MIDI Rescan (M), Record WAV (R)
                match key.code {
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        if let Ok(mut guard) = state.lock() {
                            let next = (guard.selected_preset_idx + 1) % app.presets.len();
                            guard.selected_preset_idx = next;
                            let preset = app.presets[next].clone();
                            guard.engine.load_preset(&preset);
                            app.status_banner =
                                format!("Loaded Preset: {} — {}", preset.name, preset.description);
                        }
                        continue;
                    }
                    KeyCode::Char('v') | KeyCode::Char('V') => {
                        if let Ok(mut guard) = state.lock() {
                            guard.engine.params.filter_revision =
                                match guard.engine.params.filter_revision {
                                    FilterRevision::Rev1Korg35 => FilterRevision::Rev2Ota,
                                    FilterRevision::Rev2Ota => FilterRevision::Rev1Korg35,
                                };
                            app.status_banner = format!(
                                "Switched VCF IC Topology to {}",
                                guard.engine.params.filter_revision.label()
                            );
                        }
                        continue;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Ok(mut guard) = state.lock() {
                            guard.engine.params.drone_gate = !guard.engine.params.drone_gate;
                            app.status_banner = if guard.engine.params.drone_gate {
                                "Drone Gate LATCHED ON (Continuous MS-20 synthesis)".to_string()
                            } else {
                                "Drone Gate OFF".to_string()
                            };
                        }
                        continue;
                    }
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        midi_host.rescan(state);
                        if let Ok(mut guard) = state.lock() {
                            if guard.midi_ports.is_empty() {
                                app.status_banner =
                                    "Rescanned CoreMIDI: No external USB MIDI controllers found."
                                        .to_string();
                            } else {
                                guard.active_midi_port = match guard.active_midi_port {
                                    None => Some(0),
                                    Some(i) if i + 1 < guard.midi_ports.len() => Some(i + 1),
                                    _ => None,
                                };
                                app.status_banner =
                                    "Updated active hardware MIDI input port.".to_string();
                            }
                        }
                        continue;
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if let Ok(mut guard) = state.lock() {
                            if guard.recorder.is_recording {
                                match guard.recorder.stop_and_save() {
                                    Ok(path) => {
                                        app.status_banner =
                                            format!("Saved stereo session WAV to {}", path);
                                    }
                                    Err(e) => {
                                        app.status_banner = format!("Recording error: {}", e);
                                    }
                                }
                            } else {
                                guard.recorder.start();
                                app.status_banner =
                                    "Started WAV session recording! Play MIDI or Drone, press [R] again to save."
                                        .to_string();
                            }
                        }
                        continue;
                    }
                    // Global Output Pad / Attenuation Trim: [9] or [(] = -1.5 dB, [0] or [)] = +1.5 dB
                    KeyCode::Char('9') | KeyCode::Char('(') => {
                        if let Ok(mut guard) = state.lock() {
                            let next_db = (guard.engine.params.output_pad_db - 1.5).clamp(-30.0, 6.0);
                            guard.engine.set_parameter(38, next_db);
                            app.status_banner = format!(
                                "Output Signal Pad / Attenuation: {:+.1} dB (Press [9]/[0] to adjust)",
                                next_db
                            );
                        }
                        continue;
                    }
                    KeyCode::Char('0') | KeyCode::Char(')') => {
                        if let Ok(mut guard) = state.lock() {
                            let next_db = (guard.engine.params.output_pad_db + 1.5).clamp(-30.0, 6.0);
                            guard.engine.set_parameter(38, next_db);
                            app.status_banner = format!(
                                "Output Signal Pad / Attenuation: {:+.1} dB (Press [9]/[0] to adjust)",
                                next_db
                            );
                        }
                        continue;
                    }
                    // Computer keyboard note auditioning (Z, X, C, V, B, N or 1..8)
                    KeyCode::Char(ch)
                        if matches!(ch, 'z' | 'x' | 'c' | 'b' | 'n' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8') =>
                    {
                        let note = match ch {
                            'z' => 48, // C3
                            'x' => 50, // D3
                            'c' => 52, // E3
                            'b' => 55, // G3
                            'n' => 57, // A3
                            '1' => 36, // C2 Sub Bass
                            '2' => 48, // C3
                            '3' => 55, // G3
                            '4' => 60, // C4
                            '5' => 64, // E4
                            '6' => 67, // G4
                            '7' => 71, // B4
                            _ => 72,   // C5
                        };
                        if let Ok(mut guard) = state.lock() {
                            if let Some(prev) = app.last_audition_note.take() {
                                guard.engine.handle_midi_event([0x80, prev, 0]);
                            }
                            guard.engine.handle_midi_event([0x90, note, 115]);
                            guard.last_midi_message = format!("Audition Note #{} ON", note);
                            app.last_audition_note = Some(note);
                            last_note_time = Some(Instant::now());
                        }
                        continue;
                    }
                    _ => {}
                }

                // Tab-Specific Navigation & Controls
                match app.active_tab {
                    ActiveTab::SynthPanel => {
                        handle_synth_tab_key(app, state, key.code);
                    }
                    ActiveTab::Pedalboard => {
                        handle_pedal_tab_key(app, state, key.code);
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_synth_tab_key(app: &mut TuiApp, state: &Arc<Mutex<SharedSynthState>>, code: KeyCode) {
    if code == KeyCode::Char('j') || code == KeyCode::Char('J') {
        app.synth_focus = match app.synth_focus {
            SynthFocusArea::Knobs => SynthFocusArea::Patchbay,
            SynthFocusArea::Patchbay => SynthFocusArea::Knobs,
        };
        app.status_banner = match app.synth_focus {
            SynthFocusArea::Knobs => "Focused MS-20 Front-Panel Knobs".to_string(),
            SynthFocusArea::Patchbay => {
                "Focused 35-Jack Patchbay: [Up/Down] Source, [Left/Right] Dest, [Enter] Patch, [Backspace] Unplug".to_string()
            }
        };
        return;
    }

    match app.synth_focus {
        SynthFocusArea::Knobs => match code {
            KeyCode::Right | KeyCode::Char('l') => {
                app.selected_knob_idx = (app.selected_knob_idx + 1) % 36;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                app.selected_knob_idx = (app.selected_knob_idx + 35) % 36;
            }
            KeyCode::Up | KeyCode::Char('+') | KeyCode::Char('=') => {
                if let Ok(mut guard) = state.lock() {
                    app.adjust_selected_synth_knob(&mut guard, 0.2);
                }
            }
            KeyCode::Down | KeyCode::Char('-') => {
                if let Ok(mut guard) = state.lock() {
                    app.adjust_selected_synth_knob(&mut guard, -0.2);
                }
            }
            KeyCode::PageUp => {
                if let Ok(mut guard) = state.lock() {
                    app.adjust_selected_synth_knob(&mut guard, 1.0);
                }
            }
            KeyCode::PageDown => {
                if let Ok(mut guard) = state.lock() {
                    app.adjust_selected_synth_knob(&mut guard, -1.0);
                }
            }
            KeyCode::Char(' ') => {
                if let Ok(mut guard) = state.lock() {
                    guard.engine.params.drone_gate = !guard.engine.params.drone_gate;
                }
            }
            _ => {}
        },
        SynthFocusArea::Patchbay => match code {
            KeyCode::Up => {
                app.selected_patch_src = (app.selected_patch_src + 19) % 20;
            }
            KeyCode::Down => {
                app.selected_patch_src = (app.selected_patch_src + 1) % 20;
            }
            KeyCode::Left => {
                app.selected_patch_dst = (app.selected_patch_dst + 14) % 15;
            }
            KeyCode::Right => {
                app.selected_patch_dst = (app.selected_patch_dst + 1) % 15;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Ok(mut guard) = state.lock() {
                    app.connect_selected_patch(&mut guard);
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if let Ok(mut guard) = state.lock() {
                    app.disconnect_selected_patch(&mut guard);
                }
            }
            _ => {}
        },
    }
}

fn handle_pedal_tab_key(app: &mut TuiApp, state: &Arc<Mutex<SharedSynthState>>, code: KeyCode) {
    match code {
        KeyCode::Left => {
            app.selected_pedal_slot = (app.selected_pedal_slot + 4) % 5;
            app.selected_pedal_param = 0;
        }
        KeyCode::Right => {
            app.selected_pedal_slot = (app.selected_pedal_slot + 1) % 5;
            app.selected_pedal_param = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.selected_pedal_param > 0 {
                app.selected_pedal_param -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.selected_pedal_param < 6 {
                app.selected_pedal_param += 1;
            }
        }
        // Reorder pedal in signal chain with `<` / `,` or `>` / `.`
        KeyCode::Char('<') | KeyCode::Char(',') => {
            if let Ok(mut guard) = state.lock() {
                if app.selected_pedal_slot > 0 {
                    guard.engine.move_pedal(app.selected_pedal_slot, -1);
                    app.selected_pedal_slot -= 1;
                    app.status_banner =
                        "Moved guitar pedal earlier in the effect chain!".to_string();
                }
            }
        }
        KeyCode::Char('>') | KeyCode::Char('.') => {
            if let Ok(mut guard) = state.lock() {
                if app.selected_pedal_slot < 4 {
                    guard.engine.move_pedal(app.selected_pedal_slot, 1);
                    app.selected_pedal_slot += 1;
                    app.status_banner =
                        "Moved guitar pedal later in the effect chain!".to_string();
                }
            }
        }
        // Footswitch Bypass toggle (Space or 'b')
        KeyCode::Char(' ') | KeyCode::Char('b') | KeyCode::Char('B') => {
            if let Ok(mut guard) = state.lock() {
                let pid = guard.engine.pedals_params.chain_order[app.selected_pedal_slot];
                let now_on = match pid {
                    PedalId::BossCe2wChorus => {
                        guard.engine.pedals_params.chorus.enabled =
                            !guard.engine.pedals_params.chorus.enabled;
                        guard.engine.pedals_params.chorus.enabled
                    }
                    PedalId::ZoomMs70Cdr => {
                        guard.engine.pedals_params.zoom.enabled =
                            !guard.engine.pedals_params.zoom.enabled;
                        guard.engine.pedals_params.zoom.enabled
                    }
                    PedalId::BossDd8 => {
                        guard.engine.pedals_params.dd8.enabled =
                            !guard.engine.pedals_params.dd8.enabled;
                        guard.engine.pedals_params.dd8.enabled
                    }
                    PedalId::RedPandaParticle2 => {
                        guard.engine.pedals_params.particle.enabled =
                            !guard.engine.pedals_params.particle.enabled;
                        guard.engine.pedals_params.particle.enabled
                    }
                    PedalId::StrymonCloudburst => {
                        guard.engine.pedals_params.cloudburst.enabled =
                            !guard.engine.pedals_params.cloudburst.enabled;
                        guard.engine.pedals_params.cloudburst.enabled
                    }
                };
                app.status_banner = format!(
                    "{} Footswitch: {}",
                    pid.name(),
                    if now_on { "ENGAGED (ON)" } else { "BYPASSED (OFF)" }
                );
            }
        }
        // Enter cycles Mode if on row 0, or toggles Footswitch
        KeyCode::Enter => {
            if let Ok(mut guard) = state.lock() {
                let pid = guard.engine.pedals_params.chain_order[app.selected_pedal_slot];
                if app.selected_pedal_param == 0 {
                    adjust_pedal_param(&mut guard, pid, 0, 1.0);
                    app.status_banner = format!("Cycled {} Mode/Algorithm", pid.name());
                } else {
                    match pid {
                        PedalId::BossCe2wChorus => {
                            guard.engine.pedals_params.chorus.enabled =
                                !guard.engine.pedals_params.chorus.enabled;
                        }
                        PedalId::ZoomMs70Cdr => {
                            guard.engine.pedals_params.zoom.enabled =
                                !guard.engine.pedals_params.zoom.enabled;
                        }
                        PedalId::BossDd8 => {
                            guard.engine.pedals_params.dd8.enabled =
                                !guard.engine.pedals_params.dd8.enabled;
                        }
                        PedalId::RedPandaParticle2 => {
                            guard.engine.pedals_params.particle.enabled =
                                !guard.engine.pedals_params.particle.enabled;
                        }
                        PedalId::StrymonCloudburst => {
                            guard.engine.pedals_params.cloudburst.enabled =
                                !guard.engine.pedals_params.cloudburst.enabled;
                        }
                    }
                }
            }
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            if let Ok(mut guard) = state.lock() {
                guard.engine.pedals_params.particle.freeze =
                    !guard.engine.pedals_params.particle.freeze;
                app.status_banner = format!(
                    "Red Panda Particle 2 FREEZE: {}",
                    if guard.engine.pedals_params.particle.freeze {
                        "FROZEN"
                    } else {
                        "LIVE"
                    }
                );
            }
        }
        KeyCode::Char('+')
        | KeyCode::Char('=')
        | KeyCode::Char(']')
        | KeyCode::Char('l')
        | KeyCode::PageUp
        | KeyCode::Char('-')
        | KeyCode::Char('[')
        | KeyCode::Char('h')
        | KeyCode::PageDown => {
            let delta = match code {
                KeyCode::PageUp => 1.0,
                KeyCode::PageDown => -1.0,
                KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') | KeyCode::Char('l') => 0.4,
                _ => -0.4,
            };
            if let Ok(mut guard) = state.lock() {
                let pid = guard.engine.pedals_params.chain_order[app.selected_pedal_slot];
                adjust_pedal_param(&mut guard, pid, app.selected_pedal_param, delta);
            }
        }
        _ => {}
    }
}

fn adjust_pedal_param(
    guard: &mut SharedSynthState,
    pid: PedalId,
    param_idx: usize,
    delta: f32,
) {
    let step_enum = |cur: usize, count: usize| -> f32 {
        if delta > 0.0 {
            ((cur + 1) % count) as f32
        } else {
            ((cur + count - 1) % count) as f32
        }
    };

    match pid {
        PedalId::BossCe2wChorus => match param_idx {
            0 => {
                let next = step_enum(guard.engine.pedals_params.chorus.mode as usize, 4);
                guard.engine.set_parameter(141, next);
            }
            1 => {
                let v = guard.engine.pedals_params.chorus.rate + delta;
                guard.engine.set_parameter(142, v);
            }
            2 => {
                let v = guard.engine.pedals_params.chorus.depth + delta;
                guard.engine.set_parameter(143, v);
            }
            3 => {
                let v = guard.engine.pedals_params.chorus.stereo_width + delta;
                guard.engine.set_parameter(144, v);
            }
            4 => {
                let v = guard.engine.pedals_params.chorus.warmth + delta;
                guard.engine.set_parameter(145, v);
            }
            _ => {
                let v = guard.engine.pedals_params.chorus.mix + delta;
                guard.engine.set_parameter(146, v);
            }
        },
        PedalId::ZoomMs70Cdr => match param_idx {
            0 => {
                let next = step_enum(guard.engine.pedals_params.zoom.mode as usize, 4);
                guard.engine.set_parameter(101, next);
            }
            1 => {
                let v = guard.engine.pedals_params.zoom.depth + delta;
                guard.engine.set_parameter(102, v);
            }
            2 => {
                let v = guard.engine.pedals_params.zoom.rate + delta;
                guard.engine.set_parameter(103, v);
            }
            3 => {
                let v = guard.engine.pedals_params.zoom.tone + delta;
                guard.engine.set_parameter(104, v);
            }
            _ => {
                let v = guard.engine.pedals_params.zoom.mix + delta;
                guard.engine.set_parameter(105, v);
            }
        },
        PedalId::BossDd8 => match param_idx {
            0 => {
                let next = step_enum(guard.engine.pedals_params.dd8.mode as usize, 5);
                guard.engine.set_parameter(111, next);
            }
            1 => {
                let v = guard.engine.pedals_params.dd8.effect_level + delta;
                guard.engine.set_parameter(112, v);
            }
            2 => {
                let v = guard.engine.pedals_params.dd8.feedback + delta;
                guard.engine.set_parameter(113, v);
            }
            3 => {
                let v = guard.engine.pedals_params.dd8.delay_time + delta;
                guard.engine.set_parameter(114, v);
            }
            _ => {
                let v = guard.engine.pedals_params.dd8.mod_depth + delta;
                guard.engine.set_parameter(115, v);
            }
        },
        PedalId::RedPandaParticle2 => match param_idx {
            0 => {
                let next = step_enum(guard.engine.pedals_params.particle.mode as usize, 5);
                guard.engine.set_parameter(121, next);
            }
            1 => {
                let v = guard.engine.pedals_params.particle.blend + delta;
                guard.engine.set_parameter(122, v);
            }
            2 => {
                let v = guard.engine.pedals_params.particle.chop + delta;
                guard.engine.set_parameter(123, v);
            }
            3 => {
                let v = guard.engine.pedals_params.particle.pitch_detune + delta;
                guard.engine.set_parameter(124, v);
            }
            4 => {
                let v = guard.engine.pedals_params.particle.grain_size + delta;
                guard.engine.set_parameter(125, v);
            }
            5 => {
                let v = guard.engine.pedals_params.particle.feedback + delta;
                guard.engine.set_parameter(126, v);
            }
            _ => {
                let next = if guard.engine.pedals_params.particle.freeze {
                    0.0
                } else {
                    1.0
                };
                guard.engine.set_parameter(127, next);
            }
        },
        PedalId::StrymonCloudburst => match param_idx {
            0 => {
                let next = step_enum(guard.engine.pedals_params.cloudburst.ensemble_mode as usize, 3);
                guard.engine.set_parameter(131, next);
            }
            1 => {
                let v = guard.engine.pedals_params.cloudburst.decay + delta;
                guard.engine.set_parameter(132, v);
            }
            2 => {
                let v = guard.engine.pedals_params.cloudburst.ensemble + delta;
                guard.engine.set_parameter(133, v);
            }
            3 => {
                let v = guard.engine.pedals_params.cloudburst.mix + delta;
                guard.engine.set_parameter(134, v);
            }
            4 => {
                let v = guard.engine.pedals_params.cloudburst.tone + delta;
                guard.engine.set_parameter(135, v);
            }
            5 => {
                let v = guard.engine.pedals_params.cloudburst.pre_delay + delta;
                guard.engine.set_parameter(136, v);
            }
            _ => {
                let v = guard.engine.pedals_params.cloudburst.mod_speed + delta;
                guard.engine.set_parameter(137, v);
            }
        },
    }
}
