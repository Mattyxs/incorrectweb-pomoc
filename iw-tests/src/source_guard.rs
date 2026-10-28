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

#[test]
fn no_rustdesk_title_in_tabbar() {
    let tab = repo_file("flutter/lib/desktop/widgets/tabbar_widget.dart");
    assert!(!tab.contains("\"RustDesk\""), "the tab bar must show the product name, not RustDesk");
}

#[test]
fn theme_uses_incorrectus_brand_and_onest() {
    let common = repo_file("flutter/lib/common.dart");
    assert!(common.contains("0xFF431C77"), "MyTheme.accent must be the brand violet");
    // both themes come from one builder on the IncorrectWeb tokens (flutter/lib/iw/iw_tokens.dart)
    assert!(common.contains("_iwTheme(IwPalette.lightBranded)"), "light theme from the tokens");
    assert!(common.contains("_iwTheme(IwPalette.darkBranded)"), "dark theme from the tokens");
    assert_eq!(common.matches("fontFamily: 'Onest', // IW: font in the package").count(), 1, "one builder, Onest");
    // a second `colorScheme:` argument in one ThemeData does not compile (it broke iw.3)
    assert_eq!(common.matches("      colorScheme:").count(), 1, "one colorScheme in the builder");
    let tokens = repo_file("flutter/lib/iw/iw_tokens.dart");
    for v in ["0xFFEEEBF6", "0xFF1B1630", "0xFF5B2FC4", "0xFF2A1252", "0xFF0F0A1C"] {
        assert!(tokens.contains(v), "{v} of theme.css missing in iw_tokens.dart");
    }
    let pubspec = repo_file("flutter/pubspec.yaml");
    assert!(pubspec.contains("family: Onest"));
    for f in ["Regular", "Medium", "SemiBold", "Bold"] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../flutter/assets/fonts/Onest-{f}.ttf"));
        assert!(p.exists(), "{}", p.display());
        assert!(pubspec.contains(&format!("Onest-{f}.ttf")), "{f} not in pubspec");
    }
    assert!(repo_file("flutter/assets/fonts/OFL.txt").contains("SIL OPEN FONT LICENSE"));
}

#[test]
fn branding_lives_in_one_place() {
    let brand = repo_file("flutter/lib/iw/iw_brand.dart");
    assert!(brand.contains("class IwBranding") && brand.contains("static void apply("), "IwBranding with apply()");
    for asset in ["iw_logo.png", "icon.png"] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../flutter/assets/{asset}"));
        assert!(p.exists(), "{}", p.display());
    }
    let tab = repo_file("flutter/lib/desktop/widgets/tabbar_widget.dart");
    assert!(tab.contains("IwBranding.current.name"), "tab bar takes the name from IwBranding");
}

#[test]
fn desktop_ui_has_no_links_to_rustdesk() {
    for f in [
        "flutter/lib/common.dart",
        "flutter/lib/desktop/pages/desktop_home_page.dart",
        "flutter/lib/desktop/pages/connection_page.dart",
        "flutter/lib/desktop/pages/install_page.dart",
        "flutter/lib/desktop/pages/desktop_setting_page.dart",
    ] {
        let src = repo_file(f);
        for (i, line) in src.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            assert!(
                !code.contains("rustdesk.com") && !code.contains("github.com/rustdesk/rustdesk/releases"),
                "{f}:{}: link to RustDesk in the UI: {line}",
                i + 1
            );
        }
    }
    let about = repo_file("flutter/lib/desktop/pages/iw_widgets.dart");
    assert!(about.contains("AGPL-3.0") && about.contains("RustDesk"), "O programie keeps the license and the origin");
}

#[test]
fn release_waits_for_flutter_tests() {
    let wf = repo_file(".github/workflows/incorrectweb-build.yml");
    assert!(wf.contains("flutter-tests:"), "workflow needs the flutter-tests job");
    assert!(wf.contains("flutter test test/iw_"), "the job must run the iw Flutter tests");
    let needs = wf.lines().find(|l| l.trim_start().starts_with("needs: [unit-tests")).expect("release needs");
    assert!(needs.contains("flutter-tests"), "release must wait for flutter-tests: {needs}");
}

#[test]
fn updates_come_from_the_incorrectweb_manifest() {
    let common = repo_file("src/common.rs");
    let check = common.split("pub fn do_check_software_update").nth(1).expect("do_check_software_update").split("\n}\n").next().unwrap();
    assert!(check.contains("iw_app::check_update()"), "do_check_software_update asks the IncorrectWeb manifest: {check}");
    let updater = repo_file("src/updater.rs");
    assert!(updater.contains("iw_app::update_offer()"), "the package comes from the offer of the manifest");
    assert!(!updater.contains("rustdesk-{}"), "no RustDesk file names");
    assert!(!updater.contains("replace(\"tag\", \"download\")"), "no GitHub release pages of RustDesk");
}

#[test]
fn nothing_runs_a_package_before_it_is_verified() {
    let updater = repo_file("src/updater.rs");
    let check = updater.split("fn check_update(").nth(1).expect("check_update").split("\n}\n").next().unwrap();
    let verify = check.find("iw::update::verify(").expect("check_update verifies the package");
    let run = check.find("update_new_version(").expect("check_update installs");
    assert!(verify < run, "the signature is checked before the package runs");
    assert!(!check.contains("update_msi"), "Pomoc is published as .exe only: never the MSI path");
}

fn body(text: &str, start: &str) -> String {
    text.split(start).nth(1).unwrap_or_else(|| panic!("{start}")).split("\n}\n").next().unwrap().to_owned()
}

#[test]
fn updates_have_their_own_http_client() {
    let app = repo_file("src/iw_app.rs");
    let client = body(&app, "pub fn update_client(");
    assert!(client.contains("use_rustls_tls()"), "rustls: {client}");
    assert!(client.contains("Policy::none()"), "no redirects: {client}");
    assert!(!client.contains("danger_accept"), "never accept an invalid certificate");
    assert!(!body(&app, "pub fn check_update(").contains("create_http_client_with_url"), "not the RustDesk client (its TLS fallback, its proxy)");
    let check = body(&repo_file("src/updater.rs"), "fn check_update(");
    assert!(!check.contains("create_http_client_with_url"), "the package neither");
    assert!(!check.contains("get_download_file_from_url"), "no file in the temporary folder");
}

#[test]
fn the_package_runs_from_the_handle_that_was_verified() {
    let check = body(&repo_file("src/updater.rs"), "fn check_update(");
    let read = check.find("update_file::read_all(").expect("the package is read back through the locked handle");
    let verify = check[read..].find("iw::update::verify(").map(|i| i + read).expect("and verified again");
    let run = check.find("update_new_version(").unwrap();
    assert!(check.contains("update_file::open_locked("));
    assert!(read < verify && verify < run, "read, verify, then run");
}
