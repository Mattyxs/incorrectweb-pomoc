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

#[test]
fn no_old_update_path_of_rustdesk_in_the_ui_bridge() {
    let ffi = repo_file("src/flutter_ffi.rs");
    for key in ["\"download-new-version\"", "\"update-me\"", "\"extract-update-dmg\""] {
        assert!(!ffi.contains(key), "{key}: the UI must not download or run update files");
    }
}

#[test]
fn the_installer_command_line_is_quoted() {
    let updater = repo_file("src/updater.rs");
    assert!(!updater.contains("format!(\"{} --update\""), "unquoted path in the command line");
    assert!(updater.contains("update_file::install_command("));
    assert!(!repo_file("src/iw_app.rs").contains("ProgramData"), "the packages live in the installation folder");
}

fn repo_bytes(rel: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn tray_icons_are_made_from_the_current_app_icon() {
    use sha2::{Digest, Sha256};
    // res/make-tray-icons.py writes the tray icons and res/tray-icons.sha256 (shasum format);
    // a new app icon or mark without new tray icons fails here (the tray kept the icon of 19.10 until iw.8)
    let stamp = repo_file("res/tray-icons.sha256");
    let mut seen = Vec::new();
    for line in stamp.lines().filter(|l| !l.trim().is_empty()) {
        let (hash, path) = line.split_once("  ").unwrap_or_else(|| panic!("bad line: {line}"));
        let now = hex::encode(Sha256::digest(repo_bytes(path)));
        assert_eq!(hash, now, "{path} changed: run python3 res/make-tray-icons.py");
        seen.push(path.to_owned());
    }
    for f in [
        "res/icon.png",
        "res/gocorrect-mark.png",
        "res/tray-icon.ico",
        "res/mac-tray-dark-x2.png",
        "res/mac-tray-light-x2.png",
    ] {
        assert!(seen.iter().any(|s| s == f), "{f} missing in res/tray-icons.sha256");
    }
}

/// (width, height) of the PNG images in an ICO, in directory order; panics on anything that is not a PNG entry.
fn ico_png_sizes(ico: &[u8]) -> Vec<(u32, u32)> {
    assert_eq!(&ico[..4], &[0, 0, 1, 0], "an ICO");
    let n = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    (0..n)
        .map(|i| {
            let e = &ico[6 + 16 * i..6 + 16 * (i + 1)];
            let len = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
            let off = u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize;
            let png = &ico[off..off + len];
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "entry {i} is a PNG");
            let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
            let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
            let dir = |b: u8| if b == 0 { 256 } else { b as u32 };
            assert_eq!((dir(e[0]), dir(e[1])), (w, h), "directory and PNG of entry {i} agree");
            (w, h)
        })
        .collect()
}

#[test]
fn windows_tray_icon_has_one_image_per_small_icon_size() {
    // SM_CXSMICON at 100, 125, 150, 200, 250 and 300 % scaling: each size is drawn for itself
    // (GoCorrect mark on a white tile), Windows does not scale a bigger one down
    let sizes = ico_png_sizes(&repo_bytes("res/tray-icon.ico"));
    assert_eq!(sizes, [16, 20, 24, 32, 40, 48].map(|s| (s, s)).to_vec());
}

#[test]
fn macos_tray_glyph_is_square_and_retina_high() {
    for f in ["res/mac-tray-dark-x2.png", "res/mac-tray-light-x2.png"] {
        let png = repo_bytes(f);
        let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        // the GoCorrect mark is round: the old glyph "iW" was wider than high
        assert_eq!((w, h), (60, 60), "{f}");
        assert_eq!(png[25], 6, "{f}: RGBA (a template keeps only the alpha)");
    }
}

#[test]
fn app_icon_is_the_gocorrect_icon_everywhere() {
    use sha2::{Digest, Sha256};
    // the icon of the GoCorrect desktop app (apps/desktop/src-tauri/icons of incorrectus-platform), copied as is
    assert_eq!(repo_bytes("flutter/assets/icon.png"), repo_bytes("res/icon.png"), "flutter/assets/icon.png = res/icon.png");
    assert_eq!(
        repo_bytes("flutter/windows/runner/resources/app_icon.ico"),
        repo_bytes("res/icon.ico"),
        "app_icon.ico = res/icon.ico"
    );
    let ico = ico_png_sizes(&repo_bytes("res/icon.ico"));
    for s in [16, 24, 32, 48, 256] {
        assert!(ico.contains(&(s, s)), "res/icon.ico has {s} px");
    }
    // the old "iW" icons must not come back (sha256 of the iW res/icon.png, res/32x32.png and the iW icns)
    for (f, old) in [
        ("res/icon.png", "5619b309382cc927c4c8142786c2062afe9174198542c63e32e991f514fc646f"),
        ("res/32x32.png", "b6816d4f479240974bba684d6618cbb988b10e2d5950bdf9b003b9036bf2f8ac"),
    ] {
        assert_ne!(hex::encode(Sha256::digest(repo_bytes(f))), old, "{f} is still the iW icon");
    }
    // the SVG fallbacks show the mark on a white tile, not the violet tile with the "W" polyline
    for f in ["flutter/assets/icon.svg", "res/scalable.svg"] {
        let svg = repo_file(f);
        assert!(!svg.contains("<polyline") && !svg.contains("fill=\"#431c77\"/>"), "{f} is still the iW icon");
        assert!(svg.contains("#ef7d1d") && svg.contains("#1e0e33"), "{f} has the GoCorrect mark");
    }
}

#[test]
fn macos_tray_uses_the_template_glyph_not_the_app_icon() {
    let tray = repo_file("src/tray.rs");
    // a template image keeps only the shape: the full app icon from the assets would be a filled square
    assert!(tray.contains("let asset_icon: Option<image::DynamicImage> = None;"), "macOS must not take assets/icon.png");
    assert!(tray.contains("asset_icon\n"), "the asset icon goes through asset_icon");
}

#[test]
fn windows_tray_takes_the_entry_for_the_small_icon_size_not_the_asset_icon() {
    let tray = repo_file("src/tray.rs");
    // the 512 px assets/icon.png scaled down by the shell was blurry; the ICO has one image per size
    assert!(tray.contains("SM_CXSMICON"), "the size comes from the small icon metric");
    assert!(tray.contains("crate::iw::tray::ico_entry_for("), "the entry is picked by src/iw/tray.rs");
    assert!(tray.contains("#[cfg(target_os = \"linux\")]\nfn load_icon_from_asset()"), "only Linux takes the asset icon");
}

#[test]
fn tray_tooltip_comes_from_iw_tray() {
    let tray = repo_file("src/tray.rs");
    assert!(tray.contains("crate::iw::tray::tooltip("), "the tooltip text lives in src/iw/tray.rs");
    assert!(!tray.contains('\u{2014}'), "no long dash in the tray texts");
}

#[test]
fn connection_window_texts_are_polish() {
    let cm = repo_file("flutter/lib/desktop/pages/server_page.dart");
    // keys that do not exist in src/lang (upstream typos) stay English
    for key in ["translate(\"File Transfer\")", "translate(\"View Camera\")"] {
        assert!(!cm.contains(key), "{key}: not a key of src/lang/pl.rs");
    }
    assert!(cm.contains("translate(\"Transfer file\")") && cm.contains("translate(\"View camera\")"));
    assert!(!cm.contains("\"ON\" : \"OFF\""), "permission tooltips in Polish");
    let pl = repo_file("src/lang/pl.rs");
    assert!(pl.contains("(\"Accept and Elevate\", \"Akceptuj i podnieś uprawnienia\")"));
}

fn winres_section(cargo_toml: &str) -> String {
    repo_file(cargo_toml).split("[package.metadata.winres]").nth(1).expect("winres section").split("\n[").next().unwrap().to_owned()
}

const COPYRIGHT: &str = "Copyright © 2026 Incorrectus. Oparty na RustDesk (AGPL-3.0), Copyright © 2026 Purslane Tech Pte. Ltd.";

#[test]
fn windows_file_properties_show_gocorrect_pomoc() {
    // the released IncorrectWebPomoc-*.exe is the portable packer; the app and the Flutter runner carry the same names
    for toml in ["libs/portable/Cargo.toml", "Cargo.toml"] {
        let res = winres_section(toml);
        for line in ["ProductName = \"GoCorrect Pomoc\"", "FileDescription = \"GoCorrect Pomoc\"", "CompanyName = \"Incorrectus\""] {
            assert!(res.contains(line), "{toml}: {line}");
        }
        assert!(res.contains(&format!("LegalCopyright = \"{COPYRIGHT}\"")), "{toml}: copyright keeps the RustDesk origin");
    }
    let rc = repo_file("flutter/windows/runner/Runner.rc");
    for value in ["\"ProductName\", \"GoCorrect Pomoc\"", "\"FileDescription\", \"GoCorrect Pomoc\"", "\"CompanyName\", \"Incorrectus\""] {
        assert!(rc.contains(value), "Runner.rc: {value}");
    }
    assert!(rc.contains(&format!("\"LegalCopyright\", \"{COPYRIGHT}\"")), "Runner.rc copyright");
}

#[test]
fn portable_exe_carries_the_iw_version() {
    let build = repo_file("libs/portable/build.rs");
    assert!(build.contains("../../src/iw/mod.rs") && build.contains("IW_VERSION"), "version read from src/iw/mod.rs");
    assert!(build.contains("\"FileVersion\"") && build.contains("\"ProductVersion\""));
}

#[test]
fn windows_install_shows_gocorrect_pomoc() {
    let win = repo_file("src/platform/windows.rs");
    assert!(!win.contains("/v DisplayName /t REG_SZ /d \\\"{app_name}\\\""), "Programs and Features: product name, not the internal name");
    assert!(!win.contains("/v Publisher /t REG_SZ /d \\\"{app_name}\\\""), "publisher is Incorrectus");
    assert_eq!(win.matches("/v DisplayName /t REG_SZ /d \\\"{display_name}\\\"").count(), 2, "install and update write the name");
    assert_eq!(win.matches("/v Publisher /t REG_SZ /d \\\"{publisher}\\\"").count(), 2, "install and update write the publisher");
    assert!(!win.contains("DisplayName= \\\"{app_name} Service\\\""), "service shown with the product name");
    assert!(crate::iw::PUBLISHER == "Incorrectus");
}

#[test]
fn macos_bundle_shows_gocorrect_pomoc() {
    let plist = repo_file("flutter/macos/Runner/Info.plist");
    assert!(plist.contains("<key>CFBundleName</key>\n\t<string>GoCorrect Pomoc</string>"));
    assert!(plist.contains("<key>CFBundleDisplayName</key>\n\t<string>GoCorrect Pomoc</string>"));
    let xc = repo_file("flutter/macos/Runner/Configs/AppInfo.xcconfig");
    assert!(xc.contains("PRODUCT_NAME = IncorrectWebPomoc\n"), "bundle and executable names stay (updates, links)");
    assert!(xc.contains("PRODUCT_BUNDLE_IDENTIFIER = com.incorrectweb.pomoc\n"));
    assert!(xc.contains(&format!("PRODUCT_COPYRIGHT = {COPYRIGHT}\n")));
    let menu = repo_file("flutter/macos/Runner/Base.lproj/MainMenu.xib");
    assert!(!menu.contains("<menuItem title=\"APP_NAME\"") && !menu.contains(" APP_NAME\""), "menu items name the product");
    assert!(menu.contains("<menuItem title=\"Quit GoCorrect Pomoc\""));
}

#[test]
fn installation_and_update_write_the_full_version_as_display_version() {
    // GoCorrect reads DisplayVersion of HKLM\...\Uninstall\IncorrectWebPomoc to know the installed 1.4.9-iw.N
    let win = repo_file("src/platform/windows.rs");
    assert_eq!(win.matches("/v DisplayVersion /t REG_SZ /d \\\"{iw_version}\\\"").count(), 2, "install_me and update_me");
    assert!(!win.contains("/v DisplayVersion /t REG_SZ /d \\\"{version}\\\""), "not the upstream 1.4.9");
    assert_eq!(win.matches("iw_version = crate::iw::IW_VERSION").count(), 2);
}

#[test]
fn macos_info_plist_carries_the_full_version() {
    let plist = repo_file("flutter/macos/Runner/Info.plist");
    let want = format!("<key>IWVersion</key>\n\t<string>{}</string>", crate::iw::IW_VERSION);
    assert!(plist.contains(&want), "Info.plist: IWVersion = {}", crate::iw::IW_VERSION);
}

#[test]
fn check_now_signal_reaches_the_updater_of_the_service() {
    let mediator = repo_file("src/rendezvous_mediator.rs");
    assert!(mediator.contains("crate::iw_app::start_check_now_listener();"), "started with the auto update of the service");
    let app = repo_file("src/iw_app.rs");
    assert!(app.contains("iw::check_now::EVENT_NAME") && app.contains("iw::check_now::SDDL"), "event and DACL from src/iw");
    assert!(app.contains("limiter.accept("), "signals go through the limiter");
    let up = repo_file("src/updater.rs");
    assert!(up.contains("UpdateMsg::CheckNow") && up.contains("pub fn check_now()"), "a message of its own");
}

#[test]
fn contract_for_gocorrect_is_documented_with_the_values_of_the_code() {
    let doc = repo_file("INCORRECTWEB.md");
    let part = doc.split("## Dla GoCorrect").nth(1).expect("section Dla GoCorrect");
    for v in [crate::iw::check_now::EVENT_NAME, crate::iw::check_now::SDDL, "IWVersion", "DisplayVersion", "5 min"] {
        assert!(part.contains(&v), "INCORRECTWEB.md, Dla GoCorrect: {v}");
    }
}
