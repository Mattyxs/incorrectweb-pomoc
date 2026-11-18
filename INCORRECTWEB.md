# GoCorrect Pomoc

Fork of [RustDesk](https://github.com/rustdesk/rustdesk) 1.4.9 (AGPL-3.0). Version `1.4.9-iw.10`.
Source: https://github.com/Mattyxs/incorrectweb-pomoc. `libs/hbb_common` is the fork https://github.com/Mattyxs/incorrectweb-pomoc-hbb-common (branch `iw`).

The product was formerly called IncorrectWeb Pomoc. Only texts shown to people use the name GoCorrect Pomoc; the executable, bundle, installer and update file names (`IncorrectWebPomoc*`), the link scheme and paths stay as they were.

Every change is marked `// IW:` (Rust, Dart). Pure logic is in `src/iw/` and is tested by `iw-tests` (`cargo test --manifest-path iw-tests/Cargo.toml`).

## Changed files
- `libs/hbb_common/src/config.rs`: app name `IncorrectWebPomoc`, ORG `com.incorrectweb`, no default server or key.
- `libs/hbb_common/protos/message.proto`: `LoginRequest.iw_intent = 1001`.
- `src/iw/*`, `iw-tests/*`: new (signing, organization code, link, consent policy, device API client, session log).
- `src/iw_app.rs`: new (glue: defaults, org setup, background worker, notifications).
- `src/lib.rs`, `src/server.rs`: module registration, defaults, worker start.
- `src/core_main.rs`: module registration, defaults; a link starts the program with the link only, links of other schemes are ignored.
- `src/common.rs`: link scheme `iwpomoc://`; no update check against api.rustdesk.com (updates come from the IncorrectWeb manifest instead); no public STUN servers.
- `src/iw/update.rs`, `src/iw_app.rs` (`check_update`), `src/updater.rs`: updates. The manifest `https://license.incorrectus.com/desktop/pomoc.json` names the newest version and one package per platform (`windows-x86_64`, `darwin-aarch64`, `darwin-x86_64`) with its SHA-256 and a minisign signature: `pomocSignature` made with the Pomoc key (`POMOC_KEY`, since 18.11.2026) or, during the transition, `signature` made with the key of IncorrectWeb releases (the key of the IncorrectWeb desktop app's updates; `ACCEPT_RELEASE_KEY`); a package runs only when both match, the signature names its file (`file:<name>` in the trusted comment), the name carries the offered version (`IncorrectWebPomoc-<version>-…`) and it comes over https from license.incorrectus.com/desktop/pomoc/. Manifest and packages use their own client (rustls, no redirects, not RustDesk's client). On Windows the package is written to `<installation folder>\updates` (Program Files: administrators and SYSTEM only; no symbolic link or junction on the way; `src/iw/update_file.rs`), opened read-only without sharing writes or deletion, checked again through that handle and kept open until the installer started; the installer path is quoted on the command line. The UI no longer downloads or runs update files (`download-new-version`, `update-me`, `extract-update-dmg` removed from `src/flutter_ffi.rs`). The installed program (Windows service) checks every hour and installs a newer version itself when no session is on (`--update`, as RustDesk does); `allow-auto-update` cannot be turned off. macOS: no automatic installation yet (the manifest has no Mac packages until it is ready).
- `src/client.rs`: intent in the login request; no built-in server.
- `src/ipc.rs`, `src/ui_cm_interface.rs`: `iw` data for the consent window, session log, notification.
- `src/server/connection.rs`: consent policy (click always, verify, unattended only when allowed), session reports; the pointer shown to the client ("show my cursor") is brand orange with the consultant name.
- `src/flutter_ffi.rs`: `main_iw_*` functions.
- `src/tray.rs`, `src/iw/tray.rs`: tooltip text (product name with its space, Polish plural of sessions, a line when unattended access is on); on macOS the menu bar always takes the template glyph, never the full app icon of the assets (as a template it would be a filled square); on Windows the image of `res/tray-icon.ico` for the small icon size, not the 512 px app icon of the assets (blurry when the shell scales it down).
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
- Names people see: file properties of the Windows programs (`[package.metadata.winres]` in `Cargo.toml` and `libs/portable/Cargo.toml`, the released `IncorrectWebPomoc-*.exe` is the portable packer; `Runner.rc`) say ProductName and FileDescription "GoCorrect Pomoc", CompanyName "Incorrectus", LegalCopyright "Copyright © 2026 Incorrectus. Oparty na RustDesk (AGPL-3.0), Copyright © 2026 Purslane Tech Pte. Ltd."; `libs/portable/build.rs` writes FileVersion and ProductVersion from `IW_VERSION`. Windows installation (`src/platform/windows.rs`): Programs and Features shows "GoCorrect Pomoc" by "Incorrectus" (also written by an update, so older installations get the new name), services.msc shows "GoCorrect Pomoc". macOS: `CFBundleName` and `CFBundleDisplayName` "GoCorrect Pomoc", the app menu (`MainMenu.xib`) names the product, copyright as on Windows. Unchanged on purpose: the bundle and executable `IncorrectWebPomoc`, the registry key, the service name, the firewall rule, the shortcut file names and the window title (on Windows the second start finds the running window by the title `IncorrectWebPomoc`). Guarded in `iw-tests/src/source_guard.rs`.
- `res/{icon.png,icon.ico,32x32.png,64x64.png,128x128.png,128x128@2x.png,mac-icon.png}`, `flutter/macos/Runner/AppIcon.icns`, `flutter/windows/runner/resources/app_icon.ico`, `flutter/assets/icon.png`: the GoCorrect app icon, copied as is from the GoCorrect desktop app (`apps/desktop/src-tauri/icons` of incorrectus-platform: mark on a white rounded tile; the macOS `.icns` with the faintly shaded tile, so macOS 26+ does not darken it); `flutter/assets/icon.svg` (fallback of `loadIcon`) and `res/scalable.svg` (Linux packages) draw the same from GoCorrect.svg. `flutter/assets/iw_logo.png`: Incorrectus logo (packages/ui), the default brand of the organization (`IwBranding`). PNGs are force-added (`*png` is ignored). Tray icons are made from the whole mark `res/gocorrect-mark.png` (GoCorrect.svg, viewBox 120 120 840 840, centred on the ring, the orange arc whole) by `python3 res/make-tray-icons.py`, which also writes `res/tray-icons.sha256`: `res/tray-icon.ico` has the mark on a white tile drawn separately at 16, 20, 24, 32, 40 and 48 px and Windows takes the image for `SM_CXSMICON` (`src/tray.rs`, `src/iw/tray.rs` `ico_entry_for`); `res/mac-tray-{dark,light}-x2.png` is the template glyph (silhouette of the mark, 60 x 60). iw-tests fails when the icon, the mark and the tray icons drift apart. The macOS disk image window (`res/installer`) names GoCorrect Pomoc.
- `flutter/lib/iw/iw_brand.dart`: `IwBranding`, the one place for name, main and accent color, light/dark logo and app icon (Incorrectus defaults; `IwBranding.apply()` before runApp for the branding of the organization).
- `flutter/lib/iw/iw_tokens.dart`, `flutter/test/iw_tokens_test.dart`: tokens of the IncorrectWeb apps (light and dark, = theme.css of incorrectus-platform), contrast tests, logo tile and app icon widgets.
- Look of the IncorrectWeb apps: `common.dart` (both themes from one builder on the tokens), `desktop_home_page.dart` + `iw_widgets.dart` (dark column with brand, number and password, status, settings; organization screen, banners, consent card, "O programie" with license and origin), `connection_page.dart`, `peer_tab_page.dart`, `peer_card.dart`, `peer_tab_model.dart` (line icons), `desktop_setting_page.dart` (navigation and cards), `server_page.dart` (consent window), `remote_toolbar.dart` (dark violet toolbar), `tabbar_widget.dart`, `desktop_tab_page.dart`.
- No links to rustdesk.com in the desktop UI (`common.dart` loadPowered, help cards, `install_page.dart` license link, `connection_page.dart`); guarded in `iw-tests/src/source_guard.rs`.
- `.github/workflows/incorrectweb-build.yml`: build (RustDesk workflows removed), `flutter-tests` job before the release, read-only token for build jobs, Flutter engine and nasm pinned by SHA-256.
- `src/lan.rs`: LAN discovery answers only after an explicit opt-in (default off).
- `src/platform/windows.rs`: installation and every update write `DisplayVersion` = `IW_VERSION` (1.4.9-iw.N) into the Uninstall key; `Version` and `VersionMajor/Minor/Build` stay those of RustDesk. `flutter/macos/Runner/Info.plist`: `IWVersion` = `IW_VERSION` (raised with it, checked by iw-tests).
- `src/iw/check_now.rs`, `src/iw_app.rs` (`start_check_now_listener`), `src/updater.rs` (`CheckNow`), `src/rendezvous_mediator.rs`: "check for an update now" for GoCorrect, see Dla GoCorrect.
- `src/iw/{notify,scheme,policy}.rs`: notification commands without text in the script, rules for writing the `iwpomoc://` registration, safe defaults.

## Dla GoCorrect
What another program of the computer (GoCorrect) may read and do without administrator rights.

**Installed version.** The full version `1.4.9-iw.N` (the same string as `IW_VERSION` and the release tag):
- Windows: registry value `DisplayVersion` (REG_SZ) of `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IncorrectWebPomoc` (64-bit view; readable by every user). Written by the installation and by every update (the update runs the code of the new version), so it changes when an update is done. Older installations: the key of an Inno Setup installation of RustDesk `{54E86BC2-6C85-41F3-A9EB-1A94AC9B1F93}_is1` is taken first by the program when it exists (`get_valid_subkey`); our installations do not have it. Versions up to 1.4.9-iw.9 wrote `1.4.9` there: a value without `-iw.` means "iw.9 or older".
- macOS: key `IWVersion` (string) of `IncorrectWebPomoc.app/Contents/Info.plist` (`defaults read /Applications/IncorrectWebPomoc.app/Contents/Info.plist IWVersion`). `CFBundleShortVersionString` stays `1.4.9` (Apple wants numbers only there). From 1.4.9-iw.10.

**Check for an update now (Windows, from 1.4.9-iw.10).** A named event, no data:
- Name `Global\GoCorrectPomocCheckUpdate`, auto-reset, created by the service process of Pomoc (`--server`, SYSTEM) when it starts, beside the automatic update check (installed program only).
- DACL (SDDL) `D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x2;;;AU)`: authenticated users may only set it (EVENT_MODIFY_STATE).
- Caller: `OpenEventW(EVENT_MODIFY_STATE, FALSE, L"Global\\GoCorrectPomocCheckUpdate")`, `SetEvent`, `CloseHandle`. Not found (ERROR_FILE_NOT_FOUND) = Pomoc not installed, older than iw.10 or its service is not running. No answer: read the version above again later.
- Limit: at most one accepted signal per 5 min (from the last accepted one); signals in between are dropped, not queued. An accepted signal starts the usual check at once (not held back by the 10-minute minimum of the hourly check): manifest `pomoc.json`, SHA-256 and signature, installation only when no session is on, otherwise a new try after 10 minutes.
- It works with nobody signed in to the window of Pomoc (the service runs it). Nothing the caller sends is read: no parameter, address or path.
- macOS: Pomoc does not install updates by itself there (no Mac package in the manifest), so there is no such signal; possible later: a LaunchDaemon of Pomoc watching a file or a `notify(3)` name, after Pomoc can install a Mac package itself.

## Notes
- Device key: the Ed25519 pair RustDesk keeps for the computer (shared by user and service processes).
- `state` has its own backoff (1 s, 2 s, 4 s ... 300 s, `api::StateSync`); 401 on `state` means the registration was reset: hello again at once.
- Key mismatch (409 on hello): the banner shows the key fingerprint (`api::key_fingerprint`, SHA-256 of the base64 public key, first 16 hex in groups of 4), the same form Connect shows the administrator next to the waiting key.
- Tray: tooltip only, no warning dot.
- Organization code: the name found on the server is shown and must be confirmed before anything is applied. Plain http to localhost is accepted in test builds only.
