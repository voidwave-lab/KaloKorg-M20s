# KaloKorg M20s

**KaloKorg M20s** is a component-modeled software synthesizer clone of the **Korg MS-20 Mini** paired with a reorderable **5-Pedal Ambient & Experimental Guitar Pedalboard**. Written in Rust, it provides three ways to play and produce:
1. **Interactive Terminal User Interface (TUI)** (`ratatui` + `crossterm`)
2. **Real-Time Synced Web Interface** (vintage MS-20 front panel with SVG patch cables + 5 visual guitar stompboxes)
3. **DAW Plug-in (`VST3` & `CLAP`)** powered by an `nih-plug` wrapper (`ms20-plugin`)

---

## Table of Contents
1. [Prerequisites (macOS, Linux, Windows)](#1-prerequisites-macos-linux-windows)
2. [Installation & Setup Guide](#2-installation--setup-guide)
3. [What KaloKorg M20s Implements, How It Works & Inspiration](#3-what-kalokorg-m20s-implements-how-it-works--inspiration)
4. [Basic Tutorial](#4-basic-tutorial)
5. [VST3 & CLAP DAW Plug-in (`nih-plug`)](#5-vst3--clap-daw-plug-in-nih-plug)
6. [Keyboard & Interface Controls Reference](#6-keyboard--interface-controls-reference)
7. [Basic Troubleshooting](#7-basic-troubleshooting)
8. [Changelog](#8-changelog)
9. [License](#9-license)

---

## 1. Prerequisites (macOS, Linux, Windows)

### All Platforms
- **Rust Toolchain (1.75+ / 2021 Edition)**: Install via [rustup.rs](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Terminal Emulator**: A terminal window supporting UTF-8 box drawing and truecolor/256-color (e.g., iTerm2, Ghostty, Alacritty, Kitty, WezTerm, Windows Terminal) sized to at least **130 columns × 38 rows** for the full MS-20 panel layout.
- **Web Browser (Optional, for Web UI)**: Chrome, Brave, Edge, Firefox, or Safari (Chromium-based browsers recommended if using the browser's WebMIDI API).

### macOS
- **Xcode Command Line Tools** (provides `clang` and links against macOS `AudioToolbox` / `CoreAudio` and `CoreMIDI` frameworks):
  ```bash
  xcode-select --install
  ```
- *Note*: On macOS, `ms20-tui` outputs low-latency 48 kHz stereo audio directly through native **CoreAudio (`AudioToolbox`)** and reads hardware USB MIDI controllers directly through **CoreMIDI** with zero external C library installations required.

### Linux
- **C Compiler & pkg-config**:
  - **Debian / Ubuntu**:
    ```bash
    sudo apt update && sudo apt install -y build-essential pkg-config libasound2-dev
    ```
  - **Fedora / RHEL**:
    ```bash
    sudo dnf install -y gcc pkgconf-pkg-config alsa-lib-devel
    ```
  - **Arch Linux**:
    ```bash
    sudo pacman -S base-devel pkgconf alsa-lib
    ```

### Windows
- **MSVC Build Tools**: Install [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (select the *"Desktop development with C++"* workload and Windows 10/11 SDK).
- Ensure your Rust toolchain uses the `x86_64-pc-windows-msvc` target:
  ```powershell
  rustup default stable-x86_64-pc-windows-msvc
  ```

---

## 2. Installation & Setup Guide

### Step 1: Clone or Open the Repository
```bash
git clone https://github.com/<your-username>/KaloKorg-M20s.git
cd KaloKorg-M20s
```

### Step 2: Run the Automated DSP & Plugin Test Suite
Verify that the oscillators, KORG35 filters, TC4011BP ring modulator, 35-jack patchbay, 5-pedal stereo chain, and DAW plugin wrapper pass all verification checks:
```bash
cargo test
```
*(If building in an air-gapped or offline environment using cached crates, append `--offline` to any `cargo` command.)*

### Step 3: Build and Run the Standalone Synth (TUI + Web UI)
- **Mode A — Interactive Terminal UI + Live Synced Web UI (Default)**:
  ```bash
  cargo run --release -p ms20-tui
  ```
  This launches the full Korg MS-20s TUI in your terminal **and** starts the embedded Web UI server at **`http://127.0.0.1:8080`**. Any knob turn, patch cable connection, preset change, or guitar pedal toggle in the TUI immediately updates the Web UI and vice versa.

- **Mode B — Headless / Web Interface Only**:
  ```bash
  cargo run --release -p ms20-tui -- --web --port 8080
  ```
  Then open **`http://127.0.0.1:8080`** in your browser.

---

## 3. What KaloKorg M20s Implements, How It Works & Inspiration

### Inspiration
**KaloKorg M20s** was inspired by two things:
1. **The 1978 Korg MS-20 / MS-20 Mini Hardware Architecture**: Using the original Korg service manuals, `KLM-128` / `KLM-307` circuit schematics, signal flow diagrams (`KORG-MS-20-Monophonic-Synthesizer-fig-3.jpg` through `fig-5.jpg`), and front panel layout (`front-panel.jpg`, intentionally excluding the physical piano keyboard) in [`docs and diagrams/`](./docs%20and%20diagrams/).
2. **Hardware MS-20 + Ambient Guitar Pedals Workflow**: In physical studio setups, running the raw, gritty monophonic output of a Korg MS-20 Mini through stereo chorus, granular pitch-shifting, reverse/tape delay, and ensemble cloud reverb pedals transforms aggressive analog synth patches into massive, evolving ambient and experimental soundscapes.

---

### How It Works: DSP Signal Flow

```
┌───────────────────────────────────────────────────────────────────────────────────────┐
│                          TAB 1: KORG MS-20s MONOPHONIC SYNTH                          │
│                                                                                       │
│  [VCO 1: PolyBLEP Tri/Saw/PW/Noise] ──┐                                               │
│                                       ├──> [VCO Mixer + EXT IN]                       │
│  [VCO 2: Saw/Sqr/Pulse/TC4011 XOR]  ──┘            │                                  │
│                                                    ▼                                  │
│                                     [12dB/oct KORG35 Series VC HPF]                   │
│                                                    │                                  │
│                                                    ▼                                  │
│                                     [12dB/oct KORG35 Series VC LPF]                   │
│                                                    │                                  │
│                                                    ▼                                  │
│  [MG LFO] + [EG 1 DAR] + [EG 2 HADSR] ──> [Main VCA + 35-Jack Patchbay + ESP]         │
└────────────────────────────────────────────────────┼──────────────────────────────────┘
                                                     │ (Mono Synth Signal Out)
                                                     ▼
┌───────────────────────────────────────────────────────────────────────────────────────┐
│                TAB 2: REORDERABLE 5-PEDAL AMBIENT GUITAR PEDALBOARD                   │
│                                                                                       │
│  1. [BOSS CE-2W / DC-2W]   (Mono-to-Stereo BBD Chorus & Dimension D Expander)         │
│            │                                                                          │
│            ▼                                                                          │
│  2. [ZOOM MS-70CDR]        (Stereo Tri-Chorus / +12st Ice Shimmer / SpaceHole / Tape) │
│            │                                                                          │
│            ▼                                                                          │
│  3. [RED PANDA PARTICLE 2] (4-Grain Real-Time Granular Chop, Pitch Scatter & FREEZE)  │
│            │                                                                          │
│            ▼                                                                          │
│  4. [BOSS DD-8 DELAY]      (Warm Analog / Reverse / Warp Swell / Glitch / Space Echo) │
│            │                                                                          │
│            ▼                                                                          │
│  5. [STRYMON CLOUDBURST]   (Modulated Hadamard Cloud Tank + MP/FORTE String Ensemble) │
└────────────────────────────────────────────────────┼──────────────────────────────────┘
                                                     │
                                                     ▼
                              [True Stereo Output / DAW Host / WAV Recorder]
```

### Detailed Subsystem Breakdown

#### 1. Dual PolyBLEP Oscillators & TC4011BP CMOS Ring Modulator (`ms20-core/src/dsp/oscillator.rs`)
- **VCO 1**: Triangle, Sawtooth, Variable Pulse Width (`PW` modulated by manual knob), and White Noise across `32'`, `16'`, `8'`, and `4'` octave registers.
- **VCO 2**: Sawtooth, Square, Narrow Pulse (`15%` duty cycle), and **Ring Mod** across `16'`, `8'`, `4'`, and `2'` registers with continuous `±12` semitone detune/interval pitch control.
- **PolyBLEP Anti-Aliasing**: Applies polynomial band-limited step correction around phase discontinuities (`0` and `pw`) to eliminate digital aliasing at high frequencies.
- **TC4011BP Quad 2-Input CMOS NAND Gate XOR Ring Modulator**: Faithfully models the original MS-20's logic-gate ring modulator (`RingOut = sgn(VCO1_Pulse) * sgn(VCO2_Pulse)`), producing metallic, bell-like inharmonic sidebands that change timbre as you sweep **VCO 1 Pulse Width** and **VCO 2 Pitch**.
- **Noise Generator**: 32-bit xorshift White Noise feeding a 3-stage **Paul Kellet $-3\text{ dB/octave}$ Pink Noise** filter.

#### 2. Dual Series Sallen-Key VCF (`KORG35` Rev.1 & `LM13600` OTA Rev.2) (`ms20-core/src/dsp/korg35.rs`)
- **Series Topology**: A $12\text{ dB/octave}$ Voltage-Controlled High-Pass Filter (**VC HPF**) wired directly in series into a $12\text{ dB/octave}$ Voltage-Controlled Low-Pass Filter (**VC LPF**).
- **Zero-Delay Feedback (TPT)**: Solved via Vadim Zavalishin's Topology-Preserving Transform trapezoidal integrators (`g = tan(π * fc / fs)`) so resonance (`PEAK`) remains phase-accurate all the way to Nyquist without delay-loop instability.
- **Switchable Filter IC Revisions**:
  - **Rev. 1 (`KORG35` IC on `KLM-128` board)**: Models the unbuffered NPN differential pair, `2SA733` PNP emitter follower bias offset, and dual `1SS355` silicon diodes inside the `Peak` feedback path (`k <= 2.25`). Produces the famous asymmetric odd/even harmonic distortion and screaming self-oscillation.
  - **Rev. 2 (`LM13600 / NJM13700` OTA on `KLM-307` board)**: Models the later production OTA filter with linearizing diodes and Darlington buffers for a cleaner, more symmetrical saturation curve.

#### 3. Envelopes (`EG 1`, `EG 2`) & Modulation Generator (`MG`) (`ms20-core/src/dsp/envelope.rs`, `lfo.rs`)
- **MG (LFO)**: Continuous waveform morphing from Positive Saw $\rightarrow$ Triangle $\rightarrow$ Negative Ramp ( plus simultaneous Rectangular Pulse output) spanning $0.1\text{ Hz}$ to $32\text{ Hz}$.
- **EG 1 (DAR)**: Delay–Attack–Release envelope with simultaneous Normal (`0..+5V`) and Reverse (`0..-5V`) outputs.
- **EG 2 (HADSR)**: Hold–Attack–Decay–Sustain–Release envelope controlling the Main VCA and normalled to both HPF and LPF cutoff modulation.

#### 4. 35-Jack Semi-Modular Patchbay & External Signal Processor (`ESP`) (`ms20-core/src/patchbay.rs`, `esp.rs`)
- Implements all **20 Source Jacks** and **15 Destination Jacks** from the right-hand side of the Korg MS-20 front panel.
- **Authentic Normalled Switching Jacks**: Just like the physical 3.5mm switching jacks on the MS-20, internal normalled connections (`MG Triangle -> TOTAL`, `EG1 -> FREQ`, `EG2 -> HPF/LPF CUTOFF`, `KEY CV -> VCO 1+2 CV IN`, `PINK NOISE -> S&H IN`, `MG PULSE -> S&H CLOCK`, `MG TRI -> MVCA IN`, `EG1 -> MVCA CTRL`) are active by default and automatically break when a patch cable is plugged into the corresponding input jack.
- **Smart Attenuator Engagement**: Connecting a cable into an attenuated modulation input (`TOTAL`, `FREQ`, `HPF CUTOFF`, `LPF CUTOFF`, or `EG1 TRIG`) automatically advances any `0.0` modulation depth knob (`MG/T.EXT`, `EG1/EXT`, `EG2/EXT`) to a musical default so every patch cable produces an immediate audible effect.
- Includes the auxiliary **Sample & Hold (`S&H`)** unit, **Patchbay VCA (`MVCA`)**, **Control Wheel (`WHEEL ^`)**, **Momentary Switch (`MOMENT SW`)**, and the 6-stage **External Signal Processor (`ESP`)** (`PRE-AMP` $\rightarrow$ `LOW CUT` $\rightarrow$ `HIGH CUT` $\rightarrow$ `F-V PITCH CONVERTER`, `ENVELOPE FOLLOWER`, and `TRIG SCHMITT DETECTOR`).

#### 5. The 5-Pedal Ambient Guitar Pedalboard (`ms20-core/src/dsp/pedals.rs`)
Because the Korg MS-20 is a monophonic, mono-output synthesizer, KaloKorg M20s includes a 5-slot stereo guitar pedalboard on **Tab 2**:
1. **BOSS CE-2W / DC-2W (Waza Craft Stereo Chorus & Dimension Expander)**:
   - Placed first by default to convert the mono MS-20 signal into a wide, phase-decorrelated true-stereo image using dual MN3007 bucket-brigade delay lines, Haas micro-delay stereo spread, and 4 modes: `CE-2W Stereo BBD`, `DC-2W Dimension D`, `Tri-Stereo 3-Voice`, and `Juno-60 I+II Swirl`.
2. **Zoom MS-70CDR (MultiStomp Chorus / Delay / Reverb)**:
   - Features 4 ambient algorithms: `Stereo Tri-Chorus`, `Ice Shimmer (+12st)` dual-grain octave-up pitch-shifted delay, `SpaceHole Nebula` modulated black-hole diffuser, and `Lo-Fi Tape Drift`.
3. **Red Panda Particle 2 (Granular Delay / Pitch Shifter)**:
   - 4-stream real-time overlapping granular synthesis engine with `DENS (Cloud)`, `LFO (Scrub)`, `RND (Scatter)`, `PITCH (+7/+12st)`, `REV (Reverse Grains)`, and a dedicated **`FREEZE`** footswitch that locks the live audio buffer into an infinite granular drone.
4. **BOSS DD-8 (Digital Delay)**:
   - True ping-pong stereo delay with 5 modes: `WARM (Analog)`, `REVERSE` (backwards envelope delay), `WARP (Swell)` (hyper-regenerative ambient feedback), `GLT (Glitch)` (rhythmic stutter slice repeater), and `TAPE (Space Echo)`.
5. **Strymon Cloudburst (Ambient Reverb + Harmonic String Ensemble)**:
   - 4×4 Hadamard modulated cloud tank preceded by a non-linear harmonic **String Ensemble** resonator (`OFF`, `MP` Mezzo-Piano, and `FORTE`) that synthesizes lush, bowed string-section overtones behind every synth note.

---

## 4. Basic Tutorial

### Tutorial 1: First Sound & Auditioning Presets
1. Start the synth:
   ```bash
   cargo run --release -p ms20-tui
   ```
2. **Play Notes**:
   - **Computer Keyboard**: Press `Z, X, C, B, N` or `1..8` in the TUI (or `Z..M` and `Q..I` in the Web UI at `http://127.0.0.1:8080`).
   - **Continuous Drone Mode**: Press **`D`** in the TUI (or click **`DRONE GATE: ON`** in the Web UI) to latch the gate open so you can tweak filters and pedals hands-free.
   - **USB MIDI Controller**: Plug in any class-compliant USB MIDI keyboard. Press **`M`** in the TUI to rescan/select ports.
3. **Cycle Factory Presets**: Press **`P`** in the TUI (or use the `Preset` dropdown in the Web UI) to step through the 6 factory patches:
   - `01. Classic MS-20 Lead (Fig.5)`
   - `02. KORG35 Dual-Peak Scream`
   - `03. TC4011 CMOS XOR Ring Mod`
   - `04. Cloudburst Cathedral Ensemble`
   - `05. Particle 2 Granular Cosmos`
   - `06. Sub-Bass KORG35 Growl`

### Tutorial 2: Sculpting the Signature KORG35 Dual-Peak Filter Scream
1. Latch the Drone Gate on (`D`).
2. On **Tab 1 (MS-20s Synth Panel)**, set **HPF CUTOFF** to `3.5` and raise **HPF PEAK** to `8.5`. Notice how the high-pass resonance actually *boosts* the fundamental bass frequency before cutting below it (a classic MS-20 bass trick).
3. Raise **LPF PEAK** to `8.5` and sweep **LPF CUTOFF** between `2.0` and `7.5`.
4. Press **`V`** to toggle between **`Rev.1 KORG35`** (aggressive asymmetric diode clipping) and **`Rev.2 LM13600 OTA`** (smoother, rounder resonance) and hear the difference in harmonic overtones.

### Tutorial 3: Semi-Modular Patching on the 35-Jack Patchbay
Let's patch a classic generative **Sample & Hold (S&H)** modulation routing:
- **In the Web UI**:
  1. **Single-Cable Quick Patch**: Because `S&H IN` is normalled to `PINK NOISE` and `S&H CLK` is normalled to `MG PLS |_|`, you can simply click **`S&H OUT`** (`Row 3, Col 3`) and then click **`TOTAL`** (`Row 1, Col 1`) or **`LPF FC`** (`Row 1, Col 5`) to immediately hear stepped random filter/pitch modulation!
  2. **Custom 3-Cable S&H Patch**: Or patch any custom source into **`S&H IN`** (`Row 3, Col 2`), any clock into **`S&H CLK`** (`Row 3, Col 1`), and route **`S&H OUT`** (`Row 3, Col 3`) into **`TOTAL`** (`Row 1, Col 1`).
  3. Connecting to **`TOTAL`**, **`FREQ`**, **`HPF FC`**, or **`LPF FC`** automatically engages the corresponding **`MG/T.EXT`** / **`EG1/EXT`** / **`EG2/EXT`** depth knobs if they were at `0.0`, and you can fine-tune the modulation depth with those knobs or sweep the **`WHEEL`** slider / **`MOMENT SW`** button in the patchbay header.
- **In the TUI**:
  1. Press **`J`** to switch focus from the Knobs to the **35-Jack Patchbay** (which automatically scrolls across all 20 Sources and 15 Destinations).
  2. Use `Up`/`Down` to pick a Source Jack, `Left`/`Right` to pick a Destination Jack, and `Enter` (or `Space`) to connect a patch cable (`Backspace`/`X` to unplug a destination, or `U` to clear all cables).

### Tutorial 4: Designing Your Ambient Guitar Pedal Chain
1. Press **`Tab`** (or **`F2`**) in the TUI, or click **`2. AMBIENT FX PEDALS (5 STOMPBOXES)`** in the Web UI.
2. **Mono-to-Stereo Expansion**: Observe Slot #1 (**BOSS CE-2W / DC-2W**). Switch its mode between `CE-2W Stereo BBD`, `DC-2W Dimension D`, and `Juno-60 I+II Swirl` while turning up `STEREO WIDTH`.
3. **Reorder the Signal Chain**:
   - In the TUI, select any pedal with `Left`/`Right` and press **`<`** or **`>`** to move it earlier or later in the signal chain.
   - In the Web UI, click **`◀ Left`** or **`Right ▶`** at the top of any stompbox.
   - *Tip*: Try moving **Strymon Cloudburst** *before* **Red Panda Particle 2** (`CE-2W -> Cloudburst -> Particle 2 -> DD-8 -> MS-70CDR`) so the granular chopper slices up the lush string-ensemble reverb tail!
4. **Granular Freeze**: Play a chord/interval or note and press **`F`** (or click the **`FREEZE`** footswitch on **Red Panda Particle 2**) to freeze the audio buffer into a shimmering granular cloud while you play new MS-20 lead notes over it.

### Tutorial 5: Recording a Stereo Session `.wav`
1. In the standalone TUI or Web UI, press **`R`** (or click **`● REC SESSION WAV`**).
2. Perform your synth & pedalboard session.
3. Press **`R`** again (or click **`■ STOP REC`**). Your 32-bit float stereo `.wav` file is saved in the `./recordings/` folder (and a **`⬇ Download WAV`** link appears in the Web UI toolbar).

---

## 5. VST3 & CLAP DAW Plug-in (`nih-plug`)

KaloKorg M20s includes a dedicated **`nih-plug`** wrapper crate (`ms20-plugin`) so you can load the synth and its 5-pedal ambient pedalboard directly inside Digital Audio Workstations such as **Reaper**, **Bitwig Studio**, **Ableton Live**, **Studio One**, or **FL Studio**.

> **Note on Recording in the Plug-in**: As designed for DAW workflows, the standalone `.wav` session recorder is intentionally excluded from `ms20-plugin` since MIDI sequencing, audio recording, automation, and offline/realtime bouncing are handled directly by your DAW.

### Building & Bundling the Plug-in
Run the following commands from the workspace root:
```bash
cargo build --release -p ms20-plugin
cargo run --release -p ms20-plugin --bin bundle-plugin
```
This produces the ready-to-install plugin bundles inside `target/bundled/`:
- **`target/bundled/KaloKorg M20s.clap`** (CLAP Instrument & Audio FX Plug-in)
- **`target/bundled/KaloKorg M20s.vst3`** (VST3 Instrument Plug-in)

*(If you wish to compile `ms20-plugin` against the upstream `robbert-vdh/nih-plug` git repository on an internet-connected machine, uncomment the `nih_plug = { git = "https://github.com/robbert-vdh/nih-plug.git" }` line in [`ms20-plugin/Cargo.toml`](./ms20-plugin/Cargo.toml).)*

### Installing the Plug-in into Your DAW
Copy the generated `.clap` or `.vst3` bundle into your OS plugin directory:

- **macOS**:
  ```bash
  cp -R "target/bundled/KaloKorg M20s.clap" ~/Library/Audio/Plug-Ins/CLAP/
  cp -R "target/bundled/KaloKorg M20s.vst3" ~/Library/Audio/Plug-Ins/VST3/
  ```
- **Linux**:
  ```bash
  mkdir -p ~/.clap ~/.vst3
  cp -R "target/bundled/KaloKorg M20s.clap" ~/.clap/
  cp -R "target/bundled/KaloKorg M20s.vst3" ~/.vst3/
  ```
- **Windows (PowerShell)**:
  ```powershell
  Copy-Item -Recurse "target\bundled\KaloKorg M20s.clap" "$env:COMMONPROGRAMFILES\CLAP\"
  Copy-Item -Recurse "target\bundled\KaloKorg M20s.vst3" "$env:COMMONPROGRAMFILES\VST3\"
  ```

### DAW Plug-in Capabilities
- **Sample-Accurate MIDI Input**: Responds to DAW MIDI Note On/Off, Velocity, 14-bit Pitch Bend (`±2` semitones), Mod Wheel (`CC 1`), Portamento (`CC 5`), Volume (`CC 7`), LPF Peak (`CC 71`), EG2 Release (`CC 72`), EG2 Attack (`CC 73`), LPF Cutoff (`CC 74`), HPF Cutoff (`CC 75`), and HPF Peak (`CC 76`).
- **Full Parameter Automation**: Exposes all 36 MS-20 front-panel controls, Factory Preset selection, Pedal Chain Routing Order, and all 5 Guitar Pedals (`BOSS CE-2W / DC-2W`, `Zoom MS-70CDR`, `Red Panda Particle 2`, `BOSS DD-8`, and `Strymon Cloudburst`) as automatable DAW parameters.
- **External Audio / Sidechain Input (`ESP` & `EXT IN`)**: Supports a 2-in / 2-out sidechain layout so you can route external DAW audio into the MS-20's External Signal Processor (`ESP`) and filter section.

---

## 6. Keyboard & Interface Controls Reference

| Key (TUI) | Web UI Equivalent | Action |
|---|---|---|
| `Tab` or `F1` / `F2` | Top Tab Buttons (`1` / `2`) | Switch between **Tab 1: MS-20s Synth Panel** and **Tab 2: Ambient FX Pedals (5 Stompboxes)** |
| `Left` / `Right` (`h` / `l`) | Mouse Drag on Rotary Knobs | Select previous/next MS-20 knob (Tab 1) or select Guitar Pedal Slot `#1..#5` (Tab 2) |
| `Up` / `Down` or `+` / `-` | Vertical Mouse Drag | Adjust selected parameter (`PageUp` / `PageDown` for coarse steps) |
| `J` | Click 3.5mm Jacks Directly | Toggle TUI focus between **Front-Panel Knobs** and **35-Jack Patchbay** |
| `Enter` / `Space` | Click Footswitch / `BOSS` Pad | Connect 3.5mm patch cable (in Patchbay focus) or toggle Pedal Bypass ON/OFF (Tab 2) |
| `Backspace` / `Delete` / `X` | Click Plugged Destination Jack | Unplug 3.5mm patch cable from selected Destination jack |
| `U` (in Patchbay focus) | `Clear Cables` Button | Unplug all 3.5mm patchbay cables |
| — | `WHEEL:` Slider & `MOMENT SW` Button | Sweep the MS-20 left-hand Control Wheel (`-5V..+5V` on `WHEEL (^)`) or toggle the Momentary Switch (`+5V` on `MOMENT SW`) |
| `<` / `>` (or `,` / `.`) | `◀ Left` / `Right ▶` Buttons | **Reorder the selected Guitar Pedal earlier or later in the 5-pedal chain** |
| `F` | `FREEZE` Footswitch | Toggle granular buffer **FREEZE** on Red Panda Particle 2 |
| `D` | `DRONE GATE` Button | Latch the MS-20 gate open for continuous hands-free drone synthesis |
| `P` | `Preset` Dropdown | Cycle through the 6 Factory Presets |
| `V` | `Filter IC` Dropdown | Switch between `Rev.1 KORG35 (KLM-128)` and `Rev.2 LM13600 OTA (KLM-307)` |
| `M` | `MIDI In` Dropdown | Rescan and cycle active hardware USB MIDI input controllers |
| `9` / `0` (or `(` / `)`) | `Out Pad:` Toolbar Slider | Adjust **Output Signal Pad / Trim** (`-30.0 dB` to `+6.0 dB`, default `-6.0 dB`) to tame hot KORG35 filter peaks for headphones/monitors while keeping internal patchbay voltages hot |
| `R` | `● REC SESSION WAV` Button | Start / Stop 32-bit stereo `.wav` recording to `./recordings/` |
| `Z, X, C, B, N` / `1..8` | `Z..M` / `Q..I` Keys | Trigger and audition notes from the computer keyboard |
| `Q` / `Esc` | `Ctrl+C` | Quit cleanly (auto-saves any in-progress WAV recording) |

---

## 7. Basic Troubleshooting

1. **I don't hear any sound when turning knobs**:
   - The Korg MS-20 VCA only opens when a note gate is active (or `VCA INIT GAIN` is patched). Press **`D`** in the TUI (or click **`DRONE GATE: ON`** in the Web UI) to latch a continuous note, or press keys `Z, X, C, B, N` / `1..8` (or play a connected MIDI keyboard).
   - If you are running on Linux/Windows or over SSH and listening through the browser, click the **`🔇 BROWSER AUDIO: OFF`** button in the top bar of the Web UI so it switches to **`🔈 BROWSER AUDIO: ON`** (and click anywhere on the page so the browser resumes its `AudioContext`).
   - If both Native CoreAudio and Browser WebAudio are enabled on the same Mac, keep **`BROWSER AUDIO: OFF`** to avoid hearing doubled output.

2. **The TUI layout looks cramped or truncated**:
   - Resize your terminal window to at least **130 columns × 38 rows** (or zoom out slightly with `Cmd` + `-` / `Ctrl` + `-`) so all 7 MS-20 knob columns, the 35-jack patchbay, and all 5 side-by-side guitar stompboxes have room to render.

3. **Port `8080` is already in use**:
   - Pass a custom port via `--port`:
     ```bash
     cargo run --release -p ms20-tui -- --port 8090
     ```
   - Even if port `8080` is busy, KaloKorg M20s automatically falls back to an available OS port and displays the exact `http://127.0.0.1:<port>` URL in the top status bar of the TUI.

4. **My USB MIDI controller isn't detected**:
   - On macOS TUI: Connect the controller and press **`M`** to rescan CoreMIDI sources and cycle through connected ports.
   - In the Web UI: Use a Chromium-based browser (Chrome, Edge, Brave) and allow MIDI permissions when prompted by `navigator.requestMIDIAccess()`.

5. **My DAW on macOS blocks or doesn't list the `.vst3` / `.clap` bundle**:
   - Ad-hoc sign the bundle so macOS Gatekeeper allows the DAW to load the dynamic library:
     ```bash
     codesign --force --deep --sign - ~/Library/Audio/Plug-Ins/CLAP/"KaloKorg M20s.clap"
     codesign --force --deep --sign - ~/Library/Audio/Plug-Ins/VST3/"KaloKorg M20s.vst3"
     ```
   - Then trigger a plug-in rescan inside your DAW's Preferences / Plug-in Manager.

---

## 8. Changelog

See [`CHANGELOG.md`](./CHANGELOG.md) for a detailed history of releases, DSP improvements, and bug fixes.

---

## 9. License

All source code in this project is licensed under the **GNU General Public License v3.0 (GPL-3.0-or-later)**. See [`LICENSE`](./LICENSE) for the full license text.
