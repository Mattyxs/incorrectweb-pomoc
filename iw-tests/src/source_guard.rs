//! IW: guards on shipped RustDesk sources that the pure tests in src/iw cannot reach.
use std::path::PathBuf;

fn repo_file(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn no_version_check_against_api_rustdesk_com() {
    let common = repo_file("src/common.rs");
    assert!(
        !common.contains("version_check_request("),
        "do_check_software_update must not ask api.rustdesk.com (device fingerprint, RustDesk download offer)"
    );
}

#[test]
fn no_public_stun_servers() {
    let common = repo_file("src/common.rs");
    for host in ["stun.l.google.com", "stun.cloudflare.com", "stun.nextcloud.com"] {
        assert!(!common.contains(host), "{host} is still in src/common.rs");
    }
}
