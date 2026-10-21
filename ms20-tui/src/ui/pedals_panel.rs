use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use ms20_core::params::PedalId;
use crate::audio_host::SharedSynthState;
use crate::ui::TuiApp;

pub fn render_pedals_panel(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    state: &SharedSynthState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Top Signal Chain Order & Routing Diagram
            Constraint::Min(16),   // 4 Stompbox Guitar Pedals Side-by-Side
        ])
        .split(area);

    render_chain_diagram(frame, chunks[0], app, state);

    let pedal_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ])
        .split(chunks[1]);

    for slot in 0..5 {
        let pedal_id = state.engine.pedals_params.chain_order[slot];
        let is_selected_pedal = app.selected_pedal_slot == slot;
        render_single_stompbox(
            frame,
            pedal_cols[slot],
            slot,
            pedal_id,
            is_selected_pedal,
            app.selected_pedal_param,
            state,
        );
    }
}

fn render_chain_diagram(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    state: &SharedSynthState,
) {
    let mut chain_spans = vec![Span::styled(
        " [MS-20 MONO]═>",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )];

    for (slot, pedal_id) in state.engine.pedals_params.chain_order.iter().enumerate() {
        let enabled = match pedal_id {
            PedalId::BossCe2wChorus => state.engine.pedals_params.chorus.enabled,
            PedalId::ZoomMs70Cdr => state.engine.pedals_params.zoom.enabled,
            PedalId::BossDd8 => state.engine.pedals_params.dd8.enabled,
            PedalId::RedPandaParticle2 => state.engine.pedals_params.particle.enabled,
            PedalId::StrymonCloudburst => state.engine.pedals_params.cloudburst.enabled,
        };
        let is_sel = app.selected_pedal_slot == slot;
        let status_dot = if enabled { "●" } else { "○" };
        let style = if is_sel {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if enabled {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        chain_spans.push(Span::styled(
            format!(" {}.{} {} ", slot + 1, status_dot, pedal_id.name()),
            style,
        ));
        chain_spans.push(Span::styled(
            "═>",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    chain_spans.push(Span::styled(
        " [STEREO OUT]",
        Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD),
    ));

    let help_line = Line::from(vec![
        Span::styled(
            " Controls: [Left/Right] Select Pedal | [ < / > ] Reorder Pedal in Effect Chain | [Space] Stomp Footswitch ON/OFF | [Up/Down] Select Knob | [+/-] Turn Knob ",
            Style::default().fg(Color::Gray),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" AMBIENT GUITAR PEDALBOARD SIGNAL CHAIN (CUSTOMIZABLE ORDER & BYPASS) ");
    frame.render_widget(
        Paragraph::new(vec![Line::from(chain_spans), help_line]).block(block),
        area,
    );
}

fn render_single_stompbox(
    frame: &mut Frame,
    area: Rect,
    slot: usize,
    pedal_id: PedalId,
    is_selected_pedal: bool,
    selected_param: usize,
    state: &SharedSynthState,
) {
    let pp = &state.engine.pedals_params;

    let (border_color, enabled, led_str, params_list, extra_badge) = match pedal_id {
        PedalId::BossCe2wChorus => {
            let c = &pp.chorus;
            (
                Color::LightCyan, // BOSS CE-2W Metallic Sky Blue / DC-2W Violet-Blue
                c.enabled,
                if c.enabled { "● WAZA STEREO ON" } else { "○ BYPASSED      " },
                vec![
                    ("CHORUS MODE", c.mode.label().to_string(), (c.mode as usize as f32) / 3.0),
                    ("RATE (SPEED)", format!("{:.1}", c.rate), c.rate / 10.0),
                    ("DEPTH", format!("{:.1}", c.depth), c.depth / 10.0),
                    ("STEREO WIDTH", format!("{:.1}", c.stereo_width), c.stereo_width / 10.0),
                    ("BBD WARMTH", format!("{:.1}", c.warmth), c.warmth / 10.0),
                    ("MIX (STEREO)", format!("{:.1}", c.mix), c.mix / 10.0),
                ],
                "Mono->Stereo Chorus & Dimension",
            )
        }
        PedalId::ZoomMs70Cdr => {
            let z = &pp.zoom;
            (
                Color::Cyan, // Metallic Aqua Blue
                z.enabled,
                if z.enabled { "● CHECK LED: ON " } else { "○ BYPASSED      " },
                vec![
                    ("ALGORITHM", z.mode.label().to_string(), (z.mode as usize as f32) / 3.0),
                    ("DEPTH / PITCH", format!("{:.1}", z.depth), z.depth / 10.0),
                    ("RATE / TIME", format!("{:.1}", z.rate), z.rate / 10.0),
                    ("TONE / HI-DMP", format!("{:.1}", z.tone), z.tone / 10.0),
                    ("MIX (DRY/WET)", format!("{:.1}", z.mix), z.mix / 10.0),
                ],
                "MultiStomp Chorus•Delay•Reverb",
            )
        }
        PedalId::BossDd8 => {
            let d = &pp.dd8;
            (
                Color::White, // Classic Boss White/Silver enclosure
                d.enabled,
                if d.enabled { "● CHECK (RED)   " } else { "○ BYPASSED      " },
                vec![
                    ("MODE", d.mode.label().to_string(), (d.mode as usize as f32) / 4.0),
                    ("E.LEVEL (MIX)", format!("{:.1}", d.effect_level), d.effect_level / 10.0),
                    ("F.BACK", format!("{:.1}", d.feedback), d.feedback / 10.0),
                    ("DELAY TIME", format!("{:.1}", d.delay_time), d.delay_time / 10.0),
                    ("MOD / TONE", format!("{:.1}", d.mod_depth), d.mod_depth / 10.0),
                ],
                "BOSS Compact Digital Delay",
            )
        }
        PedalId::RedPandaParticle2 => {
            let rp = &pp.particle;
            (
                Color::LightRed, // Red Panda signature enclosure
                rp.enabled,
                if rp.enabled {
                    if rp.freeze {
                        "● ON + ❄ FREEZE "
                    } else {
                        "● ON (ACTIVE)   "
                    }
                } else {
                    "○ BYPASSED      "
                },
                vec![
                    ("GRANULAR MODE", rp.mode.label().to_string(), (rp.mode as usize as f32) / 4.0),
                    ("BLEND (MIX)", format!("{:.1}", rp.blend), rp.blend / 10.0),
                    ("CHOP / DENS", format!("{:.1}", rp.chop), rp.chop / 10.0),
                    ("DELAY / PITCH", format!("{:.1}", rp.pitch_detune), rp.pitch_detune / 10.0),
                    ("PARAM (GRAIN)", format!("{:.1}", rp.grain_size), rp.grain_size / 10.0),
                    ("FBACK (REGEN)", format!("{:.1}", rp.feedback), rp.feedback / 10.0),
                    ("FREEZE BUFFER", if rp.freeze { "FROZEN [❄]".to_string() } else { "LIVE".to_string() }, if rp.freeze { 1.0 } else { 0.0 }),
                ],
                "Granular Delay / Pitch Shifter",
            )
        }
        PedalId::StrymonCloudburst => {
            let cb = &pp.cloudburst;
            (
                Color::LightBlue, // Strymon Cornflower Blue anodized chassis
                cb.enabled,
                if cb.enabled { "● AMBIENT ON    " } else { "○ BYPASSED      " },
                vec![
                    ("ENSEMBLE SW", cb.ensemble_mode.label().to_string(), (cb.ensemble_mode as usize as f32) / 2.0),
                    ("DECAY (CLOUD)", format!("{:.1}", cb.decay), cb.decay / 10.0),
                    ("ENSEMBLE LVL", format!("{:.1}", cb.ensemble), cb.ensemble / 10.0),
                    ("MIX", format!("{:.1}", cb.mix), cb.mix / 10.0),
                    ("TONE", format!("{:.1}", cb.tone), cb.tone / 10.0),
                    ("PRE-DELAY", format!("{:.1}", cb.pre_delay), cb.pre_delay / 10.0),
                    ("MODULATION", format!("{:.1}", cb.mod_speed), cb.mod_speed / 10.0),
                ],
                "Ambient Reverb + String Ensemble",
            )
        }
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(if is_selected_pedal { Color::Yellow } else { border_color })
                .add_modifier(if is_selected_pedal { Modifier::BOLD } else { Modifier::empty() }),
        )
        .title(format!(" SLOT #{} — {} ", slot + 1, pedal_id.name()));

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Stompbox Header + LED Indicator
            Constraint::Min(8),    // Pedal Knobs / LCD Screen
            Constraint::Length(4), // Stompbox Metallic Footswitch Pad at Bottom
        ])
        .split(inner);

    // 1. Top LED & Subtitle
    let led_style = if enabled {
        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let header_para = Paragraph::new(vec![
        Line::from(Span::styled(led_str, led_style)),
        Line::from(Span::styled(extra_badge, Style::default().fg(border_color))),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(header_para, sections[0]);

    // 2. Knobs Section
    let mut knob_lines = Vec::new();
    for (p_idx, (p_name, p_val_str, norm)) in params_list.iter().enumerate() {
        let is_param_sel = is_selected_pedal && selected_param == p_idx;
        let filled = (norm.clamp(0.0, 1.0) * 10.0).round() as usize;
        let bar = "█".repeat(filled) + &"░".repeat(10usize.saturating_sub(filled));

        let name_style = if is_param_sel {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let val_style = if is_param_sel {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(border_color)
        };

        knob_lines.push(Line::from(vec![
            Span::styled(format!(" {:<13}: ", p_name), name_style),
            Span::styled(format!("{:<15}", p_val_str), val_style),
        ]));
        knob_lines.push(Line::from(vec![
            Span::styled(format!("   [{}] ", bar), Style::default().fg(if enabled { border_color } else { Color::DarkGray })),
        ]));
    }
    frame.render_widget(Paragraph::new(knob_lines), sections[1]);

    // 3. Bottom Stomp Footswitch Pad
    let switch_label = if enabled {
        "╔════════════════════════════╗\n║   [●] FOOTSWITCH: ENGAGED  ║\n╚════════════════════════════╝"
    } else {
        "╔════════════════════════════╗\n║   [○] FOOTSWITCH: BYPASSED ║\n╚════════════════════════════╝"
    };
    let switch_style = if enabled {
        Style::default().fg(border_color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    frame.render_widget(
        Paragraph::new(switch_label)
            .style(switch_style)
            .alignment(Alignment::Center),
        sections[2],
    );
}
