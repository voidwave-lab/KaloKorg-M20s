### Component-Level IC & Circuit Specifications for Faithful MS-20 DSP Emulation

#### 1. The Core Hybrid IC: KORG35 (Custom SIP-8 Module)
The heart of the MS-20 Rev. 1 is the proprietary KORG35 thick-film hybrid package (designated IC1 and IC2 on board KLM-128). Unlike standard differential operational transconductance amplifiers (OTAs) with linear feedback, the KORG35 is an unbuffered, discrete-transistor variable-gain transconductance cell designed around a differential pair and a current mirror.

- **Internal Transistor Topology:**
  - **Input Differential Pair:** 2x NPN transistors (equivalent to 2SC945 / 2N3904) with emitters tied to a common current-sink transistor.
  - **Voltage-Controlled Current Source ($I_{\text{bias}}$):** Driven by Pin 6. Current range: $\approx 0\,\mu\text{A}$ to $500\,\mu\text{A}$, controlled exponentially via a temperature-compensated transistor pair convertor from the pitch/cutoff CV summing bus.
  - **Output Stage:** Single-ended PNP emitter follower (equivalent to 2SA733 / 2N3906) with direct unbuffered collector output before the feedback loop.
- **Physical Pinout (8-Pin SIP):**
  - Pin 1: Inverting signal input / AC sum junction
  - Pin 2: Bias compensation / DC offset adjustment
  - Pin 3: Signal ground (0V reference)
  - Pin 4: Signal audio output (low-to-medium impedance)
  - Pin 5: Resonance / internal feedback loop injection
  - Pin 6: Control current input ($I_{\text{bias}}$ setting cutoff $g$)
  - Pin 7: Positive power supply (+15V DC rail)
  - Pin 8: Negative power supply (-15V DC rail)

#### 2. Sallen-Key Filter Implementation (KLM-128 Board)
The MS-20 uses two distinct KORG35 modules wired in series as voltage-controlled 2nd-order (12 dB/oct) Sallen-Key filters.

- **Low-Pass Filter (LPF) Topology:**
  - Sallen-Key low-pass filter with two voltage-variable resistors (the transconductance elements of the KORG35) and two capacitors ($C_1 = 1\,\text{nF}$, $C_2 = 1\,\text{nF}$).
  - Cutoff frequency relation:
    $$f_c = \frac{g_m}{2 \pi \sqrt{C_1 C_2}} \propto I_{\text{bias}}$$
  - **Nonlinear Resonance Circuitry:** The "Peak" potentiometer routes output audio back into the non-inverting node. Unlike the Moog ladder (which exhibits symmetric 4-stage saturation), the MS-20 uses a diode/transistor junction clipper inside the feedback loop. When "Peak" approaches 10, the loop enters hard, asymmetric limiting, introducing prominent odd and even harmonic distortion while maintaining self-oscillation without collapsing input gain.

- **High-Pass Filter (HPF) Topology:**
  - Sallen-Key high-pass layout placed directly before the LPF.
  - Replaces reactive/resistive node positions: series capacitors ($C_a = 1\,\text{nF}$, $C_b = 1\,\text{nF}$) with shunted transconductance current paths to virtual ground.
  - Features its own independent "Peak" feedback loop identical to the LPF, allowing dual-peak resonant formant screams.

#### 3. Dual Operational Transconductance Amplifier: LM13600 / NJM13600 (Rev. 2)
In late-production MS-20 units (KLM-307 board):
- Korg replaced the discrete KORG35 modules with dual monolithic OTA ICs (**LM13600** or **NJM13600**).
- Includes linearizing input diodes and Darlington output buffers.
- DSP Implication: Rev. 2 has cleaner, more symmetric clipping and lower background noise compared to the raw, buzzy, asymmetric saturation of the discrete KORG35.

#### 4. Logic & Switching ICs (Ring Modulator & S-Trig)
- **TC4011BP (Quad 2-Input CMOS NAND Gate):**
  - **Ring Modulator:** The MS-20 does not employ an analog four-quadrant multiplier (like an AD633). Instead, it implements a digital XOR/NAND network. When Ring Mod is active on VCO 2, VCO 1's square wave and VCO 2's pulse wave enter the TC4011 logic gates. The output is a raw logical XOR switching function:
    $$V_{\text{out}}(t) = V_{\text{high}} \cdot \left[ \text{Square}_1(t) \oplus \text{Square}_2(t) \right]$$
    This creates an aggressive, clangorous metallic timbre rich in intermodulation sidebands.
- **TC4001BP (Quad 2-Input CMOS NOR Gate):**
  - Used for S-Trig (Switch Trigger) invert/latch logic and routing gating signals to EG1 and EG2.
  - Translates active-low ground-closure triggers into standard +15V internal envelope gating rails.

#### 5. Operational Amplifiers (Control Path & Modulation)
- **LM324 (Quad Single-Supply Op-Amp) / RC4558 (Dual High-Gain Op-Amp):**
  - **EG1 & EG2 Timing:** Configured as comparators and voltage followers charging/discharging RC timing banks.
  - **Frequency-to-Voltage Converter (ESP Section):** Operational amplifiers act as precision full-wave rectifiers and peak detectors to track input pitch and amplitude envelopes from guitar or microphone inputs.