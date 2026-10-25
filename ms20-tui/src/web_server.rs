use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};

use ms20_core::{
    engine::Preset,
    params::{PedalId, PedalboardParams, SynthParams},
    patchbay::{PatchCable, PatchDest, PatchSource},
    AudioProcessor,
};
use crate::audio_host::SharedSynthState;

const INDEX_HTML: &str = include_str!("web/index.html");

#[derive(Serialize)]
struct ApiStateResponse {
    synth: SynthParams,
    pedals: PedalboardParams,
    cables: Vec<PatchCable>,
    midi_ports: Vec<String>,
    active_midi_port: Option<usize>,
    last_midi_message: String,
    is_recording: bool,
    rec_duration_sec: f32,
    last_saved_wav: Option<String>,
    has_wav_download: bool,
    selected_preset_idx: usize,
    peak_l: f32,
    peak_r: f32,
    scope: Vec<f32>,
    esp_peak_led: bool,
    esp_trig_led: bool,
}

#[derive(Deserialize)]
struct ParamUpdateReq {
    param_id: u32,
    value: f32,
}

#[derive(Deserialize)]
struct PatchConnectReq {
    source: usize,
    dest: usize,
}

#[derive(Deserialize)]
struct PatchDisconnectReq {
    dest: usize,
}

#[derive(Deserialize)]
struct PedalMoveReq {
    slot: usize,
    direction: isize,
}

#[derive(Deserialize)]
struct PedalOrderReq {
    order: [usize; 5],
}

#[derive(Deserialize)]
struct PresetLoadReq {
    index: usize,
}

#[derive(Deserialize)]
struct MidiEventReq {
    status: u8,
    data1: u8,
    data2: u8,
}

pub fn spawn_web_server(state: Arc<Mutex<SharedSynthState>>, port: u16) -> Result<u16, String> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).or_else(|_| TcpListener::bind("127.0.0.1:0")).map_err(|e| e.to_string())?;
    let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let st = state.clone();
            thread::spawn(move || {
                let _ = handle_client(stream, st);
            });
        }
    });

    Ok(actual_port)
}

fn read_full_http_request(stream: &mut TcpStream) -> std::io::Result<(String, String, String)> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(1500)));
    let mut data = Vec::with_capacity(4096);
    let mut temp = [0u8; 4096];
    let mut header_end_idx: Option<usize> = None;
    let mut content_length: usize = 0;

    loop {
        let n = match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => n,
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(e) => return Err(e),
        };
        data.extend_from_slice(&temp[..n]);

        if header_end_idx.is_none() {
            if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                let hdr_end = pos + 4;
                header_end_idx = Some(hdr_end);
                let hdr_str = String::from_utf8_lossy(&data[..pos]);
                for line in hdr_str.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        if k.trim().eq_ignore_ascii_case("content-length") {
                            content_length = v.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            } else if let Some(pos) = data.windows(2).position(|w| w == b"\n\n") {
                let hdr_end = pos + 2;
                header_end_idx = Some(hdr_end);
                let hdr_str = String::from_utf8_lossy(&data[..pos]);
                for line in hdr_str.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        if k.trim().eq_ignore_ascii_case("content-length") {
                            content_length = v.trim().parse::<usize>().unwrap_or(0);
                        }
                    }
                }
            }
        }

        if let Some(hdr_end) = header_end_idx {
            if data.len() >= hdr_end + content_length {
                break;
            }
        }
        if data.len() > 65536 {
            break;
        }
    }

    if data.is_empty() {
        return Ok((String::new(), String::new(), String::new()));
    }

    let hdr_end = header_end_idx.unwrap_or(data.len());
    let header_str = String::from_utf8_lossy(&data[..hdr_end]);
    let body_str = if hdr_end < data.len() {
        String::from_utf8_lossy(&data[hdr_end..]).trim_matches('\0').to_string()
    } else {
        String::new()
    };

    let first_line = header_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    Ok((method, path, body_str))
}

fn handle_client(mut stream: TcpStream, state: Arc<Mutex<SharedSynthState>>) -> std::io::Result<()> {
    let (method, path, body_owned) = read_full_http_request(&mut stream)?;
    if method.is_empty() {
        return Ok(());
    }
    let method = method.as_str();
    let (route, query) = path.split_once('?').unwrap_or((path.as_str(), ""));
    let body = body_owned.as_str();

    match (method, route) {
        ("GET", "/") | ("GET", "/index.html") => {
            send_response(&mut stream, "200 OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes())
        }
        ("GET", "/api/state") => {
            let json = if let Ok(guard) = state.lock() {
                let resp = ApiStateResponse {
                    synth: guard.engine.params.clone(),
                    pedals: guard.engine.pedals_params.clone(),
                    cables: guard.engine.patchbay.cables.clone(),
                    midi_ports: guard.midi_ports.clone(),
                    active_midi_port: guard.active_midi_port,
                    last_midi_message: guard.last_midi_message.clone(),
                    is_recording: guard.recorder.is_recording,
                    rec_duration_sec: guard.recorder.duration_seconds(),
                    last_saved_wav: guard.recorder.last_saved_path.clone(),
                    has_wav_download: !guard.recorder.last_wav_bytes.is_empty(),
                    selected_preset_idx: guard.selected_preset_idx,
                    peak_l: guard.engine.peak_l,
                    peak_r: guard.engine.peak_r,
                    scope: guard.engine.scope_buffer[..64].to_vec(),
                    esp_peak_led: guard.engine.esp.peak_led,
                    esp_trig_led: guard.engine.esp.trig_led,
                };
                serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string())
            } else {
                "{}".to_string()
            };
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes())
        }
        ("POST", "/api/param") | ("GET", "/api/param") => {
            let mut parsed: Option<(u32, f32)> = None;
            if let Ok(req) = serde_json::from_str::<ParamUpdateReq>(body) {
                parsed = Some((req.param_id, req.value));
            } else if !query.is_empty() {
                let mut pid = None;
                let mut val = None;
                for kv in query.split('&') {
                    if let Some((k, v)) = kv.split_once('=') {
                        if k == "param_id" {
                            pid = v.parse::<u32>().ok();
                        } else if k == "value" {
                            val = v.parse::<f32>().ok();
                        }
                    }
                }
                if let (Some(p), Some(v)) = (pid, val) {
                    parsed = Some((p, v));
                }
            }
            if let Some((param_id, value)) = parsed {
                if let Ok(mut guard) = state.lock() {
                    guard.engine.set_parameter(param_id, value);
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/patch/connect") => {
            if let Ok(req) = serde_json::from_str::<PatchConnectReq>(body) {
                if let Ok(mut guard) = state.lock() {
                    guard.engine.connect_patch_cable(
                        PatchSource::from_index(req.source),
                        PatchDest::from_index(req.dest),
                    );
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/patch/disconnect") => {
            if let Ok(req) = serde_json::from_str::<PatchDisconnectReq>(body) {
                if let Ok(mut guard) = state.lock() {
                    guard
                        .engine
                        .disconnect_patch_cable(PatchDest::from_index(req.dest));
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/patch/clear") => {
            if let Ok(mut guard) = state.lock() {
                guard.engine.clear_patch_cables();
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/pedal/move") => {
            let mut parsed: Option<(usize, isize)> = None;
            if let Ok(req) = serde_json::from_str::<PedalMoveReq>(body) {
                parsed = Some((req.slot, req.direction));
            } else if !query.is_empty() {
                let mut slot = None;
                let mut dir = None;
                for kv in query.split('&') {
                    if let Some((k, v)) = kv.split_once('=') {
                        if k == "slot" {
                            slot = v.parse::<usize>().ok();
                        } else if k == "direction" {
                            dir = v.parse::<isize>().ok();
                        }
                    }
                }
                if let (Some(s), Some(d)) = (slot, dir) {
                    parsed = Some((s, d));
                }
            }
            if let Some((slot, direction)) = parsed {
                if let Ok(mut guard) = state.lock() {
                    guard.engine.move_pedal(slot, direction);
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/pedal/order") => {
            if let Ok(req) = serde_json::from_str::<PedalOrderReq>(body) {
                if let Ok(mut guard) = state.lock() {
                    let map_id = |n: usize| match n % 5 {
                        0 => PedalId::ZoomMs70Cdr,
                        1 => PedalId::BossDd8,
                        2 => PedalId::RedPandaParticle2,
                        3 => PedalId::StrymonCloudburst,
                        _ => PedalId::BossCe2wChorus,
                    };
                    guard.engine.pedals_params.chain_order = [
                        map_id(req.order[0]),
                        map_id(req.order[1]),
                        map_id(req.order[2]),
                        map_id(req.order[3]),
                        map_id(req.order[4]),
                    ];
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/preset") => {
            if let Ok(req) = serde_json::from_str::<PresetLoadReq>(body) {
                let presets = Preset::factory_presets();
                if let Some(p) = presets.get(req.index) {
                    if let Ok(mut guard) = state.lock() {
                        guard.selected_preset_idx = req.index;
                        guard.engine.load_preset(p);
                    }
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/midi") => {
            if let Ok(req) = serde_json::from_str::<MidiEventReq>(body) {
                if let Ok(mut guard) = state.lock() {
                    guard
                        .engine
                        .handle_midi_event([req.status, req.data1, req.data2]);
                    guard.last_midi_message = format!("WebMIDI {:02X} {} {}", req.status, req.data1, req.data2);
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"ok\":true}")
        }
        ("POST", "/api/record/toggle") => {
            let mut msg = String::new();
            if let Ok(mut guard) = state.lock() {
                if guard.recorder.is_recording {
                    match guard.recorder.stop_and_save() {
                        Ok(path) => msg = path,
                        Err(e) => msg = e,
                    }
                } else {
                    guard.recorder.start();
                    msg = "Recording started".to_string();
                }
            }
            let body = format!("{{\"ok\":true,\"message\":{:?}}}", msg);
            send_response(&mut stream, "200 OK", "application/json", body.as_bytes())
        }
        ("GET", "/api/recording/latest.wav") => {
            let wav = if let Ok(guard) = state.lock() {
                guard.recorder.last_wav_bytes.clone()
            } else {
                Vec::new()
            };
            if wav.is_empty() {
                send_response(&mut stream, "404 Not Found", "text/plain", b"No recording yet")
            } else {
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Disposition: attachment; filename=\"kalokorg_m20s_session.wav\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    wav.len()
                );
                stream.write_all(header.as_bytes())?;
                stream.write_all(&wav)?;
                Ok(())
            }
        }
        _ => send_response(&mut stream, "404 Not Found", "text/plain", b"Not Found"),
    }
}

fn send_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}
