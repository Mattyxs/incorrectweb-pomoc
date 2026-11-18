//! IW: updates of IncorrectWeb Pomoc from the update server of IncorrectWeb (not from RustDesk). The manifest
//! `pomoc.json` names the newest version and one package per platform with its SHA-256 and a minisign signature:
//! `pomocSignature` with the Pomoc key (POMOC_KEY, since 18.11.2026) or, during the transition, `signature` with the key
//! of IncorrectWeb releases (the key of the IncorrectWeb desktop app). A package is used only when both match.
//!
//! ```json
//! {"version": "1.4.9-iw.7", "minimum": "1.4.9-iw.6",
//!  "platforms": {"windows-x86_64": {"url": "https://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-1.4.9-iw.7-x64.exe",
//!                                   "sha256": "<64 hex>", "signature": "<.sig of `tauri signer sign`, release key>",
//!                                   "pomocSignature": "<.sig of `tauri signer sign`, Pomoc key>"}}}
//! ```

use std::cmp::Ordering;

pub const MANIFEST_URL: &str = "https://license.incorrectus.com/desktop/pomoc.json";
/// The only host packages are taken from, and the only folder there.
pub const PACKAGE_HOST: &str = "license.incorrectus.com";
pub const PACKAGE_PATH: &str = "/desktop/pomoc/";
/// Public minisign key of IncorrectWeb releases (`plugins.updater.pubkey` of apps/desktop/src-tauri/tauri.conf.json).
pub const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDRFRUU4NDUwOEUxQkZGQzUKUldURi94dU9VSVR1VHVDODRRdjNsQTlTNEFmSHgxeHlYRlVIUWJEOFBhZ1RYVngzaUNFOC9CQ0gK";
/// Public minisign key of Pomoc packages (key id 24F2AA81840412CB, 18.11.2026; `POMOC_KEY` of
/// apps/desktop/src-tauri/src/pomoc_update.rs): its own key, not the one of the desktop app's updates.
pub const POMOC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDI0RjJBQTgxODQwNDEyQ0IKUldUTEVnU0VnYXJ5Sk1paHdpNWdLZ1NCTUhFcERWRU1NeGcrSitUdjBwOVlOU01BbnNmVEp4U1oK";
/// Transition: a package signed only with the release key (`signature`, pomoc.json up to 1.4.9-iw.10) still passes.
/// Turned off together with the desktop app (`ACCEPT_RELEASE_KEY` of pomoc_update.rs) once every installed Pomoc checks
/// the Pomoc key.
pub const ACCEPT_RELEASE_KEY: bool = true;
/// The keys the installed program checks packages with.
pub const KEYS: Keys<'static> = Keys { pomoc: POMOC_KEY, release: Some(PUBLIC_KEY) };

/// The keys a package may be signed with: the Pomoc key, and the release key while ACCEPT_RELEASE_KEY holds.
#[derive(Debug, Clone, Copy)]
pub struct Keys<'a> {
    pub pomoc: &'a str,
    pub release: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub url: String,
    pub sha256: String,
    /// With the release key (`signature`); empty when the manifest has none.
    pub signature: String,
    /// With the Pomoc key (`pomocSignature`).
    pub pomoc_signature: Option<String>,
}

/// A newer version for this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub version: String,
    /// This version is below the manifest's `minimum`.
    pub required: bool,
    pub package: Package,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    Manifest,
    /// The manifest has no package for this platform (not published yet).
    NoPackage,
    /// The package address is not https on the update host, or its file name is unusable.
    Address,
    Checksum,
    Signature,
}

/// The key of this platform in the manifest; None where Pomoc has no package.
pub fn platform_key(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("windows-x86_64"),
        ("macos", "aarch64") => Some("darwin-aarch64"),
        ("macos", "x86_64") => Some("darwin-x86_64"),
        _ => None,
    }
}

fn parts(v: &str) -> Option<[u64; 4]> {
    let (base, iw) = match v.trim().split_once('-') {
        Some((b, rest)) => (b, rest.strip_prefix("iw.")?.parse::<u64>().ok()?),
        None => (v.trim(), 0),
    };
    let n: Vec<u64> = base.split('.').map(|p| p.parse::<u64>().ok()).collect::<Option<_>>()?;
    match n[..] {
        [a, b, c] => Some([a, b, c, iw]),
        _ => None,
    }
}

/// Order of `1.4.9-iw.7` versions (`1.4.9` = `1.4.9-iw.0`); None for anything else.
pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    Some(parts(a)?.cmp(&parts(b)?))
}

/// What the manifest offers to `current` on `platform`. Ok(None): nothing newer.
pub fn offer(manifest: &str, current: &str, platform: &str) -> Result<Option<Offer>, UpdateError> {
    let m: serde_json::Value = serde_json::from_str(manifest).map_err(|_| UpdateError::Manifest)?;
    let version = m.get("version").and_then(|v| v.as_str()).filter(|v| parts(v).is_some()).ok_or(UpdateError::Manifest)?;
    if compare(version, current) != Some(Ordering::Greater) {
        return Ok(None);
    }
    let p = m.get("platforms").and_then(|p| p.get(platform)).ok_or(UpdateError::NoPackage)?;
    let field = |k: &str| p.get(k).and_then(|v| v.as_str()).map(str::to_owned).ok_or(UpdateError::Manifest);
    let optional = |k: &str| p.get(k).map(|v| v.as_str().map(str::to_owned).ok_or(UpdateError::Manifest)).transpose();
    let (signature, pomoc_signature) = (optional("signature")?, optional("pomocSignature")?);
    if signature.is_none() && pomoc_signature.is_none() {
        return Err(UpdateError::Manifest);
    }
    let package = Package { url: field("url")?, sha256: field("sha256")?.to_lowercase(), signature: signature.unwrap_or_default(), pomoc_signature };
    // Only `https://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-<version>-…`: never another file of the
    // update server (the desktop installer) and never an older package under the new version.
    let name = file_name(&package.url).ok_or(UpdateError::Address)?;
    let on_host = url::Url::parse(&package.url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str() == Some(PACKAGE_HOST)
            && u.username().is_empty()
            && u.password().is_none()
            && u.port().is_none()
            && u.path() == format!("{PACKAGE_PATH}{name}")
    });
    if !on_host || !name.starts_with(&format!("IncorrectWebPomoc-{version}-")) {
        return Err(UpdateError::Address);
    }
    if package.sha256.len() != 64 || !package.sha256.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(UpdateError::Manifest);
    }
    let minimum = m.get("minimum").and_then(|v| v.as_str());
    let required = minimum.is_some_and(|min| compare(current, min) == Some(Ordering::Less) && compare(version, min) != Some(Ordering::Less));
    Ok(Some(Offer { version: version.to_owned(), required, package }))
}

/// The name the package is saved under (the last part of its address); only plain names of .exe and .dmg files.
pub fn file_name(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    if u.query().is_some() || u.fragment().is_some() {
        return None;
    }
    let name = u.path_segments()?.last()?;
    let plain = !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)) && !name.starts_with('.');
    (plain && (name.ends_with(".exe") || name.ends_with(".dmg"))).then(|| name.to_owned())
}

/// The downloaded package is the published one: its SHA-256 and a minisign signature made for the file of the address
/// (`file:<name>` in the trusted comment, as `tauri signer sign` writes it): `pomocSignature` with the Pomoc key, or
/// (transition) `signature` with the release key.
pub fn verify(bytes: &[u8], package: &Package, keys: &Keys) -> Result<(), UpdateError> {
    use sha2::{Digest, Sha256};
    if !hex::encode(Sha256::digest(bytes)).eq_ignore_ascii_case(&package.sha256) {
        return Err(UpdateError::Checksum);
    }
    let name = file_name(&package.url).ok_or(UpdateError::Signature)?;
    let file = format!("file:{name}");
    let by_pomoc = package.pomoc_signature.as_deref().is_some_and(|sig| signed(bytes, sig, keys.pomoc, &file));
    let by_release = ACCEPT_RELEASE_KEY && keys.release.is_some_and(|key| signed(bytes, &package.signature, key, &file));
    if by_pomoc || by_release {
        Ok(())
    } else {
        Err(UpdateError::Signature)
    }
}

/// `signature` (base64 of a minisign signature file, as `tauri signer` writes it) of `bytes` with `public_key` (base64
/// of the public key file), naming `file` in its trusted comment (the same check as the desktop updater).
fn signed(bytes: &[u8], signature: &str, public_key: &str, file: &str) -> bool {
    use hbb_common::base64::{engine::general_purpose::STANDARD, Engine};
    let text = |b64: &str| STANDARD.decode(b64.trim()).ok().and_then(|b| String::from_utf8(b).ok());
    let Some(key) = text(public_key).and_then(|t| minisign_verify::PublicKey::decode(&t).ok()) else { return false };
    let Some(sig) = text(signature).and_then(|t| minisign_verify::Signature::decode(&t).ok()) else { return false };
    key.verify(bytes, &sig, true).is_ok() && sig.trusted_comment().split('\t').any(|field| field == file)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A throwaway key made for these tests (`tauri signer generate`), never used for releases, and files it signed under
    // their release names (the name is in the trusted comment of the signature: `file:<name>`).
    const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQwQ0MxNDc5MTJCREUwNDkKUldSSjRMMFNlUlRNUUk1YXErbUkwa0lsdHpHdW9ZR3N6azNOcG9kZjY5TCtoSmw4N3ZEWVNRVHkK";
    const TEST_FILE: &[u8] = b"IncorrectWeb Pomoc test package 1.4.9-iw.7\n";
    const TEST_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSSjRMMFNlUlRNUUQ3ZWxOTUVJQzQ5bzIzQzBOdk8wM1dUOW5zUms1c0Y4V3pnOWFNUjcyaHFWclNwVjJwSVNnUlZMVFlra2NaWm13UVNhamh6S3YwWUh3akcwdW12YkFRPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkzMjE1NzU1CWZpbGU6SW5jb3JyZWN0V2ViUG9tb2MtMS40LjktaXcuNy14NjQuZXhlClB6N2lnUVRVb0JzMzg4c3FvR0JidzVvZjRXYmgxMU5UWnVydjViZEhDTS9PSWNjZFErZjlzMFVGT2F0MHhVMXNGYmRnUTA3L2tNOWlDRkJ3RXhzU0J3PT0K";
    /// An older package, signed under its own name.
    const OLD_FILE: &[u8] = b"IncorrectWeb Pomoc test package 1.4.9-iw.5\n";
    const OLD_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSSjRMMFNlUlRNUUdrTGU1UWIvUVp6RS91NTh3WE8zVUxhUXAxb1Z5VitaWVJuT1Q4d1AwZ25Lc3F6UXJTbHBuMm9Lc1NIaFg2bElBS2s3dllBbFpZejhNbWJHZTJGaUFFPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkzMjE1NzU2CWZpbGU6SW5jb3JyZWN0V2ViUG9tb2MtMS40LjktaXcuNS14NjQuZXhlCnVGaFhoQWk5cmdyRXBXZWpYTU9kVnUyOUlNUlJybklJczJud0VvUkNkMisrRGxNTWtUb0htZmF6bFBEYmJmdS80YWpOdWRqOGJlN2pwbGpGTWVwdkNnPT0K";
    /// The bytes of TEST_FILE signed as `pkg.bin`.
    const OTHER_NAME_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVSSjRMMFNlUlRNUU1RWS9zbEJiMU1IOFI4UHJmeEJvRjRoUWNZb01yUU5GMXl0ajM4anl3Yjc0dW5ReERocW8rSGorSFVDQVdqSzRscFF4YUh3S2FhRXkxdHZRVlhZVGdZPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkzMjE1NzU2CWZpbGU6cGtnLmJpbgowNEo5Ky9iTjBaQWZaaFd3ZWsxOWNlc3ViT1lOZDlLN0tEcE0yVEkvaVN6Y3hLM29rZWIwOWNXZUNwYXdZNWJSRGdBWGZWaXJQUXhzL2h2WnBqV3REZz09Cg==";

    fn sha(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(bytes))
    }

    fn package() -> Package {
        Package { url: "https://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-1.4.9-iw.7-x64.exe".into(), sha256: sha(TEST_FILE), signature: TEST_SIG.into(), pomoc_signature: None }
    }

    /// A throwaway key standing in for POMOC_KEY and TEST_FILE signed with it under its release name and as `pkg.bin`
    /// (the same vectors as pomoc_update_tests.rs of the desktop app).
    const TEST_POMOC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDAyRUEwQTFBNkEzMDk2QTIKUldTaWxqQnFHZ3JxQXVBWFJHY2cwWW4ydyttM1Byck9yelJkKzV6elhqRmM1NEdZMnR6bWVOMDgK";
    const TEST_POMOC_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTaWxqQnFHZ3JxQWsyWEZMRkhTV3NDNDR0aVNGdy81MTdCd2RrU3pBME4zNXJWVW5iSW1CSmlrNFBzRWQvNXdhRDYxeFhXVzZvdDY4MW1UZkRDNmZBazdrbk9QaUZWU2dZPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk1MDM4NzE4CWZpbGU6SW5jb3JyZWN0V2ViUG9tb2MtMS40LjktaXcuNy14NjQuZXhlCjRLaDdlcTRjRlNZTzNMYWdzV2k3UzlMMFQ1QkMzNVZWTjZxTUJDZ0JSUE8vMHhGbE4yQUIxd1A3L3p2Z3RldVNrQzVsR0M0VDlZSFBpSE41UjU3YUR3PT0K";
    const TEST_POMOC_OTHER_NAME_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTaWxqQnFHZ3JxQXE0d2JQcmF6aHZxejJ0WDRXSUNkd2ZxOUdxNFBrYUZCVDJWNWxxaGVyNkJ0TVBuSms3RWx3QnhJRzh6czQwWFRWczNpU0JxTmM2Vk42RmxoRnRVemdBPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk1MDM4NzE5CWZpbGU6cGtnLmJpbgpSSW1WU0RURG5ybmVoUE5vL3poVkFTajBkY0kvbm04aVk0UlVmdUJReWd6V1M3TGJyV2lBSzBySnZnZDBmZjZnbkN4ci9Gck5RMldqWXpUblA3N0hCZz09Cg==";

    fn keys(release: &str) -> Keys<'_> {
        Keys { pomoc: TEST_POMOC_KEY, release: Some(release) }
    }

    fn pomoc_signed() -> Package {
        Package { signature: String::new(), pomoc_signature: Some(TEST_POMOC_SIG.into()), ..package() }
    }

    #[test]
    fn a_package_signed_with_the_pomoc_key_passes_without_the_release_signature() {
        assert_eq!(verify(TEST_FILE, &pomoc_signed(), &keys(TEST_KEY)), Ok(()));
        assert_eq!(verify(TEST_FILE, &pomoc_signed(), &Keys { pomoc: TEST_POMOC_KEY, release: None }), Ok(()));
        assert_eq!(verify(TEST_FILE, &Package { pomoc_signature: Some(TEST_POMOC_SIG.into()), ..package() }, &keys(TEST_KEY)), Ok(()));
    }

    #[test]
    fn during_the_transition_a_package_signed_only_with_the_release_key_passes() {
        assert!(ACCEPT_RELEASE_KEY);
        assert_eq!(verify(TEST_FILE, &package(), &keys(TEST_KEY)), Ok(()));
        assert_eq!(verify(TEST_FILE, &package(), &Keys { pomoc: TEST_POMOC_KEY, release: None }), Err(UpdateError::Signature));
    }

    #[test]
    fn a_pomoc_signature_of_another_key_or_file_fails() {
        let swapped = Package { signature: String::new(), pomoc_signature: Some(TEST_SIG.into()), ..package() };
        assert_eq!(verify(TEST_FILE, &swapped, &keys(TEST_KEY)), Err(UpdateError::Signature));
        let other_name = Package { pomoc_signature: Some(TEST_POMOC_OTHER_NAME_SIG.into()), ..pomoc_signed() };
        assert_eq!(verify(TEST_FILE, &other_name, &keys(TEST_KEY)), Err(UpdateError::Signature));
        let renamed = Package { url: package().url.replace("iw.7-x64", "iw.7-x86"), ..pomoc_signed() };
        assert_eq!(verify(TEST_FILE, &renamed, &keys(TEST_KEY)), Err(UpdateError::Signature));
        let broken = Package { pomoc_signature: Some("bm90IGEgc2lnbmF0dXJl".into()), ..package() };
        assert_eq!(verify(TEST_FILE, &broken, &keys(TEST_KEY)), Ok(()), "a broken Pomoc signature does not hide a good release one");
    }

    #[test]
    fn the_manifest_carries_the_pomoc_signature_next_to_the_release_signature() {
        let both = manifest("1.4.9-iw.7", None).replace(r#""signature":"#, &format!(r#""pomocSignature":"{TEST_POMOC_SIG}","signature":"#));
        let o = offer(&both, "1.4.9-iw.6", "windows-x86_64").unwrap().unwrap();
        assert_eq!(o.package, Package { pomoc_signature: Some(TEST_POMOC_SIG.into()), ..package() });
        let only_pomoc = both.replace(&format!(r#","signature":"{TEST_SIG}""#), "");
        assert_eq!(offer(&only_pomoc, "1.4.9-iw.6", "windows-x86_64").unwrap().unwrap().package, pomoc_signed());
        let none = manifest("1.4.9-iw.7", None).replace(&format!(r#","signature":"{TEST_SIG}""#), "");
        assert_eq!(offer(&none, "1.4.9-iw.6", "windows-x86_64"), Err(UpdateError::Manifest));
    }

    #[test]
    fn the_pomoc_key_is_its_own_key() {
        use hbb_common::base64::{engine::general_purpose::STANDARD, Engine};
        let text = String::from_utf8(STANDARD.decode(POMOC_KEY).unwrap()).unwrap();
        assert!(text.starts_with("untrusted comment: minisign public key: 24F2AA81840412CB\n"), "{text}");
        assert!(minisign_verify::PublicKey::decode(&text).is_ok());
        assert_ne!(POMOC_KEY, PUBLIC_KEY);
        assert_eq!(KEYS.pomoc, POMOC_KEY);
        assert_eq!(KEYS.release, Some(PUBLIC_KEY));
    }

    fn manifest(version: &str, minimum: Option<&str>) -> String {
        let p = package();
        let min = minimum.map(|m| format!(r#""minimum":"{m}","#)).unwrap_or_default();
        format!(r#"{{"version":"{version}",{min}"platforms":{{"windows-x86_64":{{"url":"{}","sha256":"{}","signature":"{}"}}}}}}"#, p.url, p.sha256, p.signature)
    }

    #[test]
    fn platform_keys() {
        assert_eq!(platform_key("windows", "x86_64"), Some("windows-x86_64"));
        assert_eq!(platform_key("macos", "aarch64"), Some("darwin-aarch64"));
        assert_eq!(platform_key("macos", "x86_64"), Some("darwin-x86_64"));
        assert_eq!(platform_key("windows", "aarch64"), None);
        assert_eq!(platform_key("linux", "x86_64"), None);
    }

    #[test]
    fn versions_compare_by_numbers_and_the_iw_release() {
        assert_eq!(compare("1.4.9-iw.6", "1.4.9-iw.7"), Some(Ordering::Less));
        assert_eq!(compare("1.4.9-iw.10", "1.4.9-iw.9"), Some(Ordering::Greater), "numbers, not text");
        assert_eq!(compare("1.4.10-iw.1", "1.4.9-iw.9"), Some(Ordering::Greater));
        assert_eq!(compare("1.4.9", "1.4.9-iw.1"), Some(Ordering::Less));
        assert_eq!(compare("1.4.9-iw.7", "1.4.9-iw.7"), Some(Ordering::Equal));
        assert_eq!(compare("1.4.9-rc.1", "1.4.9-iw.1"), None);
        assert_eq!(compare("abc", "1.4.9-iw.1"), None);
        assert_eq!(compare("1.4", "1.4.9-iw.1"), None);
    }

    #[test]
    fn a_newer_version_is_offered_with_its_package() {
        let o = offer(&manifest("1.4.9-iw.7", None), "1.4.9-iw.6", "windows-x86_64").unwrap().unwrap();
        assert_eq!(o, Offer { version: "1.4.9-iw.7".into(), required: false, package: package() });
    }

    #[test]
    fn the_same_or_an_older_version_is_not_offered() {
        assert_eq!(offer(&manifest("1.4.9-iw.7", None), "1.4.9-iw.7", "windows-x86_64"), Ok(None));
        assert_eq!(offer(&manifest("1.4.9-iw.6", None), "1.4.9-iw.7", "windows-x86_64"), Ok(None));
    }

    #[test]
    fn below_the_minimum_the_offer_is_required() {
        assert!(offer(&manifest("1.4.9-iw.7", Some("1.4.9-iw.7")), "1.4.9-iw.6", "windows-x86_64").unwrap().unwrap().required);
        assert!(!offer(&manifest("1.4.9-iw.7", Some("1.4.9-iw.6")), "1.4.9-iw.6", "windows-x86_64").unwrap().unwrap().required);
        assert!(!offer(&manifest("1.4.9-iw.7", Some("zawsze")), "1.4.9-iw.6", "windows-x86_64").unwrap().unwrap().required);
    }

    #[test]
    fn a_platform_without_a_package_gets_nothing() {
        assert_eq!(offer(&manifest("1.4.9-iw.7", None), "1.4.9-iw.6", "darwin-aarch64"), Err(UpdateError::NoPackage));
    }

    #[test]
    fn a_broken_manifest_is_an_error() {
        assert_eq!(offer("{nie json", "1.4.9-iw.6", "windows-x86_64"), Err(UpdateError::Manifest));
        assert_eq!(offer(r#"{"version":"zła","platforms":{}}"#, "1.4.9-iw.6", "windows-x86_64"), Err(UpdateError::Manifest));
    }

    #[test]
    fn packages_come_only_over_https_from_the_update_host() {
        for url in [
            "http://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-1.4.9-iw.7-x64.exe",
            "https://evil.example/IncorrectWebPomoc-1.4.9-iw.7-x64.exe",
            "https://license.incorrectus.com.evil.example/x.exe",
            "https://license.incorrectus.com@evil.example/x.exe",
            "https://license.incorrectus.com/desktop/pomoc/",
            "https://license.incorrectus.com/desktop/pomoc/run.bat",
        ] {
            let m = manifest("1.4.9-iw.7", None).replace(&package().url, url);
            assert_eq!(offer(&m, "1.4.9-iw.6", "windows-x86_64"), Err(UpdateError::Address), "{url}");
        }
    }

    #[test]
    fn file_names_are_plain() {
        assert_eq!(file_name("https://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-1.4.9-iw.7-x64.exe").as_deref(), Some("IncorrectWebPomoc-1.4.9-iw.7-x64.exe"));
        assert_eq!(file_name("https://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-1.4.9-iw.7-aarch64.dmg").as_deref(), Some("IncorrectWebPomoc-1.4.9-iw.7-aarch64.dmg"));
        assert_eq!(file_name("https://license.incorrectus.com/desktop/pomoc/x.exe?a=1"), None);
        assert_eq!(file_name("https://license.incorrectus.com/desktop/pomoc/..%2Fx.exe"), None);
        assert_eq!(file_name("https://license.incorrectus.com/desktop/pomoc/x.msi"), None);
        assert_eq!(file_name("nie adres"), None);
    }

    #[test]
    fn the_signed_file_passes() {
        assert_eq!(verify(TEST_FILE, &package(), &keys(TEST_KEY)), Ok(()));
    }

    #[test]
    fn a_changed_file_fails_the_checksum() {
        assert_eq!(verify(b"IncorrectWeb Pomoc test package!\n", &package(), &keys(TEST_KEY)), Err(UpdateError::Checksum));
    }

    #[test]
    fn a_file_with_the_right_checksum_but_no_valid_signature_fails() {
        let other = b"other file\n";
        let p = Package { sha256: sha(other), ..package() };
        assert_eq!(verify(other, &p, &keys(TEST_KEY)), Err(UpdateError::Signature), "the signature is of another file");
        assert_eq!(verify(TEST_FILE, &package(), &keys(PUBLIC_KEY)), Err(UpdateError::Signature), "signed with another key");
        let broken = Package { signature: "bm90IGEgc2lnbmF0dXJl".into(), ..package() };
        assert_eq!(verify(TEST_FILE, &broken, &keys(TEST_KEY)), Err(UpdateError::Signature));
    }

    #[test]
    fn the_release_key_is_the_desktop_app_key() {
        use hbb_common::base64::{engine::general_purpose::STANDARD, Engine};
        let text = String::from_utf8(STANDARD.decode(PUBLIC_KEY).unwrap()).unwrap();
        assert!(text.starts_with("untrusted comment: minisign public key: 4EEE84508E1BFFC5\n"), "{text}");
    }

    #[test]
    fn names_of_update_files_are_never_taken_for_an_organization_code() {
        for name in ["IncorrectWebPomoc-1.4.9-iw.7-x64.exe", "IncorrectWebPomoc-1.4.9-iw.10-x64.exe"] {
            assert_eq!(crate::iw::org::code_from_filename(name), None, "{name}");
        }
    }

    #[test]
    fn a_signature_made_for_another_file_name_fails() {
        let p = Package { signature: OTHER_NAME_SIG.into(), ..package() };
        assert_eq!(verify(TEST_FILE, &p, &keys(TEST_KEY)), Err(UpdateError::Signature));
    }

    #[test]
    fn an_older_signed_package_cannot_be_passed_off_as_the_new_version() {
        // Under the new name: the signature names the old file.
        let p = Package { sha256: sha(OLD_FILE), signature: OLD_SIG.into(), ..package() };
        assert_eq!(verify(OLD_FILE, &p, &keys(TEST_KEY)), Err(UpdateError::Signature));
        // Under its own name in a manifest of the new version: the name does not carry the offered version.
        let old_url = "https://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-1.4.9-iw.5-x64.exe";
        let m = manifest("1.4.9-iw.7", None).replace(&package().url, old_url);
        assert_eq!(offer(&m, "1.4.9-iw.6", "windows-x86_64"), Err(UpdateError::Address));
    }

    #[test]
    fn other_files_of_the_update_server_are_never_offered() {
        for url in [
            "https://license.incorrectus.com/desktop/IncorrectWeb-windows-setup.exe",
            "https://license.incorrectus.com/desktop/IncorrectWebPomoc-1.4.9-iw.7-x64.exe",
            "https://license.incorrectus.com/desktop/pomoc/sub/IncorrectWebPomoc-1.4.9-iw.7-x64.exe",
            "https://license.incorrectus.com/desktop/pomoc/IncorrectWeb_0.9.8_x64-setup.exe",
            "https://license.incorrectus.com/desktop/pomoc/IncorrectWebPomoc-1.4.9-iw.70-x64.exe",
        ] {
            let m = manifest("1.4.9-iw.7", None).replace(&package().url, url);
            assert_eq!(offer(&m, "1.4.9-iw.6", "windows-x86_64"), Err(UpdateError::Address), "{url}");
        }
    }

    #[test]
    fn the_verified_package_is_named_for_its_version() {
        assert_eq!(verify(TEST_FILE, &package(), &keys(TEST_KEY)), Ok(()));
        let renamed = Package { url: package().url.replace("iw.7-x64", "iw.7-x86"), ..package() };
        assert_eq!(verify(TEST_FILE, &renamed, &keys(TEST_KEY)), Err(UpdateError::Signature), "the address names another file than the signature");
    }
}
