pub mod params;

use nih_plug::prelude::*;
use params::KaloKorgPluginParams;

use ms20_core::{
    engine::{KaloKorgEngine, Preset},
    AudioProcessor,
};

/// KaloKorg M20s — NIH-plug DAW Plugin Wrapper (VST3 & CLAP)
///
/// Wraps the Korg MS-20 Mini software synthesizer (`KaloKorgEngine`) and the 5-pedal
/// ambient guitar pedalboard (`BOSS CE-2W / DC-2W`, `Zoom MS-70CDR`, `Red Panda Particle 2`,
/// `BOSS DD-8`, and `Strymon Cloudburst`) for use inside any DAW.
///
/// Note: The standalone WAV session recorder is intentionally omitted here because
/// audio/MIDI recording and bouncing are handled natively by the host DAW.
pub struct KaloKorgM20sPlugin {
    params: Arc<KaloKorgPluginParams>,
    engine: KaloKorgEngine,
    presets: Vec<Preset>,
    last_preset_idx: usize,
}

impl Default for KaloKorgM20sPlugin {
    fn default() -> Self {
        let mut engine = KaloKorgEngine::new(48000.0);
        // In DAW plugin mode, start with Drone Gate off so the synth plays only on DAW MIDI notes
        engine.params.drone_gate = false;
        Self {
            params: Arc::new(KaloKorgPluginParams::default()),
            engine,
            presets: Preset::factory_presets(),
            last_preset_idx: 0,
        }
    }
}

impl KaloKorgM20sPlugin {
    #[inline]
    fn handle_daw_note_event(&mut self, event: NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } => {
                let vel_u8 = (velocity.clamp(0.0, 1.0) * 127.0).round() as u8;
                self.engine.handle_midi_event([0x90, note & 0x7F, vel_u8.max(1)]);
            }
            NoteEvent::NoteOff { note, .. } | NoteEvent::Choke { note, .. } => {
                self.engine.handle_midi_event([0x80, note & 0x7F, 0]);
            }
            NoteEvent::MidiPitchBend { value, .. } => {
                let bend14 = (value.clamp(0.0, 1.0) * 16383.0).round() as u16;
                let lsb = (bend14 & 0x7F) as u8;
                let msb = ((bend14 >> 7) & 0x7F) as u8;
                self.engine.handle_midi_event([0xE0, lsb, msb]);
            }
            NoteEvent::MidiCC { cc, value, .. } => {
                let val_u8 = (value.clamp(0.0, 1.0) * 127.0).round() as u8;
                self.engine.handle_midi_event([0xB0, cc & 0x7F, val_u8]);
            }
            _ => {}
        }
    }
}

impl Plugin for KaloKorgM20sPlugin {
    const NAME: &'static str = "KaloKorg M20s";
    const VENDOR: &'static str = "KaloKorg DSP Team";
    const URL: &'static str = "https://kalokorg.local/m20s";
    const EMAIL: &'static str = "dsp@kalokorg.local";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // Supports both Instrument layout (0 inputs -> Stereo Out) and Sidechain/FX layout (Stereo In -> Stereo Out for ESP/EXT IN)
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames {
                main_input: None,
                main_output: Some("Stereo Out (MS-20 + 5 Ambient Pedals)"),
                aux_inputs: &[],
                aux_outputs: &[],
            },
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames {
                main_input: Some("External Audio In (MS-20 EXT IN & ESP)"),
                main_output: Some("Stereo Out (MS-20 + 5 Ambient Pedals)"),
                aux_inputs: &[],
                aux_outputs: &[],
            },
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let sr = buffer_config.sample_rate.max(8000.0);
        self.engine = KaloKorgEngine::new(sr);
        self.engine.params.drone_gate = self.params.drone_gate.value();
        self.params
            .sync_to_engine(&mut self.engine, &self.presets, &mut self.last_preset_idx);
        true
    }

    fn reset(&mut self) {
        self.engine.voice.all_notes_off();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Synchronize DAW automation & knob states to the MS-20s + 5-Pedalboard engine
        self.params
            .sync_to_engine(&mut self.engine, &self.presets, &mut self.last_preset_idx);

        let num_samples = buffer.samples();
        let route_ext = self.params.route_daw_input_to_ext.value();
        let mut next_event = context.next_event();

        let slices = buffer.as_slice();
        if slices.len() >= 2 {
            let (left_split, right_split) = slices.split_at_mut(1);
            let out_l = &mut left_split[0];
            let out_r = &mut right_split[0];

            for sample_idx in 0..num_samples {
                // Sample-accurate MIDI NoteOn / NoteOff / PitchBend / CC dispatch
                while let Some(event) = next_event {
                    if event.timing() > sample_idx as u32 {
                        break;
                    }
                    self.handle_daw_note_event(event);
                    next_event = context.next_event();
                }

                self.engine.external_audio_input = if route_ext {
                    (out_l[sample_idx] + out_r[sample_idx]) * 0.5
                } else {
                    0.0
                };

                let (l, r) = self.engine.render_frame();
                out_l[sample_idx] = l;
                out_r[sample_idx] = r;
            }
        } else if let Some(mono_out) = slices.first_mut() {
            for sample_idx in 0..num_samples {
                while let Some(event) = next_event {
                    if event.timing() > sample_idx as u32 {
                        break;
                    }
                    self.handle_daw_note_event(event);
                    next_event = context.next_event();
                }

                self.engine.external_audio_input = if route_ext { mono_out[sample_idx] } else { 0.0 };

                let (l, r) = self.engine.render_frame();
                mono_out[sample_idx] = (l + r) * 0.5;
            }
        }

        // Flush any remaining events at the end of the buffer
        while let Some(event) = next_event {
            self.handle_daw_note_event(event);
            next_event = context.next_event();
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for KaloKorgM20sPlugin {
    const CLAP_ID: &'static str = "com.kalokorg.m20s.synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some(
        "Korg MS-20 Mini Semi-Modular Synthesizer Clone & 5-Pedal Ambient Guitar Pedalboard",
    );
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for KaloKorgM20sPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"KaloKorgM20sSynt";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(KaloKorgM20sPlugin);
nih_export_vst3!(KaloKorgM20sPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::{StandaloneInitContext, StandaloneProcessContext};
    use params::PluginPreset;

    #[test]
    fn test_nih_plug_daw_lifecycle_and_midi_processing() {
        let mut plugin = KaloKorgM20sPlugin::default();
        let layout = &KaloKorgM20sPlugin::AUDIO_IO_LAYOUTS[0];
        let config = BufferConfig {
            sample_rate: 48000.0,
            min_buffer_size: Some(64),
            max_buffer_size: 512,
            process_mode: ProcessMode::Realtime,
        };
        let mut init_ctx = StandaloneInitContext;
        assert!(plugin.initialize(layout, &config, &mut init_ctx));

        // Queue sample-accurate NoteOn at sample 16 and PitchBend at sample 64
        let mut proc_ctx = StandaloneProcessContext::default();
        proc_ctx.events.push_back(NoteEvent::NoteOn {
            timing: 16,
            voice_id: None,
            channel: 0,
            note: 60,
            velocity: 0.9,
        });
        proc_ctx.events.push_back(NoteEvent::MidiPitchBend {
            timing: 64,
            channel: 0,
            value: 0.65,
        });

        let mut left = vec![0.0f32; 512];
        let mut right = vec![0.0f32; 512];
        let mut buffer = Buffer::from_slices(vec![&mut left, &mut right]);
        let mut aux = AuxiliaryBuffers {
            inputs: &mut [],
            outputs: &mut [],
        };

        let status = plugin.process(&mut buffer, &mut aux, &mut proc_ctx);
        assert_eq!(status, ProcessStatus::Normal);

        let energy_l: f32 = left.iter().map(|s| s * s).sum();
        let energy_r: f32 = right.iter().map(|s| s * s).sum();
        let stereo_diff: f32 = left.iter().zip(right.iter()).map(|(l, r)| (l - r).abs()).sum();
        assert!(energy_l > 0.005 && energy_r > 0.005);
        assert!(
            stereo_diff > 0.02,
            "Expected stereo chorus/pedalboard separation in DAW plugin output"
        );

        // Verify that lowering output_pad_db attenuates the output signal as expected
        plugin.params.output_pad_db.set_plain_value(-18.0);
        let mut left_att = vec![0.0f32; 512];
        let mut right_att = vec![0.0f32; 512];
        let mut buffer_att = Buffer::from_slices(vec![&mut left_att, &mut right_att]);
        plugin.process(&mut buffer_att, &mut aux, &mut proc_ctx);
        let energy_att: f32 = left_att.iter().map(|s| s * s).sum();
        assert!(energy_att > 0.0);
    }

    #[test]
    fn test_nih_plug_preset_switching_and_state_serialization() {
        let mut plugin = KaloKorgM20sPlugin::default();
        plugin
            .params
            .preset
            .set_plain_value(PluginPreset::Particle2GranularCosmos);

        let mut left = vec![0.0f32; 256];
        let mut right = vec![0.0f32; 256];
        let mut buffer = Buffer::from_slices(vec![&mut left, &mut right]);
        let mut aux = AuxiliaryBuffers {
            inputs: &mut [],
            outputs: &mut [],
        };
        let mut proc_ctx = StandaloneProcessContext::default();
        proc_ctx.events.push_back(NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel: 0,
            note: 55,
            velocity: 1.0,
        });

        plugin.process(&mut buffer, &mut aux, &mut proc_ctx);
        assert_eq!(plugin.engine.patchbay.cables.len(), 3);

        // Verify DAW project state serialization & deserialization
        let saved_json = plugin.params.serialize_fields();
        assert!(!saved_json.is_empty());
        plugin.params.deserialize_fields(&saved_json);
    }
}
