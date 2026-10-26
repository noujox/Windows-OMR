//! State machine for opt-in browser call detection.
//! This module does not record anything; the UI owns all effects.
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Background tabs can be timer-throttled to ~60s by Chromium.
// Prefer a late safety pause over interrupting a call that is still live.
const LOST_AFTER: Duration = Duration::from_secs(75);
const SHOW_AFTER: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Start,
    Pause,
    Resume,
    Show,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicState { Live, Muted, Unknown }

#[derive(Default)]
pub struct Satellite {
    active: HashMap<String, Instant>,
    mic_states: HashMap<String, MicState>,
    owned: bool,
    paused_by_us: bool,
    show_at: Option<Instant>,
}

impl Satellite {
    /// manually_busy includes recording, transcribing or stopping.
    /// Never take ownership of an existing user's recording.
    pub fn event(&mut self, action: &str, session: &str, manually_busy: bool, now: Instant) -> Option<Action> {
        if !valid_session(session) {
            return None;
        }
        match action {
            // The native host places this status BEFORE join/beat on the same
            // pipe, so a newly claimed recording can start with its mic gated.
            "satellite-mic-muted" | "satellite-mic-live" | "satellite-mic-unknown" => {
                let state = match action {
                    "satellite-mic-muted" => MicState::Muted,
                    "satellite-mic-live" => MicState::Live,
                    _ => MicState::Unknown,
                };
                self.mic_states.insert(session.to_owned(), state);
            }
            "satellite-join" | "satellite-beat" => {
                let had = self.active.contains_key(session);
                // Heartbeats aren't allowed to start sessions.
                if action == "satellite-beat" && !had { return None; }
                self.active.insert(session.to_owned(), now);
                if !self.owned {
                    // Only a new join may claim ownership. A heartbeat from
                    // a call ignored while a manual recording was active must
                    // never start a new recording after the user stops.
                    if manually_busy || action != "satellite-join" { return None; }
                    self.owned = true;
                    self.show_at = None;
                    return Some(Action::Start);
                }
                if self.paused_by_us {
                    self.paused_by_us = false;
                    self.show_at = None;
                    return Some(Action::Resume);
                }
            }
            "satellite-leave" => {
                self.active.remove(session);
                self.mic_states.remove(session);
                return self.maybe_pause(now);
            }
            _ => {}
        }
        None
    }

    fn maybe_pause(&mut self, now: Instant) -> Option<Action> {
        if self.owned && self.active.is_empty() && !self.paused_by_us {
            self.paused_by_us = true;
            self.show_at = Some(now + SHOW_AFTER);
            return Some(Action::Pause);
        }
        None
    }

    pub fn tick(&mut self, now: Instant, still_owned_and_paused: bool) -> Option<Action> {
        self.active.retain(|_, last| now.saturating_duration_since(*last) < LOST_AFTER);
        self.mic_states.retain(|session, _| self.active.contains_key(session));
        if self.owned && self.paused_by_us && !still_owned_and_paused {
            self.release();
            return None;
        }
        if let Some(action) = self.maybe_pause(now) {
            return Some(action);
        }
        if self.show_at.is_some_and(|due| now >= due) {
            self.release();
            return Some(Action::Show);
        }
        None
    }

    /// Fail closed if any active browser session is muted or has an unknown
    /// microphone state. This is used ONLY on a recording owned by Satellite.
    pub fn microphone_should_be_muted(&self) -> bool {
        self.active.is_empty()
            || self.active.keys().any(|session| self.mic_states.get(session) != Some(&MicState::Live))
    }

    pub fn owns_recording(&self) -> bool { self.owned }
    pub fn paused_by_us(&self) -> bool { self.paused_by_us }
    pub fn release(&mut self) {
        self.owned = false;
        self.paused_by_us = false;
        self.active.clear();
        self.mic_states.clear();
        self.show_at = None;
    }
}

pub fn valid_session(key: &str) -> bool {
    let Some((provider, token)) = key.split_once('.') else { return false };
    matches!(provider, "meet" | "teams" | "zoom")
        && (8..=64).contains(&token.len())
        && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> &'static str { "meet.session_01234567" }
    #[test]
    fn never_claims_manual_recordings() {
        let now = Instant::now();
        let mut s = Satellite::default();
        assert_eq!(s.event("satellite-join", key(), true, now), None);
        assert!(!s.owns_recording());
        assert_eq!(s.event("satellite-leave", key(), true, now), None);
    }
    #[test]
    fn pauses_once_and_shows_after_ten_seconds() {
        let now = Instant::now();
        let mut s = Satellite::default();
        assert_eq!(s.event("satellite-join", key(), false, now), Some(Action::Start));
        assert_eq!(s.event("satellite-leave", key(), true, now), Some(Action::Pause));
        assert_eq!(s.tick(now + Duration::from_secs(9), true), None);
        assert_eq!(s.tick(now + Duration::from_secs(10), true), Some(Action::Show));
        assert!(!s.owns_recording());
    }
    #[test]
    fn rejoin_resumes_only_our_pause() {
        let now = Instant::now();
        let mut s = Satellite::default();
        s.event("satellite-join", key(), false, now);
        assert_eq!(s.event("satellite-leave", key(), true, now), Some(Action::Pause));
        assert_eq!(s.event("satellite-join", key(), true, now + Duration::from_secs(4)), Some(Action::Resume));
        assert_eq!(s.tick(now + Duration::from_secs(80), false), Some(Action::Pause));
    }
    #[test]
    fn heartbeat_never_takes_over_after_manual_busy_session() {
        let now = Instant::now();
        let mut s = Satellite::default();
        // The call starts while the user is recording manually.
        assert_eq!(s.event("satellite-join", key(), true, now), None);
        assert!(!s.owns_recording());
        // The manual recording ends; this is the same browser call, not a
        // new call. A heartbeat must not silently start a new recording.
        assert_eq!(
            s.event("satellite-beat", key(), false, now + Duration::from_secs(5)),
            None
        );
        assert!(!s.owns_recording());
    }

    #[test]
    fn lost_tab_pauses_not_stops() {
        let now = Instant::now();
        let mut s = Satellite::default();
        s.event("satellite-join", key(), false, now);
        assert_eq!(s.tick(now + Duration::from_secs(76), false), Some(Action::Pause));
        assert_eq!(s.tick(now + Duration::from_secs(86), true), Some(Action::Show));
    }
    #[test]
    fn heartbeats_cannot_start_recording() {
        let now = Instant::now();
        let mut s = Satellite::default();
        assert_eq!(s.event("satellite-beat", key(), false, now), None);
    }
    #[test]
    fn smart_mic_is_scoped_to_owned_sessions_and_fails_closed() {
        let now = Instant::now();
        let mut s = Satellite::default();
        // Mic state precedes join, but must never claim ownership itself.
        assert_eq!(s.event("satellite-mic-muted", key(), false, now), None);
        assert!(!s.owns_recording());
        assert_eq!(s.event("satellite-join", key(), false, now), Some(Action::Start));
        assert!(s.microphone_should_be_muted());
        assert_eq!(s.event("satellite-mic-live", key(), true, now), None);
        assert!(!s.microphone_should_be_muted());
        assert_eq!(s.event("satellite-mic-unknown", key(), true, now), None);
        assert!(s.microphone_should_be_muted());
        assert_eq!(s.event("satellite-leave", key(), true, now), Some(Action::Pause));
        assert!(s.microphone_should_be_muted());
        s.release();
        assert!(!s.owns_recording());
    }

    #[test]
    fn smart_mic_requires_all_active_tabs_to_be_live() {
        let now = Instant::now();
        let other = "meet.another_session_123";
        let mut s = Satellite::default();
        s.event("satellite-mic-live", key(), false, now);
        s.event("satellite-join", key(), false, now);
        assert!(!s.microphone_should_be_muted());
        s.event("satellite-join", other, true, now);
        assert!(s.microphone_should_be_muted(), "unknown second tab must not open mic");
        s.event("satellite-mic-live", other, true, now);
        assert!(!s.microphone_should_be_muted());
        s.event("satellite-mic-muted", key(), true, now);
        assert!(s.microphone_should_be_muted());
        s.event("satellite-leave", key(), true, now);
        assert!(!s.microphone_should_be_muted());
    }

    #[test]
    fn safe_key_validation() {
        assert!(valid_session("zoom.abcd_1234567"));
        assert!(!valid_session("zoom.hello\nstart"));
        assert!(!valid_session("meet.short"));
        assert!(!valid_session("http.local-session_123"));
    }
}
