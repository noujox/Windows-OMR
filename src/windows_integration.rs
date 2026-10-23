//! Windows shell integration: tray icon, user-level login startup, and
//! explicit, non-recording start hidden in the notification area.
use std::cell::RefCell;
use std::io;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gio, glib};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

use crate::{ipc, settings};

const LOGIN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const LOGIN_VALUE: &str = "MeetingRecorder";

thread_local! {
    // Holds the application open while its windows are hidden. The tray icon
    // must live as long as the app; dropping it removes the Windows icon.
    static TRAY: RefCell<Option<(TrayIcon, gio::ApplicationHoldGuard)>> =
        const { RefCell::new(None) };
}

pub fn active() -> bool {
    TRAY.with(|tray| tray.borrow().is_some())
}

/// Register in HKCU (no elevation or system-wide changes). The --start-hidden
/// flag deliberately never starts a recording.
pub fn sync_startup() -> io::Result<()> {
    let current = std::env::current_exe()?;
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(LOGIN_KEY)?;
    if settings::start_with_windows() {
        let startup = format!("\"{}\" --start-hidden", current.display());
        key.set_value(LOGIN_VALUE, &startup)?;
    } else if let Err(err) = key.delete_value(LOGIN_VALUE) {
        if err.kind() != io::ErrorKind::NotFound {
            return Err(err);
        }
    }
    Ok(())
}

/// Small self-contained fallback until the application has branded assets.
/// 32x32 opaque blue tile with a clear white microphone glyph.
fn tray_icon() -> Result<Icon, String> {
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let offset = (y * 32 + x) * 4;
            let dx = x as i32 - 15;
            let dy = y as i32 - 15;
            if dx * dx + dy * dy <= 225 {
                pixels[offset..offset + 4].copy_from_slice(&[38, 112, 226, 255]);
            }
            let capsule = (11..=19).contains(&x) && (7..=19).contains(&y);
            let stem = (14..=16).contains(&x) && (20..=26).contains(&y);
            let base = (11..=19).contains(&x) && (25..=27).contains(&y);
            if capsule || stem || base {
                pixels[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    Icon::from_rgba(pixels, 32, 32).map_err(|e| e.to_string())
}

/// Called once from GTK's startup signal on the GUI thread.
/// Returns false on tray creation failure; autostart then opens the normal UI.
pub fn install(app: &adw::Application) -> bool {
    if !settings::minimize_to_tray() {
        return false;
    }
    if active() {
        return true;
    }
    let menu = Menu::new();
    let open = MenuItem::with_id("open", "Open Meeting Recorder", true, None);
    let start = MenuItem::with_id("start", "Start recording", true, None);
    let pause = MenuItem::with_id("pause", "Pause / Resume", true, None);
    let stop = MenuItem::with_id("stop", "Stop recording", true, None);
    let exit = MenuItem::with_id("exit", "Exit", true, None);
    for item in [&open, &start, &pause, &stop] {
        if menu.append(item).is_err() {
            return false;
        }
    }
    let _ = menu.append(&PredefinedMenuItem::separator());
    if menu.append(&exit).is_err() {
        return false;
    }
    let Ok(icon) = tray_icon() else { return false };
    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Meeting Recorder")
        .with_icon(icon)
        .build()
    {
        Ok(tray) => tray,
        Err(err) => {
            eprintln!("Meeting Recorder: notification tray failed: {err}");
            return false;
        }
    };
    let keep_alive = app.hold();
    TRAY.with(|cell| *cell.borrow_mut() = Some((tray, keep_alive)));

    let app = app.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id().0.as_str() {
                "open" => app.activate(),
                "start" => { ipc::send("start"); },
                "pause" => { ipc::send("pause"); },
                "stop" => { ipc::send("stop"); },
                "exit" => {
                    // Use the existing close handler: it asks before ending a
                    // recording or transcription and preserves recovery logic.
                    app.activate();
                    app.activate_action("quit", None);
                    let app = app.clone();
                    let mut attempts = 0usize;
                    glib::timeout_add_local(Duration::from_millis(100), move || {
                        attempts += 1;
                        if app.windows().is_empty() {
                            app.quit();
                            return glib::ControlFlow::Break;
                        }
                        if attempts > 50 {
                            // User declined a confirmation: keep the app alive.
                            return glib::ControlFlow::Break;
                        }
                        glib::ControlFlow::Continue
                    });
                }
                _ => {}
            }
        }
        glib::ControlFlow::Continue
    });
    true
}
