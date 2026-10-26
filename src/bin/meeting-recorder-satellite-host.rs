//! Browser Native Messaging -> current-user IPC. Intentionally no GTK,
//! model, microphone, file access, or app autostart dependencies.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod host {
    use std::io::{self, Read, Write};
    use interprocess::local_socket::{GenericNamespaced, Stream, prelude::*};

    const PIPE: &str = "meeting-recorder.user-control.v1";
    const MAX_MESSAGE: usize = 4096;

    fn validate(key: &str) -> bool {
        let Some((provider, token)) = key.split_once('.') else { return false };
        matches!(provider, "meet" | "teams" | "zoom")
            && (8..=64).contains(&token.len())
            && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    }

    /// One write preserves ordering: mic state always precedes join/beat.
    /// Unknown (including old extensions) is fail-closed when Smart Mic Sync
    /// is explicitly enabled; never claim that native messaging confirms
    /// recording actually started.
    fn commands(cmd: &str, key: &str, mic: &str) -> Option<String> {
        if !matches!(cmd, "join" | "leave" | "beat") || !validate(key) { return None; }
        if !matches!(mic, "muted" | "live" | "unknown") { return None; }
        if cmd == "leave" {
            return Some(format!("satellite-leave {key}\n"));
        }
        Some(format!("satellite-mic-{mic} {key}\nsatellite-{cmd} {key}\n"))
    }

    fn send(cmd: &str, key: &str, mic: &str) -> bool {
        let Some(packet) = commands(cmd, key, mic) else { return false };
        let Ok(name) = PIPE.to_ns_name::<GenericNamespaced>() else { return false };
        let Ok(mut stream) = Stream::connect(name) else { return false };
        stream.write_all(packet.as_bytes()).is_ok()
    }

    pub fn run() -> io::Result<()> {
        // Rust stdin/stdout on Windows use inherited file handles (binary
        // ReadFile/WriteFile); never print diagnostic text to stdout.
        let mut input = io::stdin().lock();
        let mut output = io::stdout().lock();
        let mut size = [0u8; 4];
        while input.read_exact(&mut size).is_ok() {
            let count = u32::from_le_bytes(size) as usize;
            if count == 0 || count > MAX_MESSAGE {
                break;
            }
            let mut buf = vec![0u8; count];
            input.read_exact(&mut buf)?;
            let value = serde_json::from_slice::<serde_json::Value>(&buf).ok();
            let action = value.as_ref().and_then(|v| v["event"].as_str()).unwrap_or("");
            let key = value.as_ref().and_then(|v| v["session"].as_str()).unwrap_or("");
            let mic = value.as_ref().and_then(|v| v["mic"].as_str()).unwrap_or("unknown");
            let accepted = commands(action, key, mic).is_some();
            let connected = accepted && send(action, key, mic);
            let response = serde_json::json!({"ok": connected, "connected": connected, "valid": accepted});
            let response = response.to_string();
            output.write_all(&(response.len() as u32).to_le_bytes())?;
            output.write_all(response.as_bytes())?;
            output.flush()?;
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::{commands, validate};
        #[test]
        fn refuses_invalid_session() {
            assert!(validate("meet.01234567"));
            assert!(!validate("meet.123\nstop"));
            assert!(!validate("meet.short"));
            assert!(!validate("example.01234567"));
        }
        #[test]
        fn mute_precedes_start_in_one_pipe_write() {
            let key = "meet.01234567";
            assert_eq!(commands("join", key, "muted").unwrap(),
                "satellite-mic-muted meet.01234567\nsatellite-join meet.01234567\n");
            assert_eq!(commands("beat", key, "live").unwrap(),
                "satellite-mic-live meet.01234567\nsatellite-beat meet.01234567\n");
            assert_eq!(commands("join", key, "unknown").unwrap(),
                "satellite-mic-unknown meet.01234567\nsatellite-join meet.01234567\n");
            assert_eq!(commands("leave", key, "unknown").unwrap(),
                "satellite-leave meet.01234567\n");
            assert!(commands("join", key, "maybe\nstop").is_none());
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    if let Err(err) = host::run() {
        eprintln!("Meeting Recorder Satellite native messaging failed: {err}");
    }
    #[cfg(not(target_os = "windows"))]
    eprintln!("Meeting Recorder Satellite host supports Windows only.");
}
