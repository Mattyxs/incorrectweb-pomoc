//! Organization code (`XXXX-XXXX`) and the configuration it unlocks.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgConfig {
    pub name: String,
    pub host: String,
    pub key: String,
    pub api: String,
    /// Address of the organization's Panel (https, no trailing slash): its `/api/instance` gives the brand
    /// (logo, colours) set in Panel → Wygląd i marka. Empty when unknown.
    pub panel: String,
    /// Brand colours from the license server (`#rrggbb`), the fallback before the Panel answers. Empty when unknown.
    pub primary: String,
    pub accent: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgError {
    /// The organization exists but has no remote help (`remote: null`).
    NoRemote,
    /// The answer is not what the contract promises.
    Invalid,
}

/// Normalised code (`XXXX-XXXX`, Crockford base32) or None; same rules as the desktop app (`code.rs`).
pub fn normalize_code(input: &str) -> Option<String> {
    let mut out = String::with_capacity(9);
    for c in input.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c if c.is_ascii_digit() => c,
            c @ ('A'..='H' | 'J' | 'K' | 'M' | 'N' | 'P'..='T' | 'V'..='Z') => c,
            _ => return None,
        };
        if out.len() == 8 {
            return None;
        }
        out.push(c);
    }
    if out.len() != 8 {
        return None;
    }
    out.insert(4, '-');
    Some(out)
}

const FILE_PREFIX: &str = "incorrectwebpomoc-";

fn is_code_char(c: char) -> bool {
    matches!(c, '0'..='9' | 'A'..='H' | 'J' | 'K' | 'M' | 'N' | 'P'..='T' | 'V'..='Z')
}

/// Code from a file name such as `IncorrectWebPomoc-ABCD-EFGH.exe` (path and browser suffixes allowed).
/// Strict on purpose: only uppercase Crockford characters, so ordinary names are never taken for a code.
pub fn code_from_filename(path: &str) -> Option<String> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let start = name.to_ascii_lowercase().find(FILE_PREFIX)? + FILE_PREFIX.len();
    let mut chars = name.get(start..)?.chars().peekable();
    let mut code = String::with_capacity(8);
    for i in 0..8 {
        if i == 4 && chars.peek() == Some(&'-') {
            chars.next();
        }
        let c = chars.next()?;
        if !is_code_char(c) {
            return None;
        }
        code.push(c);
    }
    if chars.peek().map_or(false, |c| c.is_ascii_alphanumeric()) {
        return None;
    }
    normalize_code(&code)
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && !host.starts_with(['.', '-'])
        && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn valid_key(key: &str) -> bool {
    use hbb_common::base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.decode(key).map_or(false, |k| k.len() == 32)
}

fn valid_api(api: &str) -> bool {
    valid_api_with(api, cfg!(test))
}

/// https only; plain http to this very computer only when `allow_local` (test builds).
fn valid_api_with(api: &str, allow_local: bool) -> bool {

    let Ok(url) = url::Url::parse(api) else {
        return false;
    };
    match url.scheme() {
        "https" => url.host_str().is_some(),
        "http" => allow_local && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
        _ => false,
    }
}

/// Name for the screen: control characters become spaces, invisible direction marks are dropped.
fn display_name(n: &str) -> String {
    n.chars()
        .filter(|c| !matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .chars()
        .take(120)
        .collect()
}

/// What the user was asked about: the configuration is applied only after the answer "yes".
#[derive(Default)]
pub struct PendingOrg {
    slot: Option<(String, OrgConfig)>,
}

impl PendingOrg {
    pub fn put(&mut self, code: &str, cfg: OrgConfig) {
        self.slot = Some((code.to_owned(), cfg));
    }

    /// The configuration looked up for exactly this code; it can be taken once.
    pub fn take(&mut self, code: &str) -> Option<OrgConfig> {
        if self.slot.as_ref().map_or(false, |(c, _)| c == code) {
            self.slot.take().map(|(_, cfg)| cfg)
        } else {
            None
        }
    }
}

/// Question shown before an organization is applied.
pub fn confirm_prompt(name: &str) -> String {
    format!("Połączyć ten komputer z organizacją {name}?")
}

/// Reads the answer of `GET <license>/connect/<CODE>` (docs/connect/remote-api.md section 1).
pub fn parse_license_response(json: &str) -> Result<OrgConfig, OrgError> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| OrgError::Invalid)?;
    let remote = match v.get("remote") {
        None | Some(serde_json::Value::Null) => return Err(OrgError::NoRemote),
        Some(r) => r,
    };
    let text = |obj: &serde_json::Value, k: &str| obj.get(k).and_then(|x| x.as_str()).map(str::to_owned);
    let host = text(remote, "host").ok_or(OrgError::Invalid)?;
    let key = text(remote, "key").ok_or(OrgError::Invalid)?;
    let api = text(remote, "api").ok_or(OrgError::Invalid)?;
    if !valid_host(&host) || !valid_key(&key) || !valid_api(&api) {
        return Err(OrgError::Invalid);
    }
    let name = v
        .get("company")
        .and_then(|c| text(c, "name"))
        .map(|n| display_name(&n))
        .filter(|n| !n.is_empty())
        .ok_or(OrgError::Invalid)?;
    let panel = v
        .get("apps")
        .and_then(|a| text(a, "panel"))
        .filter(|p| valid_panel(p))
        .map(|p| p.trim_end_matches('/').to_owned())
        .unwrap_or_default();
    let colour = |k: &str| v.get("company").and_then(|c| text(c, k)).and_then(|c| hex_colour(&c)).unwrap_or_default();
    Ok(OrgConfig { name, host, key, api: api.trim_end_matches('/').to_owned(), panel, primary: colour("primary"), accent: colour("accent") })
}

/// Panel address: https with a host, no credentials, query or fragment.
fn valid_panel(p: &str) -> bool {
    let Ok(url) = url::Url::parse(p) else { return false };
    url.scheme() == "https" && url.host_str().is_some() && url.username().is_empty() && url.password().is_none() && url.query().is_none() && url.fragment().is_none()
}

/// `#rgb` or `#rrggbb` (any case) → `#rrggbb`; anything else → None.
fn hex_colour(c: &str) -> Option<String> {
    let s = c.trim().to_ascii_lowercase();
    let hex = s.strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        6 => Some(s),
        3 => Some(format!("#{}", hex.chars().flat_map(|ch| [ch, ch]).collect::<String>())),
        _ => None,
    }
}

impl OrgConfig {
    /// Options saved in the app config (server, relay, key, API, name).
    pub fn options(&self) -> Vec<(&'static str, String)> {
        vec![
            ("custom-rendezvous-server", self.host.clone()),
            ("relay-server", format!("{}:21117", self.host)),
            ("key", self.key.clone()),
            ("iw-api", self.api.clone()),
            ("iw-org-name", self.name.clone()),
            ("iw-panel-url", self.panel.clone()),
            ("iw-brand-primary", self.primary.clone()),
            ("iw-brand-accent", self.accent.clone()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "A6EHv/POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=";

    fn response(remote: &str) -> String {
        format!(
            r##"{{"company":{{"name":"Incorrectus","primary":"#431c77","accent":"#EF7D1D"}},"apps":{{"connect":"https://connect.incorrectus.com/"}},"remote":{remote}}}"##
        )
    }

    #[test]
    fn normalize_code_accepts_canonical_and_lookalikes() {
        assert_eq!(normalize_code("ABCD-EFGH").as_deref(), Some("ABCD-EFGH"));
        assert_eq!(normalize_code(" ab cd-ef gh ").as_deref(), Some("ABCD-EFGH"));
        assert_eq!(normalize_code("OoIi-LlAB").as_deref(), Some("0011-11AB"));
        assert_eq!(normalize_code("ABCD-EFG"), None);
        assert_eq!(normalize_code("ABCD-EFGHJ"), None);
        assert_eq!(normalize_code("ABCD-EFGU"), None);
        assert_eq!(normalize_code(""), None);
    }

    #[test]
    fn code_from_filename_reads_the_code_with_or_without_dash() {
        assert_eq!(code_from_filename("IncorrectWebPomoc-ABCD-EFGH.exe").as_deref(), Some("ABCD-EFGH"));
        assert_eq!(code_from_filename("IncorrectWebPomoc-ABCDEFGH.exe").as_deref(), Some("ABCD-EFGH"));
        assert_eq!(
            code_from_filename(r"C:\Users\Jan\Downloads\IncorrectWebPomoc-7K3M-9PQ2 (1).exe").as_deref(),
            Some("7K3M-9PQ2")
        );
        assert_eq!(code_from_filename("/tmp/IncorrectWebPomoc-ABCD-EFGH").as_deref(), Some("ABCD-EFGH"));
    }

    #[test]
    fn code_from_filename_ignores_names_without_a_code() {
        // release artifacts and renamed files
        assert_eq!(code_from_filename("IncorrectWebPomoc-1.4.9-iw.1-x64.exe"), None);
        assert_eq!(code_from_filename("IncorrectWebPomoc.exe"), None);
        assert_eq!(code_from_filename("IncorrectWebPomoc-Installer.exe"), None);
        assert_eq!(code_from_filename("pomoc-ABCD-EFGH.exe"), None);
        // a longer token is not a code
        assert_eq!(code_from_filename("IncorrectWebPomoc-ABCD-EFGHJ.exe"), None);
        assert_eq!(code_from_filename(""), None);
    }

    #[test]
    fn parse_license_response_reads_remote_and_company() {
        let cfg = parse_license_response(&response(&format!(
            r#"{{"host":"connect.incorrectus.com","key":"{KEY}","api":"https://connect.incorrectus.com"}}"#
        )))
        .unwrap();
        assert_eq!(cfg.name, "Incorrectus");
        assert_eq!(cfg.host, "connect.incorrectus.com");
        assert_eq!(cfg.key, KEY);
        assert_eq!(cfg.api, "https://connect.incorrectus.com");
    }

    #[test]
    fn license_response_keeps_panel_url_and_colours() {
        let cfg = parse_license_response(&format!(
            r##"{{"company":{{"name":"Smart-Biznes","primary":"#2F5B93","accent":"#c99700"}},"apps":{{"panel":"https://panel.smart-biznes.net/","connect":"https://connect.smart-biznes.net/"}},"remote":{{"host":"connect.smart-biznes.net","key":"{KEY}","api":"https://connect.smart-biznes.net"}}}}"##
        ))
        .unwrap();
        assert_eq!(cfg.panel, "https://panel.smart-biznes.net");
        assert_eq!(cfg.primary, "#2f5b93");
        assert_eq!(cfg.accent, "#c99700");
        let opts = cfg.options();
        assert!(opts.contains(&("iw-panel-url", "https://panel.smart-biznes.net".to_owned())));
        assert!(opts.contains(&("iw-brand-primary", "#2f5b93".to_owned())));
        assert!(opts.contains(&("iw-brand-accent", "#c99700".to_owned())));
    }

    #[test]
    fn license_response_drops_bad_panel_url_and_colours() {
        let cfg = parse_license_response(&format!(
            r##"{{"company":{{"name":"Firma","primary":"red;}}","accent":"#12345"}},"apps":{{"panel":"http://panel.firma.pl/"}},"remote":{{"host":"connect.firma.pl","key":"{KEY}","api":"https://connect.firma.pl"}}}}"##
        ))
        .unwrap();
        assert_eq!((cfg.panel.as_str(), cfg.primary.as_str(), cfg.accent.as_str()), ("", "", ""));
        // Empty values are written too: the next organization must not keep the previous one's look.
        assert!(cfg.options().contains(&("iw-panel-url", String::new())));
    }

    #[test]
    fn parse_license_response_without_remote_means_no_remote_help() {
        assert_eq!(parse_license_response(&response("null")), Err(OrgError::NoRemote));
        let no_field = r#"{"company":{"name":"Incorrectus"}}"#;
        assert_eq!(parse_license_response(no_field), Err(OrgError::NoRemote));
    }

    #[test]
    fn parse_license_response_rejects_unsafe_or_broken_values() {
        let bad = [
            // scheme in host, path in host, empty host
            format!(r#"{{"host":"https://connect.incorrectus.com","key":"{KEY}","api":"https://connect.incorrectus.com"}}"#),
            format!(r#"{{"host":"connect.incorrectus.com/x","key":"{KEY}","api":"https://connect.incorrectus.com"}}"#),
            format!(r#"{{"host":"","key":"{KEY}","api":"https://connect.incorrectus.com"}}"#),
            // key that is not 32 bytes of base64
            r#"{"host":"connect.incorrectus.com","key":"abc","api":"https://connect.incorrectus.com"}"#.to_owned(),
            // api over plain http to a public host
            format!(r#"{{"host":"connect.incorrectus.com","key":"{KEY}","api":"http://connect.incorrectus.com"}}"#),
            format!(r#"{{"host":"connect.incorrectus.com","key":"{KEY}","api":"javascript:alert(1)"}}"#),
        ];
        for remote in bad {
            assert_eq!(parse_license_response(&response(&remote)), Err(OrgError::Invalid), "{remote}");
        }
        assert_eq!(parse_license_response("not json"), Err(OrgError::Invalid));
    }

    #[test]
    fn parse_license_response_allows_plain_http_only_for_loopback() {
        let r = format!(r#"{{"host":"connect.incorrectus.com","key":"{KEY}","api":"http://127.0.0.1:3350"}}"#);
        assert!(parse_license_response(&response(&r)).is_ok());
    }

    #[test]
    fn production_rejects_plain_http_even_for_loopback() {
        for api in ["http://localhost", "http://localhost:3350", "http://127.0.0.1:3350", "http://[::1]:3350"] {
            assert!(valid_api_with(api, true), "{api} allowed in tests");
            assert!(!valid_api_with(api, false), "{api} must be refused in production");
        }
        assert!(valid_api_with("https://connect.incorrectus.com", false));
        assert!(!valid_api_with("http://connect.incorrectus.com", true));
    }

    fn cfg(name: &str) -> OrgConfig {
        OrgConfig { name: name.into(), host: "h.example".into(), key: KEY.into(), api: "https://h.example".into(), panel: String::new(), primary: String::new(), accent: String::new() }
    }

    #[test]
    fn pending_org_is_applied_only_for_the_code_that_was_shown_and_once() {
        let mut p = PendingOrg::default();
        assert_eq!(p.take("ABCD-EFGH"), None);
        p.put("ABCD-EFGH", cfg("Firma A"));
        assert_eq!(p.take("WXYZ-2345"), None);
        // a wrong code takes nothing and does not consume the shown organization; a new lookup replaces it
        p.put("ABCD-EFGH", cfg("Firma A"));
        p.put("WXYZ-2345", cfg("Firma B"));
        assert_eq!(p.take("ABCD-EFGH"), None);
        assert_eq!(p.take("WXYZ-2345").map(|c| c.name), Some("Firma B".to_owned()));
        assert_eq!(p.take("WXYZ-2345"), None);
    }

    #[test]
    fn confirm_prompt_names_the_organization() {
        assert_eq!(confirm_prompt("Incorrectus"), "Połączyć ten komputer z organizacją Incorrectus?");
    }

    #[test]
    fn organization_name_is_cleaned_of_control_characters() {
        let r = format!(r#"{{"host":"h.example","key":"{KEY}","api":"https://h.example"}}"#);
        let json = response(&r).replace("Incorrectus", r"Fir\nma\u0000 \u202eX");
        let cfg = parse_license_response(&json).unwrap();
        assert_eq!(cfg.name, "Fir ma  X");
    }

    #[test]
    fn options_map_to_rustdesk_server_settings() {
        let cfg = OrgConfig {
            name: "Incorrectus".into(),
            host: "connect.incorrectus.com".into(),
            key: KEY.into(),
            api: "https://connect.incorrectus.com".into(),
            panel: "https://panel.incorrectus.com".into(),
            primary: "#431c77".into(),
            accent: "#ef7d1d".into(),
        };
        let o: std::collections::HashMap<_, _> = cfg.options().into_iter().collect();
        assert_eq!(o["custom-rendezvous-server"], "connect.incorrectus.com");
        assert_eq!(o["relay-server"], "connect.incorrectus.com:21117");
        assert_eq!(o["key"], KEY);
        assert_eq!(o["iw-api"], "https://connect.incorrectus.com");
        assert_eq!(o["iw-org-name"], "Incorrectus");
    }
}
