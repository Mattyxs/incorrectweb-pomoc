# IncorrectWeb Pomoc

Fork of [RustDesk](https://github.com/rustdesk/rustdesk) 1.4.9 (AGPL-3.0). Version `1.4.9-iw.7`.
Source: https://github.com/Mattyxs/incorrectweb-pomoc. `libs/hbb_common` is the fork https://github.com/Mattyxs/incorrectweb-pomoc-hbb-common (branch `iw`).

Every change is marked `// IW:` (Rust, Dart). Pure logic is in `src/iw/` and is tested by `iw-tests` (`cargo test --manifest-path iw-tests/Cargo.toml`).

## Changed files
- `libs/hbb_common/src/config.rs`: app name `IncorrectWebPomoc`, ORG `com.incorrectweb`, no default server or key.
- `libs/hbb_common/protos/message.proto`: `LoginRequest.iw_intent = 1001`.
- `src/iw/*`, `iw-tests/*`: new (signing, organization code, link, consent policy, device API client, session log).
- `src/iw_app.rs`: new (glue: defaults, org setup, background worker, notifications).
- `src/lib.rs`, `src/server.rs`: module registration, defaults, worker start.
- `src/core_main.rs`: module registration, defaults; a link starts the program with the link only, links of other schemes are ignored.
- `src/common.rs`: link scheme `iwpomoc://`; no update check against api.rustdesk.com (updates come from the IncorrectWeb manifest instead); no public STUN servers.
- `src/iw/update.rs`, `src/iw_app.rs` (`check_update`), `src/updater.rs`: updates. The manifest `https://license.incorrectus.com/desktop/pomoc.json` names the newest version and one package per platform (`windows-x86_64`, `darwin-aarch64`, `darwin-x86_64`) with its SHA-256 and a minisign signature made with the key of IncorrectWeb releases (the key of the IncorrectWeb desktop app's updates); a package runs only when both match, the signature names its file (`file:<name>` in the trusted comment), the name carries the offered version (`IncorrectWebPomoc-<version>-…`) and it comes over https from license.incorrectus.com/desktop/pomoc/. Manifest and packages use their own client (rustls, no redirects, not RustDesk's client). On Windows the package is written to `<installation folder>\updates` (Program Files: administrators and SYSTEM only; no symbolic link or junction on the way; `src/iw/update_file.rs`), opened read-only without sharing writes or deletion, checked again through that handle and kept open until the installer started; the installer path is quoted on the command line. The UI no longer downloads or runs update files (`download-new-version`, `update-me`, `extract-update-dmg` removed from `src/flutter_ffi.rs`). The installed program (Windows service) checks every hour and installs a newer version itself when no session is on (`--update`, as RustDesk does); `allow-auto-update` cannot be turned off. macOS: no automatic installation yet (the manifest has no Mac packages until it is ready).
- `src/client.rs`: intent in the login request; no built-in server.
- `src/ipc.rs`, `src/ui_cm_interface.rs`: `iw` data for the consent window, session log, notification.
- `src/server/connection.rs`: consent policy (click always, verify, unattended only when allowed), session reports; the pointer shown to the client ("show my cursor") is brand orange with the consultant name.
- `src/flutter_ffi.rs`: `main_iw_*` functions.
- `src/tray.rs`, `src/iw/tray.rs`: tooltip text (product name with its space, Polish plural of sessions, a line when unattended access is on); on macOS the menu bar always takes the template glyph, never the full app icon of the assets (as a template it would be a filled square).
- `src/lang.rs`: product name in texts.
- `src/lang/pl.rs`: "Show my cursor" reads "Wskazywanie (bez sterowania)"; "Akceptuj i podnieś uprawnienia".
- `flutter/lib/desktop/pages/server_page.dart` (connection window of the client): keys `Transfer file` and `View camera` (the upstream `File Transfer` and `View Camera` are not in `src/lang`, so they stayed English), permission tooltips "włączone"/"wyłączone".
- `src/iw/policy.rs`, `src/iw_app.rs`: display defaults (fit to window, client cursor shown), pointer color and caption.
- `flutter/lib/desktop/pages/iw_widgets.dart`: new (consent banner, organization screen, settings, link dialog, about).
- `flutter/lib/iw/iw_brand.dart`: new (brand constants, product name, label of the client cursor); no FFI import, used by the Flutter tests in `flutter/test/iw_*`.
- `flutter/assets/fonts/*`, `flutter/pubspec.yaml`: Onest 400/500/600/700 (SIL OFL, `OFL.txt` beside it), static instances cut from the variable font of google/fonts.
- `flutter/lib/desktop/pages/remote_page.dart`: the local cursor stays visible while the client moves the mouse; the client cursor has a label "Klient".
- `flutter/lib/desktop/widgets/tabbar_widget.dart`: product name instead of "RustDesk".
- `flutter/lib/common.dart` (also brand colors, Onest, soft violet highlight), `desktop/pages/{server_page,desktop_home_page,desktop_setting_page}.dart`, `models/server_model.dart`: hooks into the widgets.
- `flutter/macos/*`, `flutter/windows/runner/Runner.rc`, `build.py`: names, bundle id, URL scheme.
- `res/*`, `flutter/macos/Runner/AppIcon.icns`, `flutter/windows/runner/resources/app_icon.ico`, `flutter/assets/icon.png`: app icon of the IncorrectWeb apps (apps/desktop of incorrectus-platform); `flutter/assets/iw_logo.png`: Incorrectus logo (packages/ui). Both PNGs are force-added (`*png` is ignored). Tray icons `res/tray-icon.ico` (the 32 px app icon) and `res/mac-tray-{dark,light}-x2.png` (template glyph "iW" with the underline) are made from the app icon by `python3 res/make-tray-icons.py`, which also writes `res/tray-icons.sha256`; iw-tests fails when the app icon changes without them.
- `flutter/lib/iw/iw_brand.dart`: `IwBranding`, the one place for name, main and accent color, light/dark logo and app icon (Incorrectus defaults; `IwBranding.apply()` before runApp for the branding of the organization).
- `flutter/lib/iw/iw_tokens.dart`, `flutter/test/iw_tokens_test.dart`: tokens of the IncorrectWeb apps (light and dark, = theme.css of incorrectus-platform), contrast tests, logo tile and app icon widgets.
- Look of the IncorrectWeb apps: `common.dart` (both themes from one builder on the tokens), `desktop_home_page.dart` + `iw_widgets.dart` (dark column with brand, number and password, status, settings; organization screen, banners, consent card, "O programie" with license and origin), `connection_page.dart`, `peer_tab_page.dart`, `peer_card.dart`, `peer_tab_model.dart` (line icons), `desktop_setting_page.dart` (navigation and cards), `server_page.dart` (consent window), `remote_toolbar.dart` (dark violet toolbar), `tabbar_widget.dart`, `desktop_tab_page.dart`.
- No links to rustdesk.com in the desktop UI (`common.dart` loadPowered, help cards, `install_page.dart` license link, `connection_page.dart`); guarded in `iw-tests/src/source_guard.rs`.
- `.github/workflows/incorrectweb-build.yml`: build (RustDesk workflows removed), `flutter-tests` job before the release, read-only token for build jobs, Flutter engine and nasm pinned by SHA-256.
- `src/lan.rs`: LAN discovery answers only after an explicit opt-in (default off).
- `src/iw/{notify,scheme,policy}.rs`: notification commands without text in the script, rules for writing the `iwpomoc://` registration, safe defaults.

## Notes
- Device key: the Ed25519 pair RustDesk keeps for the computer (shared by user and service processes).
- `state` has its own backoff (1 s, 2 s, 4 s ... 300 s, `api::StateSync`); 401 on `state` means the registration was reset: hello again at once.
- Key mismatch (409 on hello): the banner shows the key fingerprint (`api::key_fingerprint`, SHA-256 of the base64 public key, first 16 hex in groups of 4), the same form Connect shows the administrator next to the waiting key.
- Tray: tooltip only, no warning dot.
- Organization code: the name found on the server is shown and must be confirmed before anything is applied. Plain http to localhost is accepted in test builds only.
