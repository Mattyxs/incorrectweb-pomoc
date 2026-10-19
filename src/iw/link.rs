//! `iwpomoc://connect/<id>?intent=<token>`: the link a consultant's browser hands to the app,
//! and the one-time store that carries the token to the login request (never to history or logs).

use std::{collections::HashMap, fmt, time::{Duration, Instant}};

pub const SCHEME: &str = "iwpomoc";
pub const INTENT_TTL: Duration = Duration::from_secs(120);

#[derive(Clone, PartialEq, Eq)]
pub struct IwLink {
    pub id: String,
    pub intent: Option<String>,
}

impl fmt::Debug for IwLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IwLink")
            .field("id", &self.id)
            .field("intent", &self.intent.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// None for anything that is not exactly `iwpomoc://connect/<9-10 digits>[?intent=<token>]`.
pub fn parse_link(link: &str) -> Option<IwLink> {
    let prefix = format!("{SCHEME}://connect/");
    let head = link.get(..prefix.len())?;
    if !head.eq_ignore_ascii_case(&prefix) {
        return None;
    }
    let rest = link[prefix.len()..].split('#').next().unwrap_or_default();
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let id = path.strip_suffix('/').unwrap_or(path);
    if !(9..=10).contains(&id.len()) || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut intent = None;
    for pair in query.unwrap_or_default().split('&') {
        if let Some(("intent", value)) = pair.split_once('=') {
            let valid = (20..=64).contains(&value.len())
                && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
            if intent.is_some() || !valid {
                return None;
            }
            intent = Some(value.to_owned());
        }
    }
    Some(IwLink { id: id.to_owned(), intent })
}

/// What the first command line argument is (the program started by a link handler gets the link there).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchArg {
    /// No argument or an ordinary one (`--server`, `--connect`, a path, an ID).
    Plain,
    /// `iwpomoc://...`: only the link is kept, every argument after it is dropped.
    Link,
    /// A link of any other scheme (`incorrectwebpomoc://` of the installer, `rustdesk://`, `https://`): ignored.
    Foreign,
}

pub fn classify_launch_arg(first: Option<&str>) -> LaunchArg {
    let Some(a) = first else {
        return LaunchArg::Plain;
    };
    let prefix = format!("{SCHEME}://");
    if a.get(..prefix.len()).map_or(false, |h| h.eq_ignore_ascii_case(&prefix)) {
        return LaunchArg::Link;
    }
    // `scheme:` with a scheme of 2+ characters (a drive letter `C:` is one character)
    let scheme = a.split(':').next().unwrap_or_default();
    let is_url = a.contains(':')
        && scheme.len() >= 2
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if is_url { LaunchArg::Foreign } else { LaunchArg::Plain }
}

#[derive(Default)]
pub struct IntentStore {
    items: HashMap<String, (String, Instant)>,
}

impl fmt::Debug for IntentStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IntentStore({} items)", self.items.len())
    }
}

impl IntentStore {
    pub fn put(&mut self, id: &str, token: &str, now: Instant) {
        self.items.retain(|_, (_, at)| now.saturating_duration_since(*at) <= INTENT_TTL);
        self.items.insert(id.to_owned(), (token.to_owned(), now));
    }

    /// The token for this peer, once; None when absent or older than [`INTENT_TTL`].
    pub fn take(&mut self, id: &str, now: Instant) -> Option<String> {
        let (token, at) = self.items.remove(id)?;
        (now.saturating_duration_since(at) <= INTENT_TTL).then_some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "k3Jx9fQ2m-vYh7TzP_aLw0sBnE4uRc6dGiO1XtVyMqA";

    #[test]
    fn parses_connect_link_with_intent() {
        let l = parse_link(&format!("iwpomoc://connect/123456789?intent={TOKEN}")).unwrap();
        assert_eq!(l.id, "123456789");
        assert_eq!(l.intent.as_deref(), Some(TOKEN));
    }

    #[test]
    fn parses_connect_link_without_intent_and_with_trailing_slash() {
        assert_eq!(parse_link("iwpomoc://connect/1234567890").unwrap().intent, None);
        assert_eq!(parse_link("IWPOMOC://connect/123456789/").unwrap().id, "123456789");
    }

    #[test]
    fn other_paths_and_schemes_are_ignored() {
        for l in [
            "iwpomoc://open/123456789",
            "iwpomoc://connect/123456789/extra",
            "iwpomoc://connect/",
            "iwpomoc://connect/12345",
            "iwpomoc://connect/12345678901",
            "iwpomoc://connect/12345678a",
            "iwpomoc://connect/..%2f123456789",
            "rustdesk://connection/new/123456789",
            "https://connect/123456789",
            "iwpomoc:connect/123456789",
            "",
        ] {
            assert_eq!(parse_link(l), None, "{l}");
        }
    }

    #[test]
    fn bad_or_duplicated_intent_is_not_a_link() {
        assert_eq!(parse_link("iwpomoc://connect/123456789?intent="), None);
        assert_eq!(parse_link("iwpomoc://connect/123456789?intent=zl%20y"), None);
        assert_eq!(parse_link("iwpomoc://connect/123456789?intent=short"), None);
        assert_eq!(
            parse_link(&format!("iwpomoc://connect/123456789?intent={TOKEN}&intent={TOKEN}")),
            None
        );
    }

    #[test]
    fn unknown_query_parameters_are_ignored() {
        let l = parse_link(&format!("iwpomoc://connect/123456789?x=1&intent={TOKEN}#frag")).unwrap();
        assert_eq!(l.intent.as_deref(), Some(TOKEN));
    }

    #[test]
    fn intent_is_kept_out_of_debug_output() {
        let l = parse_link(&format!("iwpomoc://connect/123456789?intent={TOKEN}")).unwrap();
        assert!(!format!("{l:?}").contains(TOKEN));
        let mut s = IntentStore::default();
        s.put("123456789", TOKEN, Instant::now());
        assert!(!format!("{s:?}").contains(TOKEN));
    }

    #[test]
    fn intent_is_taken_once_and_only_for_its_peer() {
        let now = Instant::now();
        let mut s = IntentStore::default();
        s.put("123456789", TOKEN, now);
        assert_eq!(s.take("999999999", now), None);
        assert_eq!(s.take("123456789", now).as_deref(), Some(TOKEN));
        assert_eq!(s.take("123456789", now), None);
    }

    #[test]
    fn intent_expires_after_two_minutes() {
        let now = Instant::now();
        let mut s = IntentStore::default();
        s.put("123456789", TOKEN, now);
        assert_eq!(s.take("123456789", now + Duration::from_secs(121)), None);
        s.put("123456789", TOKEN, now);
        assert_eq!(s.take("123456789", now + Duration::from_secs(119)).as_deref(), Some(TOKEN));
    }

    #[test]
    fn launch_with_our_link_keeps_only_the_link() {
        assert_eq!(classify_launch_arg(Some("iwpomoc://connect/123456789?intent=x")), LaunchArg::Link);
        assert_eq!(classify_launch_arg(Some("IWPOMOC://connect/123456789")), LaunchArg::Link);
    }

    #[test]
    fn launch_with_a_link_of_another_scheme_is_ignored() {
        for a in [
            "incorrectwebpomoc://connection/new/123456789?password=x",
            "incorrectwebpomoc:connection/new/123456789",
            "rustdesk://123456789",
            "https://example.com/",
            "iwpomoc:connect/123456789",
        ] {
            assert_eq!(classify_launch_arg(Some(a)), LaunchArg::Foreign, "{a}");
        }
    }

    #[test]
    fn ordinary_arguments_are_plain() {
        for a in ["--server", "--connect", "--cm", r"C:\Users\Jan\x.incorrectwebpomoc", "/tmp/x", "123456789"] {
            assert_eq!(classify_launch_arg(Some(a)), LaunchArg::Plain, "{a}");
        }
        assert_eq!(classify_launch_arg(None), LaunchArg::Plain);
    }
}
