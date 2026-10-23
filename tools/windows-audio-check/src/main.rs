// Cross-compilation smoke test for the real shared audio module without GTK.
// Run cargo check --target x86_64-pc-windows-gnu from this directory.
#[path = "../../../src/audio.rs"]
mod audio;
#[cfg(target_os = "windows")]
#[path = "../../../src/player_windows.rs"]
mod player_windows;
#[cfg(target_os = "windows")]
#[path = "../../../src/ipc.rs"]
mod ipc;
#[cfg(target_os = "windows")]
mod tray_smoke;
mod settings {
    pub fn microphone_device() -> String {
        "default".into()
    }
    pub fn output_device() -> String {
        "default".into()
    }
}
fn main() {
    let _ = audio::Input::Microphone;
    let _ = audio::Input::Computer;
}
