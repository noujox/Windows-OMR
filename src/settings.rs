//! Persisted desktop preferences, independent of Omarchy.
//! The "default" device follows the operating system's selection.

use std::path::PathBuf;

use gtk::glib;

use crate::APP_NAME;
use crate::export::Format;
use crate::transcribe::LANGUAGES;

fn path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let state = glib::user_config_dir();
    #[cfg(not(target_os = "windows"))]
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| glib::home_dir().join(".local/state"));
    state.join(APP_NAME).join("settings.json")
}

fn load() -> serde_json::Value {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// Updates one key and keeps the others.
fn save(key: &str, value: &str) {
    let mut settings = load();
    settings[key] = serde_json::Value::String(value.to_owned());
    let path = path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, settings.to_string());
}

pub fn load_format() -> Format {
    load()["format"]
        .as_str()
        .map(Format::from_key)
        .unwrap_or(Format::Mono)
}

pub fn save_format(format: Format) {
    save("format", format.key());
}

/// A whisper language code from `LANGUAGES`, "auto" when unset or unknown.
pub fn load_language() -> &'static str {
    let settings = load();
    let saved = settings["language"].as_str().unwrap_or("auto");
    LANGUAGES
        .iter()
        .map(|(code, _)| *code)
        .find(|code| *code == saved)
        .unwrap_or("auto")
}

pub fn save_language(code: &str) {
    save("language", code);
}

/// The camera last recorded with, "" when there was none.
pub fn load_webcam() -> String {
    load()["webcam"].as_str().unwrap_or_default().to_owned()
}

pub fn save_webcam(name: &str) {
    save("webcam", name);
}

/// Whether the webcam toggle is on.
pub fn load_webcam_on() -> bool {
    load()["webcam_on"].as_str() == Some("yes")
}

pub fn save_webcam_on(on: bool) {
    save("webcam_on", if on { "yes" } else { "no" });
}

/// What the mic side is called in new transcripts, "You" until you change it.
pub fn load_your_name() -> String {
    load()["your_name"]
        .as_str()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(crate::meeting::DEFAULT_YOU)
        .to_owned()
}

pub fn save_your_name(name: &str) {
    save("your_name", name);
}

/// Device IDs default to the operating system choice.
pub fn microphone_device() -> String {
    load()["microphone_device"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or("default")
        .to_owned()
}

pub fn output_device() -> String {
    load()["output_device"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or("default")
        .to_owned()
}

pub fn save_microphone_device(device: &str) {
    save("microphone_device", device);
}

pub fn save_output_device(device: &str) {
    save("output_device", device);
}

/// Whisper is the only enabled STT backend in this milestone.
pub fn stt_engine() -> &'static str {
    "whisper"
}

/// Automatic GPU detection with CPU fallback is the desired runtime policy.
/// Wiring the backend selection and retry is a separate development step.
pub fn acceleration() -> &'static str {
    match load()["acceleration"].as_str() {
        Some("cpu") => "cpu",
        Some("gpu") => "gpu",
        _ => "auto",
    }
}

pub fn save_acceleration(mode: &str) {
    if matches!(mode, "auto" | "cpu" | "gpu") {
        save("acceleration", mode);
    }
}

/// Integration defaults; tray/startup OS plumbing is not implemented yet.
pub fn minimize_to_tray() -> bool {
    load()["minimize_to_tray"].as_str() != Some("no")
}

pub fn save_minimize_to_tray(enabled: bool) {
    save("minimize_to_tray", if enabled { "yes" } else { "no" });
}

pub fn start_with_windows() -> bool {
    load()["start_with_windows"].as_str() != Some("no")
}

pub fn save_start_with_windows(enabled: bool) {
    save("start_with_windows", if enabled { "yes" } else { "no" });
}
