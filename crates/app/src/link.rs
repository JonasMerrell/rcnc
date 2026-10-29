//! Local link for CAD/CAM apps: a small HTTP endpoint on 127.0.0.1 that
//! loads a program (and its tools) sent from the CAM, so posting is one
//! click instead of save, switch, open.
//!
//! - `GET /status` returns `{"app": "rcnc", "mode": .., "program": ..}`.
//! - `POST /program` takes `{"name": .., "gcode": .., "tools": [{"number", "diameter", "description"}]}`.
//!   Tools set diameters and descriptions in the tool table and keep any
//!   measured length. Refused while the machine is running.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use gcode::Tool;
use machine::{Command, ControllerHandle, Mode};
use serde::Deserialize;
use serde_json::json;

pub const PORT: u16 = 7350;
const MAX_BODY: usize = 64 << 20;

#[derive(Deserialize)]
struct Program {
    name: String,
    gcode: String,
    #[serde(default)]
    tools: Vec<SentTool>,
}

#[derive(Deserialize)]
struct SentTool {
    number: u32,
    #[serde(default)]
    diameter: f64,
    #[serde(default)]
    description: String,
}

/// Start listening in the background. Failing to bind (another rcnc is
/// running) only disables the link.
pub fn spawn(machine: ControllerHandle) {
    let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("CAM link disabled: can't listen on 127.0.0.1:{PORT}: {e}");
            return;
        }
    };
    std::thread::Builder::new()
        .name("cam-link".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                if let Err(e) = handle(stream, &machine) {
                    eprintln!("CAM link: {e}");
                }
            }
        })
        .expect("spawn CAM link thread");
}

fn handle(stream: TcpStream, machine: &ControllerHandle) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                length = v.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut parts = request.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let (status, body) = match (method, path) {
        ("GET", "/status") => {
            let s = machine.snapshot();
            (200, json!({"app": "rcnc", "mode": format!("{:?}", s.mode), "program": s.program_name}))
        }
        ("POST", "/program") if length > MAX_BODY => (413, json!({"ok": false, "message": "program too large"})),
        ("POST", "/program") => {
            let mut buf = vec![0; length];
            reader.read_exact(&mut buf)?;
            match serde_json::from_slice::<Program>(&buf) {
                Ok(p) => load(machine, p),
                Err(e) => (400, json!({"ok": false, "message": format!("bad request: {e}")})),
            }
        }
        _ => (404, json!({"ok": false, "message": "unknown request"})),
    };
    respond(stream, status, &body)
}

fn load(machine: &ControllerHandle, p: Program) -> (u16, serde_json::Value) {
    let s = machine.snapshot();
    if matches!(s.mode, Mode::Running | Mode::Homing | Mode::Probing) {
        return (409, json!({"ok": false, "message": "the machine is running; stop it first"}));
    }
    for t in p.tools {
        let mut tool = s.tools.get(&t.number).cloned().unwrap_or_else(Tool::default);
        tool.diameter = t.diameter;
        if !t.description.is_empty() {
            tool.description = t.description;
        }
        machine.send(Command::SetTool(t.number, Some(tool)));
    }
    let name = p.name.clone();
    machine.send(Command::Load { name: p.name, text: p.gcode });
    (200, json!({"ok": true, "message": format!("Loaded {name} in rcnc")}))
}

fn respond(mut stream: TcpStream, status: u16, body: &serde_json::Value) -> std::io::Result<()> {
    let text = body.to_string();
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Payload Too Large",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use machine::sim::{RealClock, Simulator};
    use machine::{Controller, MachineConfig};
    use std::sync::Arc;

    fn request(port: u16, raw: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(raw.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn loads_programs_and_tools() {
        let clock = Arc::new(RealClock::new());
        let c = Controller::new(MachineConfig::dmc2(), Box::new(Simulator::new(clock.clone())), clock);
        let handle = ControllerHandle::spawn(c);
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let h = handle.clone();
        std::thread::spawn(move || {
            for s in listener.incoming().flatten() {
                handle_ok(s, &h);
            }
        });
        let body = r#"{"name":"bracket.nc","gcode":"G0 X1\nM30","tools":[{"number":2,"diameter":6,"description":"6 mm flat"}]}"#;
        let reply = request(port, &format!("POST /program HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}", body.len()));
        assert!(reply.starts_with("HTTP/1.1 200") && reply.contains("Loaded bracket.nc"), "{reply}");
        std::thread::sleep(Duration::from_millis(100));
        let s = handle.snapshot();
        assert_eq!(s.program_name.as_deref(), Some("bracket.nc"));
        assert_eq!(s.tools[&2].diameter, 6.0);
        let reply = request(port, "GET /status HTTP/1.1\r\n\r\n");
        assert!(reply.contains("\"app\":\"rcnc\""), "{reply}");
    }

    fn handle_ok(s: TcpStream, h: &ControllerHandle) {
        handle(s, h).unwrap();
    }
}
