//! Small persistent diagnostic log, independent of GTK's event loop.
//! Contains state transitions and errors, never the actual transcript content.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::APP_NAME;

static LOG_FILE: OnceLock<Mutex<PathBuf>> = OnceLock::new();
const LIMIT: u64 = 5 * 1024 * 1024;

pub fn path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(gtk::glib::user_cache_dir);
    #[cfg(not(target_os = "windows"))]
    let base = gtk::glib::user_cache_dir();
    base.join(APP_NAME).join("logs").join("app.log")
}

pub fn init() {
    let file = path();
    if let Some(parent) = file.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Meeting Recorder: cannot create log folder: {e}");
            return;
        }
    }
    if LOG_FILE.set(Mutex::new(file)).is_ok() {
        event(
            "startup",
            &format!(
                "version={} pid={} OS={} args={}",
                env!("CARGO_PKG_VERSION"),
                std::process::id(),
                std::env::consts::OS,
                std::env::args().skip(1).count()
            ),
        );
        let old = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            event("panic", &format!("{info}"));
            old(info);
        }));
    }
}

pub fn event(section: &str, message: &str) {
    let Some(lock) = LOG_FILE.get() else { return };
    let Ok(file) = lock.lock() else { return };
    let file = &*file;
    if let Ok(meta) = fs::metadata(file)
        && meta.len() >= LIMIT
    {
        let backup = file.with_extension("previous.log");
        let _ = fs::remove_file(&backup);
        let _ = fs::rename(file, backup);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    if let Ok(mut stream) = OpenOptions::new().create(true).append(true).open(file) {
        let safe = message.replace(['\n', '\r'], " ");
        let _ = writeln!(
            stream,
            "{now} pid={} [{section}] {safe}",
            std::process::id()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_path_has_expected_file_name() {
        assert_eq!(path().file_name().unwrap(), "app.log");
    }
}
