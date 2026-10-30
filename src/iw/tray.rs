//! IW: text of the tray icon (tooltip): the product name with its space, Polish plural of sessions,
//! a warning line when unattended access is on.

/// Tooltip of the tray icon. `sessions` is the number of controlled sessions (0 = none).
pub fn tooltip(sessions: usize, unattended: bool) -> String {
    let state = match sessions {
        0 => "usługa działa".to_owned(),
        1 => "1 aktywna sesja".to_owned(),
        n if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) => format!("{n} aktywne sesje"),
        n => format!("{n} aktywnych sesji"),
    };
    let mut text = format!("{}: {state}", super::DISPLAY_NAME);
    if unattended {
        text.push_str("\nDostęp bez akceptacji WŁĄCZONY");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_session_says_the_service_runs() {
        assert_eq!(tooltip(0, false), "IncorrectWeb Pomoc: usługa działa");
    }

    #[test]
    fn sessions_use_the_polish_plural() {
        assert_eq!(tooltip(1, false), "IncorrectWeb Pomoc: 1 aktywna sesja");
        assert_eq!(tooltip(2, false), "IncorrectWeb Pomoc: 2 aktywne sesje");
        assert_eq!(tooltip(4, false), "IncorrectWeb Pomoc: 4 aktywne sesje");
        assert_eq!(tooltip(5, false), "IncorrectWeb Pomoc: 5 aktywnych sesji");
        assert_eq!(tooltip(12, false), "IncorrectWeb Pomoc: 12 aktywnych sesji");
        assert_eq!(tooltip(14, false), "IncorrectWeb Pomoc: 14 aktywnych sesji");
        assert_eq!(tooltip(22, false), "IncorrectWeb Pomoc: 22 aktywne sesje");
        assert_eq!(tooltip(111, false), "IncorrectWeb Pomoc: 111 aktywnych sesji");
    }

    #[test]
    fn unattended_access_adds_a_warning_line() {
        assert_eq!(tooltip(0, true), "IncorrectWeb Pomoc: usługa działa\nDostęp bez akceptacji WŁĄCZONY");
        assert_eq!(tooltip(3, true), "IncorrectWeb Pomoc: 3 aktywne sesje\nDostęp bez akceptacji WŁĄCZONY");
    }

    #[test]
    fn fits_the_windows_tooltip_and_uses_the_name_with_its_space() {
        for n in [0usize, 1, 5, 22, usize::MAX] {
            for u in [false, true] {
                let t = tooltip(n, u);
                // NOTIFYICONDATAW.szTip holds 128 UTF-16 units with the terminating zero
                assert!(t.encode_utf16().count() <= 127, "{t}");
                assert!(t.starts_with(super::super::DISPLAY_NAME), "{t}");
                assert!(!t.contains("IncorrectWebPomoc") && !t.contains('\u{2014}'), "{t}");
            }
        }
    }
}
