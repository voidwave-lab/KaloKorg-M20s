pub mod dsp;
pub mod engine;
pub mod params;
pub mod patchbay;
pub mod voice;

/// Core Audio Processor Trait as specified in `Korg_MS20_DSP_Engineering_Specification.docx` Section 6.
pub trait AudioProcessor {
    fn process(&mut self, output_left: &mut [f32], output_right: &mut [f32]);
    fn handle_midi_event(&mut self, message: [u8; 3]);
    fn set_parameter(&mut self, param_id: u32, value: f32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{KaloKorgEngine, Preset};
    use crate::params::{FilterRevision, PedalId, Vco2Waveform};
    use crate::patchbay::{PatchDest, PatchSource};

    #[test]
    fn test_ms20_polyblep_and_korg35_synthesis() {
        let mut engine = KaloKorgEngine::new(48000.0);
        engine.handle_midi_event([0x90, 60, 120]); // Note On C4

        let mut left = [0.0f32; 2048];
        let mut right = [0.0f32; 2048];
        engine.process(&mut left, &mut right);

        let energy: f32 = left.iter().map(|x| x * x).sum();
        assert!(energy > 0.1, "Expected audible signal energy, got {}", energy);
        assert!(left.iter().all(|x| x.is_finite()));
        assert!(right.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn test_korg35_dual_peak_self_oscillation_stability() {
        let mut engine = KaloKorgEngine::new(48000.0);
        engine.params.hpf_peak = 10.0;
        engine.params.lpf_peak = 10.0;
        engine.params.filter_revision = FilterRevision::Rev1Korg35;
        engine.handle_midi_event([0x90, 48, 127]);

        for _ in 0..8000 {
            let (l, r) = engine.render_frame();
            assert!(l.is_finite() && l.abs() <= 1.01);
            assert!(r.is_finite() && r.abs() <= 1.01);
        }
    }

    #[test]
    fn test_tc4011_cmos_xor_ring_mod_and_patchbay() {
        let mut engine = KaloKorgEngine::new(48000.0);
        engine.params.vco2_waveform = Vco2Waveform::RingMod;
        engine
            .patchbay
            .connect(PatchSource::PinkNoiseOut, PatchDest::SampleHoldIn);
        engine
            .patchbay
            .connect(PatchSource::MgPulseOut, PatchDest::SampleHoldClock);
        engine
            .patchbay
            .connect(PatchSource::SampleHoldOut, PatchDest::TotalExt);
        engine.handle_midi_event([0x90, 64, 110]);

        let mut energy = 0.0;
        for _ in 0..4096 {
            let (l, r) = engine.render_frame();
            energy += l.abs() + r.abs();
        }
        assert!(energy > 10.0);
    }

    #[test]
    fn test_pedalboard_reordering_and_all_presets() {
        let mut engine = KaloKorgEngine::new(48000.0);
        engine.move_pedal(0, 1);
        assert_eq!(engine.pedals_params.chain_order[0], PedalId::ZoomMs70Cdr);
        assert_eq!(engine.pedals_params.chain_order[1], PedalId::BossCe2wChorus);

        for preset in Preset::factory_presets() {
            engine.load_preset(&preset);
            engine.handle_midi_event([0x90, 57, 115]);
            for _ in 0..1024 {
                let (l, r) = engine.render_frame();
                assert!(l.is_finite() && r.is_finite());
            }
        }
    }

    #[test]
    fn test_boss_ce2w_mono_to_stereo_chorus_modes() {
        use crate::params::ChorusMode;

        for mode_idx in 0..4 {
            let mut engine = KaloKorgEngine::new(48000.0);
            // Isolate only the BOSS CE-2W / DC-2W Stereo Chorus pedal
            engine.pedals_params.chorus.enabled = true;
            engine.pedals_params.chorus.mode = ChorusMode::from_index(mode_idx);
            engine.pedals_params.zoom.enabled = false;
            engine.pedals_params.dd8.enabled = false;
            engine.pedals_params.particle.enabled = false;
            engine.pedals_params.cloudburst.enabled = false;

            engine.handle_midi_event([0x90, 60, 120]);
            let mut stereo_diff = 0.0f32;
            for _ in 0..4096 {
                let (l, r) = engine.render_frame();
                assert!(l.is_finite() && r.is_finite());
                stereo_diff += (l - r).abs();
            }
            assert!(
                stereo_diff > 10.0,
                "Expected wide mono-to-stereo separation in ChorusMode {:?}, got {}",
                ChorusMode::from_index(mode_idx),
                stereo_diff
            );
        }
    }

    #[test]
    fn test_pedal_bounds_and_sonic_impact() {
        use crate::params::{
            BossDd8Mode, ChorusMode, CloudburstEnsembleMode, ParticleMode, ZoomMode,
        };

        let mut engine = KaloKorgEngine::new(48000.0);
        engine.handle_midi_event([0x90, 60, 120]);

        // Stress test all modes and extreme knob values across 50,000 samples (exceeding 24,000 and 48,000 wrap points)
        for sample_idx in 0..50_000 {
            if sample_idx % 5000 == 0 {
                let step = sample_idx / 5000;
                engine.pedals_params.chorus.mode = ChorusMode::from_index(step);
                engine.pedals_params.zoom.mode = ZoomMode::from_index(step);
                engine.pedals_params.dd8.mode = BossDd8Mode::from_index(step);
                engine.pedals_params.particle.mode = ParticleMode::from_index(step);
                engine.pedals_params.cloudburst.ensemble_mode =
                    CloudburstEnsembleMode::from_index(step);
                engine.pedals_params.cloudburst.pre_delay = (step as f32) % 10.0;
            }
            let (l, r) = engine.render_frame();
            assert!(l.is_finite() && r.is_finite());
        }

        // Verify wide stereo diff when pedals are active vs bypassed
        let mut stereo_diff_energy = 0.0f32;
        for _ in 0..4096 {
            let (l, r) = engine.render_frame();
            stereo_diff_energy += (l - r).abs();
        }
        assert!(
            stereo_diff_energy > 5.0,
            "Expected wide stereo effect separation from pedals, got {}",
            stereo_diff_energy
        );
    }

    #[test]
    fn test_all_patchbay_destinations_and_sources_measurably_alter_sound() {
        let disable_pedals = |eng: &mut KaloKorgEngine| {
            eng.pedals_params.chorus.enabled = false;
            eng.pedals_params.zoom.enabled = false;
            eng.pedals_params.dd8.enabled = false;
            eng.pedals_params.particle.enabled = false;
            eng.pedals_params.cloudburst.enabled = false;
        };

        // 1. Baseline unpatched render (default params, where fm_mg_text=0, fm_eg1_ext=0, hpf_eg2_ext=0)
        let mut base_eng = KaloKorgEngine::new(48000.0);
        disable_pedals(&mut base_eng);
        base_eng.handle_midi_event([0x90, 60, 115]);
        let mut base_samples = Vec::with_capacity(4096);
        for _ in 0..4096 {
            let (l, _) = base_eng.render_frame();
            base_samples.push(l);
        }

        // 2. Verify every single PatchDest produces a clear, unmistakable sonic difference
        for dest in PatchDest::ALL {
            let mut eng = KaloKorgEngine::new(48000.0);
            disable_pedals(&mut eng);
            let src = match dest {
                PatchDest::SampleHoldClock | PatchDest::Eg1TrigIn | PatchDest::MainTrigIn => {
                    PatchSource::MgPulseOut
                }
                PatchDest::SampleHoldIn | PatchDest::ExtAudioIn | PatchDest::EspSignalIn => {
                    PatchSource::PinkNoiseOut
                }
                _ => PatchSource::MgTriangleOut,
            };
            eng.connect_patch_cable(src, dest);
            eng.handle_midi_event([0x90, 60, 115]);

            let mut diff_energy = 0.0f32;
            for &b in &base_samples {
                let (l, _) = eng.render_frame();
                diff_energy += (l - b).abs();
            }
            assert!(
                diff_energy > 15.0,
                "Expected PatchDest::{:?} (with {:?}) to strongly alter sound, but diff_energy was only {}",
                dest,
                src,
                diff_energy
            );
        }

        // 3. Verify every single PatchSource (all 20 jacks!) produces a clear sonic difference
        for src in PatchSource::ALL {
            let mut eng = KaloKorgEngine::new(48000.0);
            disable_pedals(&mut eng);
            eng.params.drone_gate = true;
            // Warm up 512 samples so SignalOut/PhonesOut/ESP have live internal signal
            for _ in 0..512 {
                let _ = eng.render_frame();
            }

            let mut base_drone = KaloKorgEngine::new(48000.0);
            disable_pedals(&mut base_drone);
            base_drone.params.drone_gate = true;
            for _ in 0..512 {
                let _ = base_drone.render_frame();
            }

            eng.connect_patch_cable(src, PatchDest::TotalExt);

            let mut diff_energy = 0.0f32;
            for _ in 0..4096 {
                let (l_patched, _) = eng.render_frame();
                let (l_unpatched, _) = base_drone.render_frame();
                diff_energy += (l_patched - l_unpatched).abs();
            }
            assert!(
                diff_energy > 10.0,
                "Expected PatchSource::{:?} -> TotalExt to strongly alter sound in Drone mode, got diff_energy={}",
                src,
                diff_energy
            );
        }
    }
}
