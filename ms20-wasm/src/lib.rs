use ms20_core::{
    engine::{KaloKorgEngine, Preset},
    patchbay::{PatchDest, PatchSource},
    AudioProcessor,
};

/// WebAssembly AudioWorklet Processor Wrapper for KaloKorg M20s
pub struct WasmMs20Processor {
    engine: KaloKorgEngine,
    left_buf: [f32; 128],
    right_buf: [f32; 128],
}

impl WasmMs20Processor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            engine: KaloKorgEngine::new(sample_rate),
            left_buf: [0.0; 128],
            right_buf: [0.0; 128],
        }
    }

    pub fn process_quantum(&mut self) -> (&[f32; 128], &[f32; 128]) {
        self.engine.process(&mut self.left_buf, &mut self.right_buf);
        (&self.left_buf, &self.right_buf)
    }

    pub fn on_midi(&mut self, status: u8, data1: u8, data2: u8) {
        self.engine.handle_midi_event([status, data1, data2]);
    }

    pub fn set_param(&mut self, param_id: u32, value: f32) {
        self.engine.set_parameter(param_id, value);
    }

    pub fn connect_patch(&mut self, src: usize, dst: usize) {
        self.engine
            .connect_patch_cable(PatchSource::from_index(src), PatchDest::from_index(dst));
    }

    pub fn load_factory_preset(&mut self, idx: usize) {
        let presets = Preset::factory_presets();
        if let Some(p) = presets.get(idx) {
            self.engine.load_preset(p);
        }
    }
}

#[no_mangle]
pub extern "C" fn kalokorg_wasm_create(sample_rate: f32) -> *mut WasmMs20Processor {
    Box::into_raw(Box::new(WasmMs20Processor::new(sample_rate)))
}

#[no_mangle]
pub unsafe extern "C" fn kalokorg_wasm_set_param(
    ptr: *mut WasmMs20Processor,
    param_id: u32,
    value: f32,
) {
    if let Some(proc) = ptr.as_mut() {
        proc.set_param(param_id, value);
    }
}

#[no_mangle]
pub unsafe extern "C" fn kalokorg_wasm_midi(
    ptr: *mut WasmMs20Processor,
    status: u8,
    data1: u8,
    data2: u8,
) {
    if let Some(proc) = ptr.as_mut() {
        proc.on_midi(status, data1, data2);
    }
}
