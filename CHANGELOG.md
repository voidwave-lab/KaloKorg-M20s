# Changelog

All notable changes to **KaloKorg M20s** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.1] - 2026-10-25

### Fixed
- **35-Jack Patchbay Signal Flow & Modulation Impact (`ms20-core`)**:
  - **Auto-Engaged Modulation Attenuators**: Connecting a 3.5mm patch cable to `TOTAL IN`, `FREQ MOD IN`, `HPF CUTOFF IN`, `LPF CUTOFF IN`, `EG1 TRIG IN`, or `TRIG IN (EG1+2)` now automatically engages a musical modulation depth when the corresponding attenuator knob (`MG/T.EXT`, `EG1/EXT`, `EG2/EXT`) is at `0.0`, while also providing an effective fallback depth in `KaloKorgEngine::render_frame`. Previously, those knobs defaulted to `0.0` across factory presets, causing patched modulation signals to be multiplied by `0.0` with zero audible effect.
  - **Auxiliary Module Normalling (`S&H` & `MVCA`)**: Normalled `S&H IN` to `PINK NOISE`, `S&H CLOCK IN` to `MG PULSE`, `VCA (PATCH) IN` to `MG TRIANGLE`, and `VCA CTRL IN` to `EG1 NORMAL OUT` per the MS-20 front-panel normalled guide lines. Single-cable patches from `S&H OUT` or `VCA (PATCH) OUT` now work immediately without requiring 3 separate cables, and patching into `S&H IN/CLK` or `MVCA IN/CTRL` before routing their output automatically normals them into the `TOTAL` modulation bus.
  - **Inverted & Bipolar MVCA / Main VCA Control**: Fixed `MvcaCtrl` and `VcaInitGain` clamping (`max(0.0)`) that previously zeroed out inverted envelopes (`EG1 REV -`, `EG2 REV`) and negative LFO half-cycles. Inverted envelopes now duck VCA gain (`+5V -> 0V`), and bipolar LFOs/signals produce deep tremolo/AM gating even when a note or Drone Gate is active.
  - **Full `EG 1` (DAR) Envelope Contour & Drone Auto-Cycling**: Fixed `Eg1Dar` freezing at a static DC `1.0` (`SustainHold`) after a 13 ms attack for the entire duration of held notes or Drone sessions. `EG 1` now transitions smoothly from `Attack` (`1.0`) into `Release` decay while the note is sounding and auto-cycles (`Delay -> Attack -> Release`) in hands-free Drone mode.
  - **Keyboard Pitch CV (`KEY CV OUT`, `VCO 1+2 CV IN`, `VCO 2 CV IN`)**: Fixed `KeyCvOut` outputting `0.0` at the default middle C (`C4`, MIDI note 60) by offsetting `KeyCvOut` to a realistic unipolar positive voltage (`+0.55` at `C4`). Patched modulation into `Vco1And2CvIn` and `Vco2CvIn` now sweeps `±2.2` octaves around the played keyboard note instead of replacing keyboard tracking and jumping into ultrasonic frequencies above the LPF cutoff.
  - **Filter Cutoff Modulation Centering (`HPF CUTOFF IN` & `LPF CUTOFF IN`)**: Centered patched filter modulation into the audible harmonic passband (`80 Hz .. 5 kHz`) so positive, negative, and bipolar patch sources carve dramatic resonant sweeps rather than saturating below 22 Hz on the HPF or above 10 kHz on the LPF.
  - **Hotter `PINK NOISE`, `SIGNAL OUT` / `PHONES OUT` Overdrive & `ESP` Tracking**: Boosted `PinkNoiseOut` to full `±1.0` dynamic excursions, added asymmetric analog feedback drive when patching `SIGNAL OUT` or `PHONES OUT` into `EXT SIG IN`, and increased normalled internal signal gain into the External Signal Processor (`ESP`) so `ESP AMP OUT`, `ESP BPF OUT`, `ESP F-V CV`, `ESP ENV OUT`, and `ESP TRIG OUT` (plus `PEAK` and `TRIG` LEDs) respond actively.
- **Web Interface Patchbay & In-Browser WebAudio Engine (`ms20-tui/src/web/index.html`, `web_server.rs`)**:
  - Implemented full 20-source / 15-destination `activeCables` patchbay routing inside the optional in-browser WebAudio engine (`ensureBrowserAudio`), which previously ignored patch cables.
  - Fixed race conditions between `onJackClick` / `clearAllCables` and the 250 ms `/api/state` poller by tracking `inFlightWrites` and comparing full cable signatures (`source->dest`) rather than only array length.
- **Terminal UI (`TUI`) Patchbay Viewport (`ms20-tui/src/ui/synth_panel.rs`, `main.rs`)**:
  - Replaced the fixed 12-row truncation (`max_rows = 12`) with an auto-scrolling 12-row viewport so all 20 `PatchSource` jacks (`0..19`) and all 15 `PatchDest` jacks (`0..14`) are visible and accessible in the TUI.
  - Added `[X]` to unplug the selected destination jack and `[U]` to clear all patch cables in TUI Patchbay focus mode.

### Added
- **Control Wheel (`WHEEL ^`) & Momentary Switch (`MOMENT SW`) Controls**:
  - Added engine parameter IDs `39` (`control_wheel`, `-5.0 .. +5.0`) and `40` (`momentary_switch`, `bool`), interactive `WHEEL` slider and `MOMENT SW` button in the Web UI patchbay header, and automatic active throw defaults when patching from `ControlWheelOut` or `MomentarySwOut`.
- **Comprehensive Patchbay Sonic Verification Suite (`ms20-core/src/lib.rs`)**:
  - Added `test_all_patchbay_destinations_and_sources_measurably_alter_sound` to verify that every single one of the 15 `PatchDest` jacks and 20 `PatchSource` jacks measurably alters the synthesized output audio.

---

## [0.1.0] - 2026-10-21

### Added
- Initial release of **KaloKorg M20s**:
  - Component-modeled Korg MS-20 Mini synthesis engine (`ms20-core`) featuring Dual PolyBLEP VCOs, TC4011BP CMOS XOR Ring Modulator, Dual Series Sallen-Key VCF (`Rev.1 KORG35` & `Rev.2 LM13600 OTA`), `EG 1` (DAR), `EG 2` (HADSR), `MG` (LFO), 35-jack semi-modular patchbay, and External Signal Processor (`ESP`).
  - Reorderable 5-pedal stereo ambient guitar pedalboard (`BOSS CE-2W / DC-2W`, `Zoom MS-70CDR`, `Red Panda Particle 2`, `BOSS DD-8`, and `Strymon Cloudburst`).
  - Interactive Terminal UI (`ms20-tui`), real-time synced Web UI (`http://127.0.0.1:8080`), WebAssembly wrapper (`ms20-wasm`), and `VST3` / `CLAP` DAW plug-in (`ms20-plugin`).
