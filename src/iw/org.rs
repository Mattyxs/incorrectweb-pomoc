//! Organization code (`XXXX-XXXX`) and the configuration it unlocks.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgConfig {
    pub name: String,
    pub host: String,
    pub key: String,
    pub api: String,
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

/// https only; plain http is for a server on this very computer (development).
fn valid_api(api: &str) -> bool {
    let Ok(url) = url::Url::parse(api) else {
        return false;
    };
    match url.scheme() {
        "https" => url.host_str().is_some(),
        "http" => matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
        _ => false,
    }
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
        .map(|n| n.trim().chars().take(120).collect::<String>())
        .filter(|n| !n.is_empty())
        .ok_or(OrgError::Invalid)?;
    Ok(OrgConfig { name, host, key, api: api.trim_end_matches('/').to_owned() })
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
    fn options_map_to_rustdesk_server_settings() {
        let cfg = OrgConfig {
            name: "Incorrectus".into(),
            host: "connect.incorrectus.com".into(),
            key: KEY.into(),
            api: "https://connect.incorrectus.com".into(),
        };
        let o: std::collections::HashMap<_, _> = cfg.options().into_iter().collect();
        assert_eq!(o["custom-rendezvous-server"], "connect.incorrectus.com");
        assert_eq!(o["relay-server"], "connect.incorrectus.com:21117");
        assert_eq!(o["key"], KEY);
        assert_eq!(o["iw-api"], "https://connect.incorrectus.com");
        assert_eq!(o["iw-org-name"], "Incorrectus");
    }
}
