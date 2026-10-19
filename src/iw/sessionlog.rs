//! One line of `iw-sessions.log` per finished session: when, who, how, how long.

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() || c == '|' { ' ' } else { c })
        .collect::<String>()
        .trim()
        .chars()
        .take(120)
        .collect()
}

pub fn method_label(method: &str) -> &'static str {
    match method {
        "click" => "kliknięcie",
        "password" => "hasło i kliknięcie",
        "unattended" => "bez akceptacji",
        _ => "nieznany",
    }
}

/// `when` is the already formatted local time; `person` is empty for an unverified controller.
pub fn format_line(when: &str, person: &str, controller_id: &str, method: &str, duration_sec: u64) -> String {
    let person = clean(person);
    let who = if person.is_empty() { format!("niezweryfikowany ({})", clean(controller_id)) } else { person };
    format!("{} | {} | {} | {} s", clean(when), who, method_label(method), duration_sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_time_person_method_and_duration() {
        assert_eq!(
            format_line("2026-10-19 14:03:22", "Jan Kowalski (Incorrectus)", "555", "click", 61),
            "2026-10-19 14:03:22 | Jan Kowalski (Incorrectus) | kliknięcie | 61 s"
        );
    }

    #[test]
    fn unverified_controller_is_labelled_with_the_id() {
        let l = format_line("t", "", "555666777", "password", 5);
        assert_eq!(l, "t | niezweryfikowany (555666777) | hasło i kliknięcie | 5 s");
    }

    #[test]
    fn newlines_and_separators_cannot_forge_a_line() {
        let l = format_line("t", "Jan\n2026-01-01 | Admin | bez akceptacji | 0 s", "1", "unattended", 1);
        assert_eq!(l.lines().count(), 1);
        assert_eq!(l.matches('|').count(), 3);
    }
}
