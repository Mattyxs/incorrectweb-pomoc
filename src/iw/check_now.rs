//! IW: "check for an update now" for another program of the user (GoCorrect), without administrator rights.
//!
//! The service process (`--server`, SYSTEM) owns a named auto-reset event in the global namespace; any signed-in user
//! may only set it (EVENT_MODIFY_STATE). The signal carries nothing: no parameter, address or path comes from the
//! caller, so there is nothing to parse. A signal starts the usual update check of `src/updater.rs` (manifest,
//! SHA-256, signature, no session on, retry after 10 minutes during a session); signals closer than `MIN_GAP` to the
//! last accepted one are dropped. Only SYSTEM and services can create objects in `Global\`, so a user cannot create
//! the event first with a DACL of their own.

use std::time::{Duration, Instant};

/// Name of the event (`OpenEventW(EVENT_MODIFY_STATE, FALSE, EVENT_NAME)` + `SetEvent`).
pub const EVENT_NAME: &str = "Global\\GoCorrectPomocCheckUpdate";

/// DACL of the event: SYSTEM and the administrators full access, authenticated users EVENT_MODIFY_STATE (0x2) only.
pub const SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x2;;;AU)";

/// At most one accepted signal per this time; more are ignored (not queued).
pub const MIN_GAP: Duration = Duration::from_secs(5 * 60);

/// Drops signals that come sooner than `MIN_GAP` after the last accepted one.
#[derive(Default)]
pub struct Limiter {
    last: Option<Instant>,
}

impl Limiter {
    pub fn accept(&mut self, now: Instant) -> bool {
        if matches!(self.last, Some(last) if now.saturating_duration_since(last) < MIN_GAP) {
            return false;
        }
        self.last = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_signal_is_accepted_then_none_for_five_minutes() {
        let t = Instant::now();
        let mut l = Limiter::default();
        assert!(l.accept(t));
        assert!(!l.accept(t + Duration::from_secs(1)));
        assert!(!l.accept(t + MIN_GAP - Duration::from_millis(1)));
        assert!(l.accept(t + MIN_GAP));
    }

    #[test]
    fn dropped_signals_do_not_push_the_window() {
        let t = Instant::now();
        let mut l = Limiter::default();
        assert!(l.accept(t));
        for s in 1..300 {
            assert!(!l.accept(t + Duration::from_secs(s)));
        }
        assert!(l.accept(t + Duration::from_secs(300)), "5 minutes after the accepted one, not after the last dropped");
    }

    #[test]
    fn the_gap_is_five_minutes() {
        assert_eq!(MIN_GAP, Duration::from_secs(300));
    }

    #[test]
    fn users_may_only_signal_the_event() {
        // ACEs: SYSTEM and administrators full, authenticated users EVENT_MODIFY_STATE and nothing else
        let aces: Vec<&str> = SDDL.trim_start_matches("D:").split(')').filter(|a| !a.is_empty()).collect();
        assert_eq!(aces, ["(A;;GA;;;SY", "(A;;GA;;;BA", "(A;;0x2;;;AU"]);
        for who in [";WD", ";AN", ";BU", ";IU", ";NU", ";LS", ";NS"] {
            assert!(!SDDL.contains(who), "{who}");
        }
        assert!(!SDDL.contains("P(") && !SDDL.starts_with("O:"), "no owner or group of the caller's choice");
    }

    #[test]
    fn the_event_is_global_and_named_for_the_product() {
        assert!(EVENT_NAME.starts_with("Global\\"));
        assert!(EVENT_NAME.len() < 260 && !EVENT_NAME[7..].contains('\\'));
    }
}
