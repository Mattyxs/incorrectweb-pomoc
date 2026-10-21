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

#[test]
fn lan_discovery_listener_asks_the_iw_policy() {
    let lan = repo_file("src/lan.rs");
    assert!(lan.contains("// IW:"), "src/lan.rs: mark the change with // IW:");
    assert!(lan.contains("lan_discovery_enabled("), "src/lan.rs must use iw::policy::lan_discovery_enabled");
    assert!(
        !lan.contains("option2bool(\n                                \"enable-lan-discovery\""),
        "src/lan.rs still treats an unset option as enabled"
    );
    let app = repo_file("src/iw_app.rs");
    assert!(app.contains("DEFAULT_LAN_DISCOVERY"), "iw_app::init must set the default of enable-lan-discovery");
}

#[test]
fn notification_scripts_are_not_built_from_texts() {
    let app = repo_file("src/iw_app.rs");
    assert!(!app.contains("\"-Command\""), "powershell must get -EncodedCommand, not -Command <script with text>");
    assert!(!app.contains("CreateTextNode('{}')"), "no text formatted into the PowerShell script");
    assert!(app.contains("notify::powershell_toast("));
    assert!(app.contains("notify::osascript_args("));
}

#[test]
fn url_scheme_registration_asks_the_iw_scheme_policy() {
    let app = repo_file("src/iw_app.rs");
    assert!(app.contains("scheme::should_register("), "register_url_scheme must call iw::scheme::should_register");
}

#[test]
fn organization_is_applied_only_after_confirmation() {
    let app = repo_file("src/iw_app.rs");
    assert!(app.contains("PENDING_ORG"), "lookup must keep the answer until the user confirms");
    let ffi = repo_file("src/flutter_ffi.rs");
    assert!(ffi.contains("main_iw_lookup_org_code"));
    let ui = repo_file("flutter/lib/desktop/pages/iw_widgets.dart");
    assert!(ui.contains("mainIwLookupOrgCode"), "UI must look the organization up first");
    assert!(!ui.contains("addPostFrameCallback((_) => _apply())"), "a code from the file name must not be applied unasked");
}

#[test]
fn plain_http_is_a_test_build_only_option() {
    let org = repo_file("src/iw/org.rs");
    assert!(org.contains("valid_api_with(api, cfg!(test))"), "valid_api must allow http only under cfg!(test)");
}

#[test]
fn workflow_downloads_are_checked_against_sha256() {
    // sha256sum on Linux and Windows (Git Bash); macOS runners only have shasum
    let wf = repo_file(".github/workflows/incorrectweb-build.yml");
    let checks: Vec<&str> =
        wf.lines().filter(|l| l.contains("sha256sum -c") || l.contains("shasum -a 256 -c")).collect();
    assert!(checks.len() >= 2, "both the Flutter engine and nasm need a checked SHA-256, found {checks:?}");
    assert!(wf.contains("sha256sum -c -"), "the Flutter engine must be verified with sha256sum -c");
    assert!(!wf.contains("Invoke-WebRequest"), "no unchecked download of the Flutter engine");
    assert!(!wf.contains("wget https://www.nasm.us"), "no unchecked download of nasm");
    for line in checks {
        assert!(!line.contains("--ignore-missing") && !line.contains("|| true"), "{line}");
    }
    // the hashes are 64 hex digits
    for h in [
        "ec8cabf36ee4ff24c8d98de25b00e70781eb03876265aee84d0fe554a110036e",
        "0d29bcd8a5fc617333f4549c7c1f93d1866a4a0915c40359e0a8585bb1a5aa75",
    ] {
        assert!(wf.contains(h), "missing {h}");
    }
}

#[test]
fn iw_app_sets_display_defaults() {
    let app = repo_file("src/iw_app.rs");
    assert!(app.contains("DEFAULT_DISPLAY_SETTINGS"), "iw_app::init must fill DEFAULT_DISPLAY_SETTINGS");
    assert!(app.contains("DISPLAY_DEFAULTS"), "iw_app::init must use iw::policy::DISPLAY_DEFAULTS");
}

#[test]
fn remote_page_keeps_local_cursor_visible() {
    let page = repo_file("flutter/lib/desktop/pages/remote_page.dart");
    assert!(page.contains("IwCursorLabel"), "CursorPaint must draw the label of the client cursor");
    assert!(page.contains("IW: both cursors visible"), "the local cursor must stay visible while the client moves the mouse");
}

#[test]
fn whiteboard_cursor_uses_brand_orange() {
    let conn = repo_file("src/server/connection.rs");
    assert!(conn.contains("POINTER_ARGB") && conn.contains("iw_cursor_label"), "the cursor of the technician shown at the client must be #ef7d1d");
}
