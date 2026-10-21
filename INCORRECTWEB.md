# IncorrectWeb Pomoc

Fork of [RustDesk](https://github.com/rustdesk/rustdesk) 1.4.9 (AGPL-3.0). Version `1.4.9-iw.3`.
Source: https://github.com/Mattyxs/incorrectweb-pomoc. `libs/hbb_common` is the fork https://github.com/Mattyxs/incorrectweb-pomoc-hbb-common (branch `iw`).

Every change is marked `// IW:` (Rust, Dart). Pure logic is in `src/iw/` and is tested by `iw-tests` (`cargo test --manifest-path iw-tests/Cargo.toml`).

## Changed files
- `libs/hbb_common/src/config.rs`: app name `IncorrectWebPomoc`, ORG `com.incorrectweb`, no default server or key.
- `libs/hbb_common/protos/message.proto`: `LoginRequest.iw_intent = 1001`.
- `src/iw/*`, `iw-tests/*`: new (signing, organization code, link, consent policy, device API client, session log).
- `src/iw_app.rs`: new (glue: defaults, org setup, background worker, notifications).
- `src/lib.rs`, `src/server.rs`: module registration, defaults, worker start.
- `src/core_main.rs`: module registration, defaults; a link starts the program with the link only, links of other schemes are ignored.
- `src/common.rs`: link scheme `iwpomoc://`; no update check against api.rustdesk.com; no public STUN servers.
- `src/client.rs`: intent in the login request; no built-in server.
- `src/ipc.rs`, `src/ui_cm_interface.rs`: `iw` data for the consent window, session log, notification.
- `src/server/connection.rs`: consent policy (click always, verify, unattended only when allowed), session reports; the pointer shown to the client ("show my cursor") is brand orange with the consultant name.
- `src/flutter_ffi.rs`: `main_iw_*` functions.
- `src/tray.rs`: tooltip of unattended access.
- `src/lang.rs`: product name in texts.
- `src/lang/pl.rs`: "Show my cursor" reads "Wskazywanie (bez sterowania)".
- `src/iw/policy.rs`, `src/iw_app.rs`: display defaults (fit to window, client cursor shown), pointer color and caption.
- `flutter/lib/desktop/pages/iw_widgets.dart`: new (consent banner, organization screen, settings, link dialog, about).
- `flutter/lib/iw/iw_brand.dart`: new (brand constants, product name, label of the client cursor); no FFI import, used by the Flutter tests in `flutter/test/iw_*`.
- `flutter/assets/fonts/*`, `flutter/pubspec.yaml`: Onest 400/500/600/700 (SIL OFL, `OFL.txt` beside it), static instances cut from the variable font of google/fonts.
- `flutter/lib/desktop/pages/remote_page.dart`: the local cursor stays visible while the client moves the mouse; the client cursor has a label "Klient".
- `flutter/lib/desktop/widgets/tabbar_widget.dart`: product name instead of "RustDesk".
- `flutter/lib/common.dart` (also brand colors, Onest, soft violet highlight), `desktop/pages/{server_page,desktop_home_page,desktop_setting_page}.dart`, `models/server_model.dart`: hooks into the widgets.
- `flutter/macos/*`, `flutter/windows/runner/Runner.rc`, `build.py`: names, bundle id, URL scheme.
- `res/*`, `flutter/assets/icon.svg`, `flutter/windows/runner/resources/app_icon.ico`: temporary logo.
- `.github/workflows/incorrectweb-build.yml`: build (RustDesk workflows removed), `flutter-tests` job before the release, read-only token for build jobs, Flutter engine and nasm pinned by SHA-256.
- `src/lan.rs`: LAN discovery answers only after an explicit opt-in (default off).
- `src/iw/{notify,scheme,policy}.rs`: notification commands without text in the script, rules for writing the `iwpomoc://` registration, safe defaults.

## Notes
- Device key: the Ed25519 pair RustDesk keeps for the computer (shared by user and service processes).
- Tray: tooltip only, no warning dot.
- Organization code: the name found on the server is shown and must be confirmed before anything is applied. Plain http to localhost is accepted in test builds only.
