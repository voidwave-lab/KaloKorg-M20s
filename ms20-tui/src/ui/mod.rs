pub mod pedals_panel;
pub mod synth_panel;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use ms20_core::{
    engine::Preset,
    patchbay::{PatchDest, PatchSource},
    AudioProcessor,
};
use crate::audio_host::SharedSynthState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveTab {
    SynthPanel = 0,
    Pedalboard = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynthFocusArea {
    Knobs,
    Patchbay,
}

pub struct TuiApp {
    pub active_tab: ActiveTab,
    pub synth_focus: SynthFocusArea,
    pub selected_knob_idx: usize, // 0..36 on Synth Panel
    pub selected_patch_src: usize,
    pub selected_patch_dst: usize,
    pub selected_pedal_slot: usize, // 0..3 in current chain order
    pub selected_pedal_param: usize, // 0..6 within selected pedal
    pub presets: Vec<Preset>,
    pub web_url: String,
    pub status_banner: String,
    pub last_audition_note: Option<u8>,
}

impl TuiApp {
    pub fn new(web_port: u16) -> Self {
        Self {
            active_tab: ActiveTab::SynthPanel,
            synth_focus: SynthFocusArea::Knobs,
            selected_knob_idx: 16, // Start on LPF Cutoff
            selected_patch_src: 2,  // MG TRI
            selected_patch_dst: 0,  // TOTAL IN
            selected_pedal_slot: 0,
            selected_pedal_param: 0,
            presets: Preset::factory_presets(),
            web_url: format!("http://127.0.0.1:{}", web_port),
            status_banner: "Use Arrows/hjkl to navigate, Up/Down or +/- to turn knobs, [Tab] for FX Pedals, [D] Drone, [R] Record WAV".to_string(),
            last_audition_note: None,
        }
    }

    pub fn adjust_selected_synth_knob(&mut self, state: &mut SharedSynthState, delta: f32) {
        let idx = self.selected_knob_idx;
        let p = state.engine.params.clone();
        match idx {
            0 => {
                let next = (p.vco1_waveform as i32 + if delta > 0.0 { 1 } else { 3 }) as usize % 4;
                state.engine.set_parameter(0, next as f32);
            }
            1 => state.engine.set_parameter(1, p.vco1_pw + delta),
            2 => {
                let next = (p.vco1_scale as i32 + if delta > 0.0 { 1 } else { 3 }) as usize % 4;
                state.engine.set_parameter(2, next as f32);
            }
            3 => state.engine.set_parameter(3, p.portamento_time + delta),
            4 => {
                let next = (p.vco2_waveform as i32 + if delta > 0.0 { 1 } else { 3 }) as usize % 4;
                state.engine.set_parameter(4, next as f32);
            }
            5 => state.engine.set_parameter(5, p.vco2_pitch + delta),
            6 => {
                let next = (p.vco2_scale as i32 + if delta > 0.0 { 1 } else { 3 }) as usize % 4;
                state.engine.set_parameter(6, next as f32);
            }
            7 => state.engine.set_parameter(7, p.master_tune + delta),
            8 => state.engine.set_parameter(8, p.vco1_level + delta),
            9 => state.engine.set_parameter(9, p.vco2_level + delta),
            10 => state.engine.set_parameter(10, p.fm_mg_text + delta),
            11 => state.engine.set_parameter(11, p.fm_eg1_ext + delta),
            12 => state.engine.set_parameter(12, p.hpf_cutoff + delta),
            13 => state.engine.set_parameter(13, p.hpf_peak + delta),
            14 => state.engine.set_parameter(14, p.hpf_mg_text + delta),
            15 => state.engine.set_parameter(15, p.hpf_eg2_ext + delta),
            16 => state.engine.set_parameter(16, p.lpf_cutoff + delta),
            17 => state.engine.set_parameter(17, p.lpf_peak + delta),
            18 => state.engine.set_parameter(18, p.lpf_mg_text + delta),
            19 => state.engine.set_parameter(19, p.lpf_eg2_ext + delta),
            20 => state.engine.set_parameter(20, p.mg_waveform + delta),
            21 => state.engine.set_parameter(21, p.mg_frequency + delta),
            22 => state.engine.set_parameter(22, p.eg1_delay + delta),
            23 => state.engine.set_parameter(23, p.eg1_attack + delta),
            24 => state.engine.set_parameter(24, p.eg1_release + delta),
            25 => state.engine.set_parameter(25, p.eg2_hold + delta),
            26 => state.engine.set_parameter(26, p.eg2_attack + delta),
            27 => state.engine.set_parameter(27, p.eg2_decay + delta),
            28 => state.engine.set_parameter(28, p.eg2_sustain + delta),
            29 => state.engine.set_parameter(29, p.eg2_release + delta),
            30 => state.engine.set_parameter(30, p.master_volume + delta),
            31 => state.engine.set_parameter(31, p.esp_signal_level + delta),
            32 => state.engine.set_parameter(32, p.esp_low_cut + delta),
            33 => state.engine.set_parameter(33, p.esp_high_cut + delta),
            34 => state.engine.set_parameter(34, p.esp_cv_adjust + delta),
            35 => state.engine.set_parameter(35, p.esp_threshold + delta),
            _ => {}
        }
    }

    pub fn connect_selected_patch(&mut self, state: &mut SharedSynthState) {
        let src = PatchSource::from_index(self.selected_patch_src);
        let dst = PatchDest::from_index(self.selected_patch_dst);
        state.engine.patchbay.connect(src, dst);
        self.status_banner = format!("Patched 3.5mm cable: [{}] ---> [{}]", src.label(), dst.label());
    }

    pub fn disconnect_selected_patch(&mut self, state: &mut SharedSynthState) {
        let dst = PatchDest::from_index(self.selected_patch_dst);
        state.engine.patchbay.disconnect_dest(dst);
        self.status_banner = format!("Unplugged cable from [{}]", dst.label());
    }
}

pub fn render_ui(frame: &mut Frame, app: &TuiApp, state: &SharedSynthState) {
    let size = frame.area();

    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top Header & Tab Bar
            Constraint::Min(20),   // Main Synth Panel or Guitar Pedalboard
            Constraint::Length(3), // Bottom Status, MIDI, Recorder & Help Bar
        ])
        .split(size);

    render_header(frame, root_chunks[0], app, state);

    match app.active_tab {
        ActiveTab::SynthPanel => {
            synth_panel::render_synth_panel(frame, root_chunks[1], app, state);
        }
        ActiveTab::Pedalboard => {
            pedals_panel::render_pedals_panel(frame, root_chunks[1], app, state);
        }
    }

    render_footer(frame, root_chunks[2], app, state);
}

fn render_header(frame: &mut Frame, area: Rect, app: &TuiApp, state: &SharedSynthState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(48),
            Constraint::Min(28),
            Constraint::Length(44),
        ])
        .split(area);

    let tab_titles = vec![
        Line::from(vec![
            Span::styled(" 1: MS-20 SYNTH PANEL ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(" 2: AMBIENT FX PEDALS ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " KALOKORG M20s ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .select(app.active_tab as usize)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, cols[0]);

    // Center: Preset & Circuit Revision Info
    let preset_name = app
        .presets
        .get(state.selected_preset_idx)
        .map(|p| p.name.as_str())
        .unwrap_or("Custom Patch");
    let rev_label = state.engine.params.filter_revision.label();
    let drone_badge = if state.engine.params.drone_gate {
        Span::styled(" [DRONE GATE: ON] ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [DRONE: OFF (D)] ", Style::default().fg(Color::DarkGray))
    };

    let pad_db = state.engine.params.output_pad_db;
    let center_para = Paragraph::new(Line::from(vec![
        Span::styled("Preset(P): ", Style::default().fg(Color::Gray)),
        Span::styled(preset_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled(rev_label, Style::default().fg(Color::LightYellow)),
        Span::raw(" | "),
        Span::styled(
            format!("PAD(9/0): {:+.1}dB ", pad_db),
            Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD),
        ),
        drone_badge,
    ]))
    .block(Block::default().borders(Borders::ALL).title(" PATCH / IC TOPOLOGY (V) / OUTPUT PAD (9/0) "));
    frame.render_widget(center_para, cols[1]);

    // Right: Session Recorder & Web UI Status
    let rec_spans = if state.recorder.is_recording {
        vec![
            Span::styled(
                format!(" ● REC {:.1}s (Press R to Save) ", state.recorder.duration_seconds()),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
        ]
    } else if let Some(ref saved) = state.recorder.last_saved_path {
        vec![
            Span::styled("[R] Rec ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(format!("Saved: {}", saved), Style::default().fg(Color::Green)),
        ]
    } else {
        vec![
            Span::styled("[R] Start WAV Rec ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
            Span::raw("| "),
            Span::styled(format!("Web: {}", app.web_url), Style::default().fg(Color::Cyan)),
        ]
    };

    let right_para = Paragraph::new(Line::from(rec_spans))
        .block(Block::default().borders(Borders::ALL).title(" SESSION RECORDER & WEB UI "));
    frame.render_widget(right_para, cols[2]);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &TuiApp, state: &SharedSynthState) {
    let midi_port_str = if state.midi_ports.is_empty() {
        "No HW MIDI Detected (Press M to Rescan | Z-M/Q-U Audition Notes)".to_string()
    } else {
        match state.active_midi_port {
            None => format!("ALL MIDI ({} dev: {})", state.midi_ports.len(), state.midi_ports[0]),
            Some(i) => state
                .midi_ports
                .get(i)
                .cloned()
                .unwrap_or_else(|| "ALL MIDI".to_string()),
        }
    };

    let meter_l = (state.engine.peak_l * 12.0).round() as usize;
    let meter_r = (state.engine.peak_r * 12.0).round() as usize;
    let bar_l: String = "█".repeat(meter_l.min(12)) + &"░".repeat(12 - meter_l.min(12));
    let bar_r: String = "█".repeat(meter_r.min(12)) + &"░".repeat(12 - meter_r.min(12));

    let footer_line = Line::from(vec![
        Span::styled(" MIDI: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(midi_port_str, Style::default().fg(Color::White)),
        Span::raw(" | "),
        Span::styled(&state.last_midi_message, Style::default().fg(Color::LightGreen)),
        Span::raw(" | "),
        Span::styled(format!("OUT L[{}] R[{}]", bar_l, bar_r), Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled(&app.status_banner, Style::default().fg(Color::Gray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [Tab] Synth/Pedals | [Arrows] Select | [+/-] Turn Knob | [9/0] Out Pad dB | [Space] Patch/Bypass | [D] Drone | [P] Preset | [R] Rec | [Q] Quit ");
    frame.render_widget(Paragraph::new(footer_line).block(block), area);
}
