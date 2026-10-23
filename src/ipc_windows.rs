//! Windows local IPC over user-restricted named pipes.
//! The server stays on the loopback machine namespace; no network ports.
//! The wire protocol is the Linux implementation's newline commands/JSON.
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use interprocess::TryClone;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*};
use interprocess::os::windows::local_socket::ListenerOptionsExt;
use interprocess::os::windows::security_descriptor::SecurityDescriptor;
use widestring::U16CString;

use super::{Command, MAX_LINE, Statuses, busiest, finished, now, parse_command, round};
use crate::audio::{Source, to_meter};

// Use a stable machine-local name. The pipe's DACL, not the name,
// provides access control and prevents cross-account reads/control.
const PIPE: &str = "meeting-recorder.user-control.v1";
const OWNER_ONLY_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;OW)";

fn name() -> io::Result<interprocess::local_socket::Name<'static>> {
    PIPE.to_ns_name::<GenericNamespaced>()
}

fn connect() -> io::Result<Stream> {
    Stream::connect(name()?)
}

fn listen() -> io::Result<interprocess::local_socket::Listener> {
    // Do not use the default Windows named-pipe DACL: it could give
    // other local users access to transcript paths or recording commands.
    let sddl = U16CString::from_str(OWNER_ONLY_SDDL)
        .map_err(|err| io::Error::new(ErrorKind::InvalidInput, err))?;
    let descriptor = SecurityDescriptor::deserialize(&sddl)?;
    ListenerOptions::new()
        .name(name()?)
        .security_descriptor(descriptor)
        .create_sync()
}

pub fn serve(
    statuses: Statuses,
    mic: Source,
    computer: Source,
    commands: async_channel::Sender<Command>,
) {
    let listener = match listen() {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("Meeting Recorder: unable to create private Windows control pipe: {err}");
            return;
        }
    };
    let clients: Arc<Mutex<Vec<Stream>>> = Arc::default();
    let connected = clients.clone();
    thread::spawn(move || {
        for result in listener.incoming() {
            let stream = match result {
                Ok(stream) => stream,
                Err(err) => {
                    eprintln!("Meeting Recorder: accepting control pipe: {err}");
                    continue;
                }
            };
            // The send-side socket must never stall the broadcaster.
            if stream
                .set_send_timeout(Some(Duration::from_millis(20)))
                .is_err()
            {
                continue;
            }
            if let Ok(reader) = stream.try_clone() {
                let commands = commands.clone();
                thread::spawn(move || read_commands(reader, &commands));
            }
            connected.lock().unwrap().push(stream);
        }
    });

    thread::spawn(move || {
        loop {
            let snapshot = busiest(&statuses);
            let recording = snapshot.state == "recording";
            let taking = recording || snapshot.state == "paused";
            let until = if snapshot.pause_began > 0 {
                snapshot.pause_began
            } else {
                now()
            };
            let busy = recording || snapshot.state == "transcribing";
            let line = serde_json::json!({
                "state": snapshot.state,
                "elapsed": if taking { (until - snapshot.started_at - snapshot.paused_secs).max(0) } else { 0 },
                "title": snapshot.title,
                "mic": round(to_meter(mic.recent_peak(3))),
                "computer": round(to_meter(computer.recent_peak(3))),
                "progress": round(snapshot.progress),
                "folder": snapshot.folder,
            }).to_string() + "\n";
            clients
                .lock()
                .unwrap()
                .retain_mut(|client| match client.write_all(line.as_bytes()) {
                    Ok(()) => true,
                    Err(err) => err.kind() == ErrorKind::Interrupted,
                });
            thread::sleep(Duration::from_millis(if recording {
                50
            } else if busy {
                250
            } else {
                1000
            }));
        }
    });
}

fn read_commands(mut stream: Stream, commands: &async_channel::Sender<Command>) {
    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.by_ref().take(MAX_LINE as u64).read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                if let Some(command) = parse_command(&line) {
                    let _ = commands.send_blocking(command);
                }
            }
        }
    }
}

pub fn send(command: &str) -> bool {
    let Ok(mut stream) = connect() else {
        return false;
    };
    stream.write_all(format!("{command}\n").as_bytes()).is_ok()
}

pub fn wait(audio_only: bool) -> bool {
    let mut busy = false;
    loop {
        let Ok(stream) = connect() else {
            if busy {
                return false;
            }
            thread::sleep(Duration::from_millis(500));
            continue;
        };
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.by_ref().take(MAX_LINE as u64).read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let Ok(status) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            let state = status["state"].as_str().unwrap_or_default();
            let folder = status["folder"].as_str().unwrap_or_default();
            if let Some(done) = finished(state, folder, busy, audio_only) {
                println!("{done}");
                return true;
            }
            busy |= matches!(state, "recording" | "paused" | "stopping" | "transcribing");
        }
    }
}

pub fn watch() {
    let mut stdout = io::stdout();
    loop {
        if let Ok(stream) = connect() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.by_ref().take(MAX_LINE as u64).read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) if !line.ends_with('\n') => break,
                    Ok(_) => {
                        if stdout
                            .write_all(line.as_bytes())
                            .and_then(|_| stdout.flush())
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
        }
        if writeln!(stdout, r#"{{"state":"off"}}"#)
            .and_then(|_| stdout.flush())
            .is_err()
        {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
}
