//! Desktop development preview runner for `kalokorg-tablet` (`cargo run -p kalokorg-tablet`).

use kalokorg_tablet::{
    param_ids, run_tablet_app, ControlEvent, KaloKorgTabletApp, RealtimeTabletProcessor,
    TARGET_BUFFER_FRAMES,
};

fn main() -> eframe::Result {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  KALOKORG M20s — Tablet Synthesizer (iPadOS & Android Tablet Edition) ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  • Target Form Factor : Tablet Only (iPad UIDeviceFamily=2 / >=600dp)  ║");
    println!("║  • Orientation        : Locked Landscape (1280x800 split viewport)     ║");
    println!("║  • Top 65% Panel      : MS-20 VCOs, KORG35 VCF, EGs, Patchbay & FX     ║");
    println!("║  • Bottom 35% Keybed  : 25/37-Key Multi-Touch + Glissando + Octave -/+ ║");
    println!("║  • Audio Pipeline     : Lock-Free crossbeam(256) <=256f (5.3ms @48kHz) ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    // Verify lock-free audio callback & DSP synthesis on startup
    {
        let (app, rx) = KaloKorgTabletApp::new_for_testing();
        app.audio.send_param(param_ids::LPF_CUTOFF, 7.2);
        app.audio.send_param(param_ids::LPF_PEAK, 5.5);
        app.audio.note_on(48, 0.9); // C3 NoteOn
        let mut proc = RealtimeTabletProcessor::new(48_000.0);
        let mut buf = [0.0f32; (TARGET_BUFFER_FRAMES as usize) * 2];
        proc.process_interleaved(&rx, &mut buf, 2);
        app.audio.send(ControlEvent::NoteOff { note: 48 });
        proc.process_interleaved(&rx, &mut buf, 2);
    }

    run_tablet_app()
}
