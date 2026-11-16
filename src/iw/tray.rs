//! IW: text of the tray icon (tooltip): the product name with its space, Polish plural of sessions,
//! a warning line when unattended access is on; the image of the Windows tray icon for its size.

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

/// Image data of the ICO entry for a tray icon of `want` px (Windows: `SM_CXSMICON`, 16 at 100 %, 24 at
/// 150 %, 32 at 200 %): the entry of that size, else the next larger one, else the largest. `None` for a file
/// that is not an icon or whose directory points past its end.
pub fn ico_entry_for(ico: &[u8], want: u32) -> Option<&[u8]> {
    if ico.len() < 6 || ico[..4] != [0, 0, 1, 0] {
        return None;
    }
    let n = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let e = ico.get(6 + 16 * i..6 + 16 * (i + 1))?;
        let size = if e[0] == 0 { 256 } else { e[0] as u32 };
        let len = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
        let off = u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize;
        entries.push((size, ico.get(off..off.checked_add(len)?)?));
    }
    let larger = entries.iter().filter(|(s, _)| *s >= want).min_by_key(|(s, _)| *s);
    larger.or_else(|| entries.iter().max_by_key(|(s, _)| *s)).map(|(_, data)| *data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ICO with one PNG-less dummy entry per size: the payload of each entry is its size repeated.
    fn ico(sizes: &[u32]) -> Vec<u8> {
        let mut head = vec![0, 0, 1, 0];
        head.extend((sizes.len() as u16).to_le_bytes());
        let mut body = Vec::new();
        let mut offset = 6 + 16 * sizes.len() as u32;
        for &s in sizes {
            let data = vec![s as u8; 8];
            let b = if s >= 256 { 0 } else { s as u8 };
            head.extend([b, b, 0, 0]);
            head.extend(1u16.to_le_bytes());
            head.extend(32u16.to_le_bytes());
            head.extend((data.len() as u32).to_le_bytes());
            head.extend(offset.to_le_bytes());
            offset += data.len() as u32;
            body.extend(data);
        }
        head.extend(body);
        head
    }

    #[test]
    fn ico_entry_is_the_one_drawn_for_the_small_icon_size() {
        let f = ico(&[16, 20, 24, 32, 40, 48]);
        for want in [16, 20, 24, 32, 40, 48] {
            assert_eq!(ico_entry_for(&f, want), Some(&[want as u8; 8][..]), "{want}");
        }
    }

    #[test]
    fn ico_entry_between_sizes_takes_the_next_larger_and_beyond_the_largest_the_largest() {
        let f = ico(&[32, 16, 24]);
        assert_eq!(ico_entry_for(&f, 18), Some(&[24u8; 8][..]));
        assert_eq!(ico_entry_for(&f, 64), Some(&[32u8; 8][..]));
        assert_eq!(ico_entry_for(&f, 1), Some(&[16u8; 8][..]));
        // 0 in the directory means 256
        assert_eq!(ico_entry_for(&ico(&[16, 256]), 200), Some(&[0u8; 8][..]));
    }

    #[test]
    fn ico_entry_rejects_broken_files() {
        assert_eq!(ico_entry_for(&[], 16), None);
        assert_eq!(ico_entry_for(&[0, 0, 2, 0, 1, 0], 16), None, "type 2 is a cursor");
        assert_eq!(ico_entry_for(&[0, 0, 1, 0, 0, 0], 16), None, "no images");
        let mut short = ico(&[16]);
        short.truncate(short.len() - 1);
        assert_eq!(ico_entry_for(&short, 16), None, "entry past the end");
        assert_eq!(ico_entry_for(&ico(&[16])[..10], 16), None, "directory past the end");
    }

    #[test]
    fn no_session_says_the_service_runs() {
        assert_eq!(tooltip(0, false), "GoCorrect Pomoc: usługa działa");
    }

    #[test]
    fn sessions_use_the_polish_plural() {
        assert_eq!(tooltip(1, false), "GoCorrect Pomoc: 1 aktywna sesja");
        assert_eq!(tooltip(2, false), "GoCorrect Pomoc: 2 aktywne sesje");
        assert_eq!(tooltip(4, false), "GoCorrect Pomoc: 4 aktywne sesje");
        assert_eq!(tooltip(5, false), "GoCorrect Pomoc: 5 aktywnych sesji");
        assert_eq!(tooltip(12, false), "GoCorrect Pomoc: 12 aktywnych sesji");
        assert_eq!(tooltip(14, false), "GoCorrect Pomoc: 14 aktywnych sesji");
        assert_eq!(tooltip(22, false), "GoCorrect Pomoc: 22 aktywne sesje");
        assert_eq!(tooltip(111, false), "GoCorrect Pomoc: 111 aktywnych sesji");
    }

    #[test]
    fn unattended_access_adds_a_warning_line() {
        assert_eq!(tooltip(0, true), "GoCorrect Pomoc: usługa działa\nDostęp bez akceptacji WŁĄCZONY");
        assert_eq!(tooltip(3, true), "GoCorrect Pomoc: 3 aktywne sesje\nDostęp bez akceptacji WŁĄCZONY");
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
