// IW: branding of IncorrectWeb Pomoc in ONE place: name, main and accent color, logos, app icon.
// No FFI imports on purpose, so widget tests can load this file without the native library.
//
// Today the values are the Incorrectus defaults below. Next step: the branding set in the Panel of
// the client (main and accent color, light/dark logo, name) is fetched from the server of the
// organization and passed to IwBranding.apply() before the first frame (see the note at apply()).
import 'dart:typed_data';

import 'package:flutter/material.dart';

class IwBranding {
  /// Product name shown in texts and in the brand row (the executable stays `IncorrectWebPomoc`).
  final String name;

  /// Company behind the product, shown next to the logo and in "O programie".
  final String company;

  /// Main color (`--brand`): primary buttons, the logo tile, the dark column is derived from it.
  final Color primary;

  /// Accent (`--orange`): the one main action, the bar of the active item, counters.
  final Color accent;

  /// Logos for the light and the dark theme (asset in the package, or bytes from the server).
  final String logoLightAsset;
  final String logoDarkAsset;
  final Uint8List? logoLightBytes;
  final Uint8List? logoDarkBytes;

  /// App icon shown inside the windows (title bar, "O programie"); the Dock / taskbar icon is
  /// built into the bundle (flutter/macos/Runner/AppIcon.icns, windows/runner/resources/app_icon.ico).
  final String appIconAsset;

  const IwBranding({
    required this.name,
    required this.company,
    required this.primary,
    required this.accent,
    required this.logoLightAsset,
    required this.logoDarkAsset,
    required this.appIconAsset,
    this.logoLightBytes,
    this.logoDarkBytes,
  });

  /// Defaults: Incorrectus / IncorrectWeb (logo of packages/ui, icon of apps/desktop).
  static const IwBranding incorrectus = IwBranding(
    name: 'IncorrectWeb Pomoc',
    company: 'Incorrectus',
    primary: Color(0xFF431C77),
    accent: Color(0xFFEF7D1D),
    logoLightAsset: 'assets/iw_logo.png',
    logoDarkAsset: 'assets/iw_logo.png',
    appIconAsset: 'assets/icon.png',
  );

  static IwBranding _current = incorrectus;
  static IwBranding get current => _current;

  /// Sets the branding of the organization. Call it before runApp (the themes in MyTheme are built
  /// once from IwBranding.current), e.g. in main.dart after the organization settings are read:
  /// `IwBranding.apply(IwBranding.incorrectus.copyWith(name: ..., primary: ..., logoLightBytes: ...))`.
  static void apply(IwBranding branding) => _current = branding;

  IwBranding copyWith({
    String? name,
    String? company,
    Color? primary,
    Color? accent,
    String? logoLightAsset,
    String? logoDarkAsset,
    String? appIconAsset,
    Uint8List? logoLightBytes,
    Uint8List? logoDarkBytes,
  }) =>
      IwBranding(
        name: name ?? this.name,
        company: company ?? this.company,
        primary: primary ?? this.primary,
        accent: accent ?? this.accent,
        logoLightAsset: logoLightAsset ?? this.logoLightAsset,
        logoDarkAsset: logoDarkAsset ?? this.logoDarkAsset,
        appIconAsset: appIconAsset ?? this.appIconAsset,
        logoLightBytes: logoLightBytes ?? this.logoLightBytes,
        logoDarkBytes: logoDarkBytes ?? this.logoDarkBytes,
      );

  ImageProvider logo(Brightness brightness) {
    final dark = brightness == Brightness.dark;
    final bytes = dark ? (logoDarkBytes ?? logoLightBytes) : logoLightBytes;
    if (bytes != null) return MemoryImage(bytes);
    return AssetImage(dark ? logoDarkAsset : logoLightAsset);
  }
}

const Color kIwViolet = Color(0xFF431C77);
const Color kIwOrange = Color(0xFFEF7D1D);
/// Violet for text and controls on the dark theme (`--violet` of the dark theme.css).
const Color kIwVioletOnDark = Color(0xFFB39BFF);
/// Background of the dark theme (`--bg` of the dark theme.css).
const Color kIwDarkBackground = Color(0xFF0F0A1C);

/// Product name for texts (the executable and window title are `IncorrectWebPomoc`, see APP_NAME).
const String kIwProductName = 'IncorrectWeb Pomoc';

const String kIwClientCursorLabel = 'Klient';

/// Small pill next to the cursor of the client, drawn while the client holds the mouse.
class IwCursorLabel extends StatelessWidget {
  final String text;
  const IwCursorLabel({Key? key, this.text = kIwClientCursorLabel})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: kIwOrange,
        borderRadius: BorderRadius.circular(6),
        boxShadow: const [
          BoxShadow(color: Color(0x40000000), blurRadius: 4, offset: Offset(0, 1))
        ],
      ),
      child: Text(
        text,
        style: const TextStyle(
          color: Colors.white,
          fontSize: 11,
          fontWeight: FontWeight.w600,
          decoration: TextDecoration.none,
        ),
      ),
    );
  }
}
