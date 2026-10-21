use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::sync::{Arc, Mutex};

use ms20_core::AudioProcessor;
use crate::audio_host::SharedSynthState;

#[cfg(target_os = "macos")]
mod macos_midi {
    use super::*;

    type OSStatus = i32;
    type ItemCount = usize;
    type MIDIClientRef = u32;
    type MIDIPortRef = u32;
    type MIDIEndpointRef = u32;
    type MIDIObjectRef = u32;
    type CFStringRef = *const c_void;
    type CFAllocatorRef = *const c_void;

    #[repr(C, packed(4))]
    struct MIDIPacket {
        time_stamp: u64,
        length: u16,
        data: [u8; 256],
    }

    #[repr(C)]
    struct MIDIPacketList {
        num_packets: u32,
        packet: [MIDIPacket; 1],
    }

    type MIDIReadProc = extern "C" fn(
        pktlist: *const MIDIPacketList,
        read_proc_ref_con: *mut c_void,
        src_conn_ref_con: *mut c_void,
    );

    #[link(name = "CoreMIDI", kind = "framework")]
    extern "C" {
        static kMIDIPropertyDisplayName: CFStringRef;
        static kMIDIPropertyName: CFStringRef;

        fn MIDIGetNumberOfSources() -> ItemCount;
        fn MIDIGetSource(source_index0: ItemCount) -> MIDIEndpointRef;
        fn MIDIObjectGetStringProperty(
            obj: MIDIObjectRef,
            property_id: CFStringRef,
            str_out: *mut CFStringRef,
        ) -> OSStatus;

        fn MIDIClientCreate(
            name: CFStringRef,
            notify_proc: *const c_void,
            notify_ref_con: *mut c_void,
            out_client: *mut MIDIClientRef,
        ) -> OSStatus;

        fn MIDIInputPortCreate(
            client: MIDIClientRef,
            port_name: CFStringRef,
            read_proc: MIDIReadProc,
            ref_con: *mut c_void,
            out_port: *mut MIDIPortRef,
        ) -> OSStatus;

        fn MIDIPortConnectSource(
            port: MIDIPortRef,
            source: MIDIEndpointRef,
            conn_ref_con: *mut c_void,
        ) -> OSStatus;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(
            alloc: CFAllocatorRef,
            c_str: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        fn CFStringGetCString(
            the_string: CFStringRef,
            buffer: *mut c_char,
            buffer_size: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(cf: *const c_void);
    }

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    fn cfstring_to_rust(cf_str: CFStringRef) -> Option<String> {
        if cf_str.is_null() {
            return None;
        }
        let mut buf = [0 as c_char; 256];
        let ok = unsafe {
            CFStringGetCString(cf_str, buf.as_mut_ptr(), 256, K_CF_STRING_ENCODING_UTF8)
        };
        if ok != 0 {
            let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
            Some(cstr.to_string_lossy().into_owned())
        } else {
            None
        }
    }

    fn endpoint_name(ep: MIDIEndpointRef) -> String {
        unsafe {
            let mut cf_name: CFStringRef = ptr::null();
            if MIDIObjectGetStringProperty(ep, kMIDIPropertyDisplayName, &mut cf_name) == 0
                && !cf_name.is_null()
            {
                let res = cfstring_to_rust(cf_name);
                CFRelease(cf_name);
                if let Some(s) = res {
                    return s;
                }
            }
            if MIDIObjectGetStringProperty(ep, kMIDIPropertyName, &mut cf_name) == 0
                && !cf_name.is_null()
            {
                let res = cfstring_to_rust(cf_name);
                CFRelease(cf_name);
                if let Some(s) = res {
                    return s;
                }
            }
        }
        format!("MIDI Device #{}", ep)
    }

    extern "C" fn midi_read_proc(
        pktlist: *const MIDIPacketList,
        read_proc_ref_con: *mut c_void,
        src_conn_ref_con: *mut c_void,
    ) {
        unsafe {
            if pktlist.is_null() || read_proc_ref_con.is_null() {
                return;
            }
            let state_arc = &*(read_proc_ref_con as *const Arc<Mutex<SharedSynthState>>);
            let src_idx = src_conn_ref_con as usize;
            let num_packets = (*pktlist).num_packets;
            let mut pkt_ptr = (*pktlist).packet.as_ptr();

            for _ in 0..num_packets {
                let len = (*pkt_ptr).length as usize;
                let data_ptr = ptr::addr_of!((*pkt_ptr).data) as *const u8;
                let data = std::slice::from_raw_parts(data_ptr, len.min(256));

                if let Ok(mut guard) = state_arc.lock() {
                    // Check if user filtered to a specific MIDI port or All Ports (None)
                    let accept = match guard.active_midi_port {
                        None => true,
                        Some(sel) => sel == src_idx,
                    };
                    if accept {
                        parse_and_dispatch_midi(&mut guard, data);
                    }
                }

                // Advance to next packet in variable-length MIDIPacketList (4-byte aligned on ARM64/macOS)
                let offset = 8 + 2 + len;
                let aligned_offset = (offset + 3) & !3;
                pkt_ptr = (pkt_ptr as *const u8).add(aligned_offset) as *const MIDIPacket;
            }
        }
    }

    fn parse_and_dispatch_midi(guard: &mut SharedSynthState, bytes: &[u8]) {
        let mut i = 0;
        let mut running_status: u8 = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b >= 0xF8 {
                // Real-time clock message, skip 1 byte
                i += 1;
                continue;
            }
            let status = if (b & 0x80) != 0 {
                running_status = b;
                i += 1;
                b
            } else {
                running_status
            };

            let cmd = status & 0xF0;
            match cmd {
                0x80 | 0x90 | 0xB0 | 0xE0 => {
                    if i + 1 < bytes.len() {
                        let d1 = bytes[i] & 0x7F;
                        let d2 = bytes[i + 1] & 0x7F;
                        i += 2;
                        guard.engine.handle_midi_event([status, d1, d2]);
                        guard.last_midi_message = match cmd {
                            0x90 if d2 > 0 => format!("MIDI Note ON  #{} vel={}", d1, d2),
                            0x80 | 0x90 => format!("MIDI Note OFF #{}", d1),
                            0xB0 => format!("MIDI CC #{} val={}", d1, d2),
                            0xE0 => format!("MIDI PitchBend {}:{}", d1, d2),
                            _ => format!("MIDI {:02X} {:02X} {:02X}", status, d1, d2),
                        };
                    } else {
                        break;
                    }
                }
                0xC0 | 0xD0 => {
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    pub struct MacMidiHost {
        _client: MIDIClientRef,
        port: MIDIPortRef,
        _user_data_ptr: *mut Arc<Mutex<SharedSynthState>>,
    }

    impl MacMidiHost {
        pub fn start(state: Arc<Mutex<SharedSynthState>>) -> Option<Self> {
            unsafe {
                let c_client_name = CStr::from_bytes_with_nul_unchecked(b"KaloKorgM20sClient\0");
                let cf_client = CFStringCreateWithCString(
                    ptr::null(),
                    c_client_name.as_ptr(),
                    K_CF_STRING_ENCODING_UTF8,
                );
                let mut client: MIDIClientRef = 0;
                if MIDIClientCreate(cf_client, ptr::null(), ptr::null_mut(), &mut client) != 0 {
                    if !cf_client.is_null() {
                        CFRelease(cf_client);
                    }
                    return None;
                }
                CFRelease(cf_client);

                let c_port_name = CStr::from_bytes_with_nul_unchecked(b"KaloKorgM20sIn\0");
                let cf_port = CFStringCreateWithCString(
                    ptr::null(),
                    c_port_name.as_ptr(),
                    K_CF_STRING_ENCODING_UTF8,
                );
                let boxed = Box::new(state.clone());
                let user_data_ptr = Box::into_raw(boxed);
                let mut port: MIDIPortRef = 0;

                if MIDIInputPortCreate(
                    client,
                    cf_port,
                    midi_read_proc,
                    user_data_ptr as *mut c_void,
                    &mut port,
                ) != 0
                {
                    CFRelease(cf_port);
                    let _ = Box::from_raw(user_data_ptr);
                    return None;
                }
                CFRelease(cf_port);

                let mut host = Self {
                    _client: client,
                    port,
                    _user_data_ptr: user_data_ptr,
                };
                host.rescan_sources(&state);
                Some(host)
            }
        }

        pub fn rescan_sources(&mut self, state: &Arc<Mutex<SharedSynthState>>) {
            unsafe {
                let count = MIDIGetNumberOfSources();
                let mut names = Vec::new();
                for idx in 0..count {
                    let ep = MIDIGetSource(idx);
                    if ep != 0 {
                        let name = endpoint_name(ep);
                        let _ = MIDIPortConnectSource(self.port, ep, idx as *mut c_void);
                        names.push(name);
                    }
                }
                if let Ok(mut guard) = state.lock() {
                    guard.midi_ports = names;
                }
            }
        }
    }
}

pub struct MidiHost {
    #[cfg(target_os = "macos")]
    mac_midi: Option<macos_midi::MacMidiHost>,
}

impl MidiHost {
    pub fn start(state: Arc<Mutex<SharedSynthState>>) -> Self {
        #[cfg(target_os = "macos")]
        {
            let mac_midi = macos_midi::MacMidiHost::start(state);
            return Self { mac_midi };
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = state;
            Self {}
        }
    }

    pub fn rescan(&mut self, state: &Arc<Mutex<SharedSynthState>>) {
        #[cfg(target_os = "macos")]
        if let Some(ref mut m) = self.mac_midi {
            m.rescan_sources(state);
        }
    }
}
