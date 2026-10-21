use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use ms20_core::patchbay::{PatchDest, PatchSource};
use crate::audio_host::SharedSynthState;
use crate::ui::{SynthFocusArea, TuiApp};

/// Helper to format an MS-20 rotary knob with ASCII/Unicode dial arc and value indicator
fn knob_lines(
    label: &str,
    val_str: &str,
    norm_0_to_1: f32,
    is_selected: bool,
) -> Vec<Line<'static>> {
    let clamped = norm_0_to_1.clamp(0.0, 1.0);
    let ticks = (clamped * 8.0).round() as usize;
    let pointer = match ticks {
        0 => "↙",
        1 => "←",
        2 => "↖",
        3 => "↑",
        4 => "↑",
        5 => "↗",
        6 => "→",
        7 => "↘",
        _ => "↘",
    };
    let bar: String = "━".repeat(ticks) + "●" + &"─".repeat(8usize.saturating_sub(ticks));

    let title_style = if is_selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    };

    let dial_style = if is_selected {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };

    vec![
        Line::from( vec![
            Span::styled(format!(" {:<11} ", label), title_style),
        ]),
        Line::from(vec![
            Span::styled(format!(" ({}) {:>7} ", pointer, val_str), dial_style),
        ]),
        Line::from(vec![
            Span::styled(format!(" [{}]", bar), Style::default().fg(if is_selected { Color::LightYellow } else { Color::DarkGray })),
        ]),
    ]
}

pub fn render_synth_panel(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    state: &SharedSynthState,
) {
    // Split main panel horizontally matching `front-panel.jpg`:
    // Left ~58%: 7 columns of MS-20 synthesis controls
    // Right ~42%: Top Semi-Modular Patchbay + Master Volume, Bottom ESP (External Signal Processor)
    let main_halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    render_left_knob_columns(frame, main_halves[0], app, state);
    render_right_patchbay_and_esp(frame, main_halves[1], app, state);
}

fn render_left_knob_columns(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    state: &SharedSynthState,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 7), // Col 1: VCO 1 + PORTAMENTO
            Constraint::Ratio(1, 7), // Col 2: VCO 2 + MASTER TUNE
            Constraint::Ratio(1, 7), // Col 3: VCO MIXER + FREQ MOD
            Constraint::Ratio(1, 7), // Col 4: HPF + HPF MOD
            Constraint::Ratio(1, 7), // Col 5: LPF + LPF MOD
            Constraint::Ratio(1, 7), // Col 6: VCA + MG + ENV 1
            Constraint::Ratio(1, 7), // Col 7: ENV 2 (HADSR)
        ])
        .split(area);

    let p = &state.engine.params;
    let focus_knobs = app.synth_focus == SynthFocusArea::Knobs;
    let sel = |idx: usize| focus_knobs && app.selected_knob_idx == idx;

    // --- COLUMN 1: VCO 1 & PORTAMENTO ---
    let c1_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(74), Constraint::Percentage(26)])
        .split(cols[0]);
    let mut vco1_lines = Vec::new();
    vco1_lines.extend(knob_lines(
        "WAVE FORM",
        p.vco1_waveform.label(),
        (p.vco1_waveform as usize as f32) / 3.0,
        sel(0),
    ));
    vco1_lines.push(Line::raw(""));
    vco1_lines.extend(knob_lines(
        "PW",
        &format!("{:.1}", p.vco1_pw),
        p.vco1_pw / 10.0,
        sel(1),
    ));
    vco1_lines.push(Line::raw(""));
    vco1_lines.extend(knob_lines(
        "SCALE",
        p.vco1_scale.label(),
        (p.vco1_scale as usize as f32) / 3.0,
        sel(2),
    ));
    frame.render_widget(
        Paragraph::new(vco1_lines).block(Block::default().borders(Borders::ALL).title(" VCO 1 ")),
        c1_split[0],
    );

    let port_lines = knob_lines(
        "TIME",
        &format!("{:.1}", p.portamento_time),
        p.portamento_time / 10.0,
        sel(3),
    );
    frame.render_widget(
        Paragraph::new(port_lines)
            .block(Block::default().borders(Borders::ALL).title(" PORTAMENTO ")),
        c1_split[1],
    );

    // --- COLUMN 2: VCO 2 & MASTER TUNE ---
    let c2_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(74), Constraint::Percentage(26)])
        .split(cols[1]);
    let mut vco2_lines = Vec::new();
    vco2_lines.extend(knob_lines(
        "WAVE FORM",
        p.vco2_waveform.label(),
        (p.vco2_waveform as usize as f32) / 3.0,
        sel(4),
    ));
    vco2_lines.push(Line::raw(""));
    vco2_lines.extend(knob_lines(
        "PITCH",
        &format!("{:+.1}", p.vco2_pitch),
        (p.vco2_pitch + 5.0) / 10.0,
        sel(5),
    ));
    vco2_lines.push(Line::raw(""));
    vco2_lines.extend(knob_lines(
        "SCALE",
        p.vco2_scale.label(),
        (p.vco2_scale as usize as f32) / 3.0,
        sel(6),
    ));
    frame.render_widget(
        Paragraph::new(vco2_lines).block(Block::default().borders(Borders::ALL).title(" VCO 2 ")),
        c2_split[0],
    );

    let tune_lines = knob_lines(
        "TUNE",
        &format!("{:+.1}", p.master_tune),
        (p.master_tune + 5.0) / 10.0,
        sel(7),
    );
    frame.render_widget(
        Paragraph::new(tune_lines)
            .block(Block::default().borders(Borders::ALL).title(" MASTER TUNE ")),
        c2_split[1],
    );

    // --- COLUMN 3: VCO MIXER & FREQ MOD ---
    let c3_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[2]);
    let mut mix_lines = Vec::new();
    mix_lines.extend(knob_lines(
        "VCO 1 LVL",
        &format!("{:.1}", p.vco1_level),
        p.vco1_level / 10.0,
        sel(8),
    ));
    mix_lines.push(Line::raw(""));
    mix_lines.extend(knob_lines(
        "VCO 2 LVL",
        &format!("{:.1}", p.vco2_level),
        p.vco2_level / 10.0,
        sel(9),
    ));
    frame.render_widget(
        Paragraph::new(mix_lines)
            .block(Block::default().borders(Borders::ALL).title(" VCO MIXER ")),
        c3_split[0],
    );

    let mut fm_lines = Vec::new();
    fm_lines.extend(knob_lines(
        "MG/T.EXT",
        &format!("{:.1}", p.fm_mg_text),
        p.fm_mg_text / 10.0,
        sel(10),
    ));
    fm_lines.push(Line::raw(""));
    fm_lines.extend(knob_lines(
        "EG1/EXT",
        &format!("{:.1}", p.fm_eg1_ext),
        p.fm_eg1_ext / 10.0,
        sel(11),
    ));
    frame.render_widget(
        Paragraph::new(fm_lines)
            .block(Block::default().borders(Borders::ALL).title(" FREQ MOD ")),
        c3_split[1],
    );

    // --- COLUMN 4: HPF & HPF MOD ---
    let c4_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[3]);
    let mut hpf_lines = Vec::new();
    hpf_lines.extend(knob_lines(
        "CUTOFF",
        &format!("{:.1}", p.hpf_cutoff),
        p.hpf_cutoff / 10.0,
        sel(12),
    ));
    hpf_lines.push(Line::raw(""));
    hpf_lines.extend(knob_lines(
        "PEAK",
        &format!("{:.1}", p.hpf_peak),
        p.hpf_peak / 10.0,
        sel(13),
    ));
    frame.render_widget(
        Paragraph::new(hpf_lines).block(Block::default().borders(Borders::ALL).title(" VC HPF ")),
        c4_split[0],
    );

    let mut hpf_mod_lines = Vec::new();
    hpf_mod_lines.extend(knob_lines(
        "MG/T.EXT",
        &format!("{:.1}", p.hpf_mg_text),
        p.hpf_mg_text / 10.0,
        sel(14),
    ));
    hpf_mod_lines.push(Line::raw(""));
    hpf_mod_lines.extend(knob_lines(
        "EG2/EXT",
        &format!("{:.1}", p.hpf_eg2_ext),
        p.hpf_eg2_ext / 10.0,
        sel(15),
    ));
    frame.render_widget(
        Paragraph::new(hpf_mod_lines)
            .block(Block::default().borders(Borders::ALL).title(" HPF MOD ")),
        c4_split[1],
    );

    // --- COLUMN 5: LPF & LPF MOD ---
    let c5_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[4]);
    let mut lpf_lines = Vec::new();
    lpf_lines.extend(knob_lines(
        "CUTOFF",
        &format!("{:.1}", p.lpf_cutoff),
        p.lpf_cutoff / 10.0,
        sel(16),
    ));
    lpf_lines.push(Line::raw(""));
    lpf_lines.extend(knob_lines(
        "PEAK",
        &format!("{:.1}", p.lpf_peak),
        p.lpf_peak / 10.0,
        sel(17),
    ));
    frame.render_widget(
        Paragraph::new(lpf_lines).block(Block::default().borders(Borders::ALL).title(" VC LPF ")),
        c5_split[0],
    );

    let mut lpf_mod_lines = Vec::new();
    lpf_mod_lines.extend(knob_lines(
        "MG/T.EXT",
        &format!("{:.1}", p.lpf_mg_text),
        p.lpf_mg_text / 10.0,
        sel(18),
    ));
    lpf_mod_lines.push(Line::raw(""));
    lpf_mod_lines.extend(knob_lines(
        "EG2/EXT",
        &format!("{:.1}", p.lpf_eg2_ext),
        p.lpf_eg2_ext / 10.0,
        sel(19),
    ));
    frame.render_widget(
        Paragraph::new(lpf_mod_lines)
            .block(Block::default().borders(Borders::ALL).title(" LPF MOD ")),
        c5_split[1],
    );

    // --- COLUMN 6: VCA (top), MG & ENV 1 (bottom) ---
    let c6_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(24), // VCA Diagram + Live Scope
            Constraint::Percentage(34), // MG (Modulation Generator)
            Constraint::Percentage(42), // ENV 1 (DAR)
        ])
        .split(cols[5]);

    // VCA schematic symbol as on `front-panel.jpg`
    let vca_lines = vec![
        Line::from(Span::styled("  ───|\\───  ", Style::default().fg(Color::Yellow))),
        Line::from(Span::styled("     | >    ", Style::default().fg(Color::Yellow))),
        Line::from(Span::styled("  ───|/───  ", Style::default().fg(Color::Yellow))),
        Line::from(Span::styled("  EG2 & EXT ", Style::default().fg(Color::Gray))),
    ];
    frame.render_widget(
        Paragraph::new(vca_lines).block(Block::default().borders(Borders::ALL).title(" VCA ")),
        c6_split[0],
    );

    let mut mg_lines = Vec::new();
    mg_lines.extend(knob_lines(
        "WAVEFORM",
        &format!("{:.1}", p.mg_waveform),
        p.mg_waveform / 10.0,
        sel(20),
    ));
    mg_lines.extend(knob_lines(
        "FREQUENCY",
        &format!("{:.1}", p.mg_frequency),
        p.mg_frequency / 10.0,
        sel(21),
    ));
    frame.render_widget(
        Paragraph::new(mg_lines).block(Block::default().borders(Borders::ALL).title(" MG ")),
        c6_split[1],
    );

    let mut eg1_lines = Vec::new();
    eg1_lines.extend(knob_lines(
        "DELAY",
        &format!("{:.1}", p.eg1_delay),
        p.eg1_delay / 10.0,
        sel(22),
    ));
    eg1_lines.extend(knob_lines(
        "ATTACK",
        &format!("{:.1}", p.eg1_attack),
        p.eg1_attack / 10.0,
        sel(23),
    ));
    eg1_lines.extend(knob_lines(
        "RELEASE",
        &format!("{:.1}", p.eg1_release),
        p.eg1_release / 10.0,
        sel(24),
    ));
    frame.render_widget(
        Paragraph::new(eg1_lines).block(Block::default().borders(Borders::ALL).title(" ENV 1 ")),
        c6_split[2],
    );

    // --- COLUMN 7: ENV 2 (HADSR) ---
    let mut eg2_lines = Vec::new();
    eg2_lines.extend(knob_lines(
        "HOLD",
        &format!("{:.1}", p.eg2_hold),
        p.eg2_hold / 10.0,
        sel(25),
    ));
    eg2_lines.extend(knob_lines(
        "ATTACK",
        &format!("{:.1}", p.eg2_attack),
        p.eg2_attack / 10.0,
        sel(26),
    ));
    eg2_lines.extend(knob_lines(
        "DECAY",
        &format!("{:.1}", p.eg2_decay),
        p.eg2_decay / 10.0,
        sel(27),
    ));
    eg2_lines.extend(knob_lines(
        "SUSTAIN",
        &format!("{:.1}", p.eg2_sustain),
        p.eg2_sustain / 10.0,
        sel(28),
    ));
    eg2_lines.extend(knob_lines(
        "RELEASE",
        &format!("{:.1}", p.eg2_release),
        p.eg2_release / 10.0,
        sel(29),
    ));
    frame.render_widget(
        Paragraph::new(eg2_lines).block(Block::default().borders(Borders::ALL).title(" ENV 2 ")),
        cols[6],
    );
}

fn render_right_patchbay_and_esp(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    state: &SharedSynthState,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(68), // Top: Semi-Modular Patchbay + Master Volume
            Constraint::Percentage(32), // Bottom: ESP (External Signal Processor)
        ])
        .split(area);

    let p = &state.engine.params;
    let focus_patch = app.synth_focus == SynthFocusArea::Patchbay;
    let focus_knobs = app.synth_focus == SynthFocusArea::Knobs;

    // --- TOP RIGHT: SEMI-MODULAR PATCHBAY & MASTER VOLUME ---
    let mut patch_lines = Vec::new();

    // MS-20 Front Panel Signal Path Block Diagram + Master Volume (Knob 30)
    let vol_selected = focus_knobs && app.selected_knob_idx == 30;
    let vol_style = if vol_selected {
        Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    };
    patch_lines.push(Line::from(vec![
        Span::styled(
            " [VCO1]─>[VCO2]─>[MIX]─>[HPF]─>[LPF]─>[VCA]───> ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" VOLUME: {:.1} ", p.master_volume), vol_style),
        Span::styled(" ◯ SIG OUT", Style::default().fg(Color::White)),
    ]));
    patch_lines.push(Line::from(vec![
        Span::styled(
            " Press [J] to Toggle Knob/Patchbay Focus | [Enter/Space] Connect Cable | [Backspace/X] Unplug ",
            Style::default().fg(if focus_patch { Color::LightGreen } else { Color::Gray }),
        ),
    ]));

    // Render Source Jacks (Left column) & Destination Jacks (Right column) side-by-side
    let max_rows = 12usize;
    for r in 0..max_rows {
        let src_opt = PatchSource::ALL.get(r);
        let dst_opt = PatchDest::ALL.get(r);

        let src_span = if let Some(src) = src_opt {
            let is_sel = focus_patch && app.selected_patch_src == r;
            let style = if is_sel {
                Style::default().fg(Color::Black).bg(Color::LightGreen).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            Span::styled(format!(" ◯ {:<16}", src.label()), style)
        } else {
            Span::raw(format!("   {:<16}", ""))
        };

        let dst_span = if let Some(dst) = dst_opt {
            let is_sel = focus_patch && app.selected_patch_dst == r;
            let plugged = state
                .engine
                .patchbay
                .cables
                .iter()
                .find(|c| c.dest == *dst);
            let badge = if let Some(c) = plugged {
                format!("◉ {:<15} <── [{}]", dst.label(), c.source.label())
            } else {
                format!("◯ {:<15}", dst.label())
            };
            let style = if is_sel {
                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if plugged.is_some() {
                Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Span::styled(badge, style)
        } else {
            Span::raw("")
        };

        patch_lines.push(Line::from(vec![src_span, Span::raw("  │  "), dst_span]));
    }

    // Active Virtual Cables summary
    if state.engine.patchbay.cables.is_empty() {
        patch_lines.push(Line::from(Span::styled(
            " Normalled MS-20 Internal Routing Active (0 external 3.5mm cables patched)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let summary: Vec<String> = state
            .engine
            .patchbay
            .cables
            .iter()
            .map(|c| format!("{}->{}", c.source.label(), c.dest.label()))
            .collect();
        patch_lines.push(Line::from(Span::styled(
            format!(" Active Cables ({}): {}", summary.len(), summary.join(" , ")),
            Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD),
        )));
    }

    let patch_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focus_patch {
            Style::default().fg(Color::LightGreen)
        } else {
            Style::default().fg(Color::White)
        })
        .title(" SEMI-MODULAR PATCHBAY (35 JACKS — S&H • MVCA • NOISE • CV/TRIG) [Press J] ");
    frame.render_widget(Paragraph::new(patch_lines).block(patch_block), rows[0]);

    // --- BOTTOM RIGHT: EXTERNAL SIGNAL PROCESSOR (ESP) ---
    let esp_block = Block::default()
        .borders(Borders::ALL)
        .title(" EXTERNAL SIGNAL PROCESSOR (ESP) — KORG MS-20 ");
    let esp_inner = esp_block.inner(rows[1]);
    frame.render_widget(esp_block, rows[1]);

    let esp_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(4)])
        .split(esp_inner);

    let peak_led = if state.engine.esp.peak_led { "● PEAK" } else { "○ PEAK" };
    let trig_led = if state.engine.esp.trig_led { "● TRIG" } else { "○ TRIG" };

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    " ◯ SIG IN ─>[AMP]─>◯─>[BAND PASS]─>◯─>[F-V CONV]─>◯─>[ENV FOLLOWER]─>◯ ENV / ◯ TRIG ",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("   ESP Status: {}   {} ", peak_led, trig_led), Style::default().fg(Color::LightRed)),
                Span::styled("   KORG MS-20s", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
        ]),
        esp_rows[0],
    );

    let esp_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ])
        .split(esp_rows[1]);

    let esp_knobs = [
        ("SIG LEVEL", p.esp_signal_level, 31usize),
        ("LOW CUT", p.esp_low_cut, 32usize),
        ("HIGH CUT", p.esp_high_cut, 33usize),
        ("CV ADJUST", p.esp_cv_adjust, 34usize),
        ("THRESHOLD", p.esp_threshold, 35usize),
    ];

    for (i, (label, val, k_idx)) in esp_knobs.iter().enumerate() {
        let lines = knob_lines(
            label,
            &format!("{:.1}", val),
            val / 10.0,
            focus_knobs && app.selected_knob_idx == *k_idx,
        );
        frame.render_widget(Paragraph::new(lines), esp_cols[i]);
    }
}
