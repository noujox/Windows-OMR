//! Persistent, per-meeting transcription schedules.
 //! Audio is exported before a job can be queued. No audio is removed here.
use std::path::{Path, PathBuf};
use serde_json::{json, Value};

pub const JOB_FILE: &str = ".windows-omr-deferred.json";

#[derive(Clone, Debug)]
pub struct Job {
    pub due: i64,
    pub status: String,
    pub error: Option<String>,
}
impl Job {
    fn from_value(v: &Value) -> Option<Self> {
        let due = v["due"].as_i64()?;
        let status = v["status"].as_str()?.to_owned();
        if due <= 0 || !matches!(status.as_str(), "pending" | "running" | "failed") {
            return None;
        }
        Some(Self {
            due,
            status,
            error: v["error"].as_str().map(str::to_owned),
        })
    }
}
pub fn load(dir: &Path) -> Option<Job> {
    let text = std::fs::read_to_string(dir.join(JOB_FILE)).ok()?;
    Job::from_value(&serde_json::from_str::<Value>(&text).ok()?)
}
pub fn save(dir: &Path, job: &Job) -> std::io::Result<()> {
    let target = dir.join(JOB_FILE);
    let temp = dir.join(".windows-omr-deferred.tmp");
    let text = serde_json::to_vec_pretty(&json!({
        "version": 1, "due": job.due, "status": job.status, "error": job.error
    })).map_err(std::io::Error::other)?;
    std::fs::write(&temp, text)?;
    // Windows rename cannot replace existing destination.
    if target.exists() {
        std::fs::remove_file(&target)?;
    }
    std::fs::rename(&temp, &target)
}
pub fn queue(dir: &Path, due: i64, now: i64) -> Result<(), String> {
    if due <= now + 10 {
        return Err("Choose a time at least 10 seconds from now".into());
    }
    if due > now + 366 * 86_400 {
        return Err("Choose a date within the next year".into());
    }
    if dir.join("transcript.md").is_file() {
        return Err("This meeting already has a transcript; use Transcribe again instead".into());
    }
    if !has_tracks(dir) {
        return Err("Separate audio tracks have not been saved yet".into());
    }
    save(dir, &Job {due, status: "pending".into(), error: None})
        .map_err(|e| format!("Could not save the schedule: {e}"))
}
pub fn has_tracks(dir: &Path) -> bool {
    let tracks = dir.join(crate::export::TRACKS_DIR);
    (tracks.join("mic.ogg").is_file() && tracks.join("computer.ogg").is_file())
        || tracks.join("source.ogg").is_file()
}
pub fn mark(dir: &Path, status: &str, error: Option<&str>) -> std::io::Result<()> {
    let Some(mut job) = load(dir) else { return Ok(()) };
    job.status = status.into();
    job.error = error.map(str::to_owned);
    save(dir, &job)
}
pub fn cancel(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(JOB_FILE));
}
pub fn root() -> PathBuf {
    gtk::glib::home_dir().join("Documents").join("Meetings")
}
pub fn due_jobs(now: i64) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root()) else { return Vec::new() };
    entries.flatten()
        .map(|entry| entry.path())
        .filter(|dir| dir.is_dir()
            && load(dir).is_some_and(|j| j.status == "pending" && j.due <= now)
            && !dir.join("transcript.md").is_file()
            && has_tracks(dir))
        .collect()
}
/// After a crash, do not silently rerun a possibly interrupted Whisper job.
/// The saved tracks stay available via the manual "Transcribe now" button.
pub fn recover_interrupted() {
    let Ok(entries) = std::fs::read_dir(root()) else {return};
    for entry in entries.flatten() {
        let dir = entry.path();
        if load(&dir).is_some_and(|job| job.status == "running") {
            let _ = mark(&dir, "failed", Some("Processing was interrupted; retry manually"));
        }
    }
}
#[cfg(target_os = "windows")]
pub fn register_wakeup(dir: &Path, due: i64) -> Result<(), String> {
    use std::{hash::{Hash, Hasher}, process::Command};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    dir.hash(&mut hash);
    let task = format!("Windows-OMR-{:016x}", hash.finish());
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let quote = |v: &str| v.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         $trigger=New-ScheduledTaskTrigger -Once -At ([DateTimeOffset]::FromUnixTimeSeconds({due}).LocalDateTime); \
         $action=New-ScheduledTaskAction -Execute '{}' -Argument '--start-hidden'; \
         $settings=New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew; \
         $user=[System.Security.Principal.WindowsIdentity]::GetCurrent().Name; \
         $principal=New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited; \
         Register-ScheduledTask -TaskName '{}' -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null",
        quote(&exe.to_string_lossy()), quote(&task)
    );
    // EncodedCommand avoids quoting problems when exe and user folders have spaces.
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64_utf16(&bytes);
    let result = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded])
        .output().map_err(|e| e.to_string())?;
    if result.status.success() { Ok(()) }
    else {
        Err(String::from_utf8_lossy(&result.stderr).trim().to_owned())
    }
}
#[cfg(target_os = "windows")]
fn base64_utf16(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(a >> 2) as usize] as char);
        out.push(TABLE[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        out.push(if chunk.len() >= 2 { TABLE[(((b & 15) << 2) | (c >> 6)) as usize] as char } else { '=' });
        out.push(if chunk.len() == 3 { TABLE[(c & 63) as usize] as char } else { '=' });
    }
    out
}
#[cfg(not(target_os = "windows"))]
pub fn register_wakeup(_dir: &Path, _due: i64) -> Result<(), String> {
    Err("Automatic wake-up while the app is closed is available on Windows only".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_audio_and_invalid_dates() {
        let dir = std::env::temp_dir().join(format!("windows-omr-defer-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(queue(&dir, 100, 100).is_err());
        assert!(queue(&dir, 300, 100).is_err());
        std::fs::create_dir_all(dir.join(crate::export::TRACKS_DIR)).unwrap();
        std::fs::write(dir.join(crate::export::TRACKS_DIR).join("source.ogg"), b"audio").unwrap();
        queue(&dir, 300, 100).unwrap();
        let j=load(&dir).unwrap();
        assert_eq!(j.status, "pending");
        assert_eq!(j.due, 300);
        mark(&dir, "running", None).unwrap();
        assert_eq!(load(&dir).unwrap().status, "running");
        mark(&dir, "failed", Some("offline")).unwrap();
        assert_eq!(load(&dir).unwrap().error.as_deref(), Some("offline"));
        cancel(&dir);
        assert!(load(&dir).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn encoded_powershell_is_valid_base64() {
        assert_eq!(base64_utf16(&[0x61,0,0x62,0]), "YQBiAA==");
    }
}
