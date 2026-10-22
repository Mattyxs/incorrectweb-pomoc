// IW: design tokens of the IncorrectWeb apps (light and dark), 1:1 with
// incorrectus-platform/packages/ui/src/styles/theme.css. No FFI import on purpose,
// so the Flutter tests can check the values and their contrast.
import 'package:flutter/material.dart';
import 'package:flutter_hbb/iw/iw_brand.dart';
import 'package:url_launcher/url_launcher.dart';

class IwPalette {
  final Brightness brightness;
  final Color bg, bg2, panel, panelSoft, panelHover;
  final Color ink, ink2, muted, dim, line, line2;
  final Color violet, violetSoft, brand, accentInk;
  final Color orange, onOrange, orangeSoft, orangeInk, orangeHover;
  final Color danger, dangerSoft, dangerInk;
  final Color ok, okSoft, okInk;
  final Color focus, field, soft;
  final Color side, sideText, sideMuted, sideDim, sideAccent, sideHover, sideLine;
  final Color shadow;

  const IwPalette({
    required this.brightness,
    required this.bg,
    required this.bg2,
    required this.panel,
    required this.panelSoft,
    required this.panelHover,
    required this.ink,
    required this.ink2,
    required this.muted,
    required this.dim,
    required this.line,
    required this.line2,
    required this.violet,
    required this.violetSoft,
    required this.brand,
    required this.accentInk,
    required this.orange,
    required this.onOrange,
    required this.orangeSoft,
    required this.orangeInk,
    required this.orangeHover,
    required this.danger,
    required this.dangerSoft,
    required this.dangerInk,
    required this.ok,
    required this.okSoft,
    required this.okInk,
    required this.focus,
    required this.field,
    required this.soft,
    required this.side,
    required this.sideText,
    required this.sideMuted,
    required this.sideDim,
    required this.sideAccent,
    required this.sideHover,
    required this.sideLine,
    required this.shadow,
  });

  static const light = IwPalette(
    brightness: Brightness.light,
    bg: Color(0xFFEEEBF6),
    bg2: Color(0xFFE6E1F2),
    panel: Color(0xFFFFFFFF),
    panelSoft: Color(0xFFFAF9FD),
    panelHover: Color(0x0D431C77), // rgba(67, 28, 119, 0.05)
    ink: Color(0xFF1B1630),
    ink2: Color(0xFF3C3552),
    muted: Color(0xFF6A6480),
    dim: Color(0xFF67617D),
    line: Color(0xFFEBE7F5),
    line2: Color(0xFFDDD6EF),
    violet: Color(0xFF5B2FC4),
    violetSoft: Color(0xFFF1EBFF),
    brand: Color(0xFF431C77),
    accentInk: Color(0xFF431C77),
    orange: Color(0xFFEF7D1D),
    onOrange: Color(0xFF1A0F05),
    orangeSoft: Color(0xFFFFF1E6),
    orangeInk: Color(0xFF8A3C00),
    orangeHover: Color(0xFFF28A2E),
    danger: Color(0xFFB3261E),
    dangerSoft: Color(0xFFFDE7E6),
    dangerInk: Color(0xFF7D140D),
    ok: Color(0xFF2F8F6B),
    okSoft: Color(0xFFEAF7F0),
    okInk: Color(0xFF1F7A57),
    focus: Color(0xFFC45A00),
    field: Color(0xFFFFFFFF),
    soft: Color(0xFFF4F1FB),
    side: Color(0xFF2A1252),
    sideText: Color(0xFFE9E2FB),
    sideMuted: Color(0xFFD9CFF5),
    sideDim: Color(0xFFBFAEEE),
    sideAccent: Color(0xFFFFB070),
    sideHover: Color(0x14FFFFFF), // rgba(255, 255, 255, 0.08)
    sideLine: Color(0x1FFFFFFF), // rgba(255, 255, 255, 0.12)
    shadow: Color(0x472A1252), // shadow-card: rgba(42, 18, 82, 0.28)
  );

  static const dark = IwPalette(
    brightness: Brightness.dark,
    bg: Color(0xFF0F0A1C),
    bg2: Color(0xFF150E27),
    panel: Color(0xFF1A1230),
    panelSoft: Color(0xFF150F29),
    panelHover: Color(0x0DFFFFFF),
    ink: Color(0xFFECE9F7),
    ink2: Color(0xFFD2CCE6),
    muted: Color(0xFFAAA4C4),
    dim: Color(0xFF9791B5),
    line: Color(0x14FFFFFF), // rgba(255, 255, 255, 0.08)
    line2: Color(0x26FFFFFF), // rgba(255, 255, 255, 0.15)
    violet: Color(0xFFB39BFF),
    violetSoft: Color(0xFF2B1D55),
    brand: Color(0xFF5B2FC4),
    accentInk: Color(0xFFC9B8FF),
    orange: Color(0xFFEF7D1D),
    onOrange: Color(0xFF1A0F05),
    orangeSoft: Color(0xFF3A2413),
    orangeInk: Color(0xFFFFB070),
    orangeHover: Color(0xFFF28A2E),
    danger: Color(0xFFFF8A80),
    dangerSoft: Color(0xFF3A1517),
    dangerInk: Color(0xFFFFB4AB),
    ok: Color(0xFF4FD1A1),
    okSoft: Color(0xFF12301F),
    okInk: Color(0xFF4FD1A1),
    focus: Color(0xFFEF7D1D),
    field: Color(0xFF120C22),
    soft: Color(0xFF251B40),
    side: Color(0xFF1D0D3F),
    sideText: Color(0xFFE9E2FB),
    sideMuted: Color(0xFFD9CFF5),
    sideDim: Color(0xFFBFAEEE),
    sideAccent: Color(0xFFFFB070),
    sideHover: Color(0x14FFFFFF),
    sideLine: Color(0x1FFFFFFF),
    shadow: Color(0xB3000000), // shadow-card: rgba(0, 0, 0, 0.7)
  );

  /// The palette with the colors of the current branding (IwBranding.current). With the Incorrectus
  /// defaults it is exactly [light] / [dark]; another main color also tints the dark column.
  static IwPalette get lightBranded => _branded(light, IwBranding.current);
  static IwPalette get darkBranded => _branded(dark, IwBranding.current);

  static IwPalette of(BuildContext context) =>
      Theme.of(context).brightness == Brightness.dark ? darkBranded : lightBranded;

  static IwPalette _branded(IwPalette p, IwBranding b) {
    final t = p.brightness == Brightness.dark ? b.darkTokens : b.lightTokens;
    if (t != null) return _fromTokens(p, t);
    const d = IwBranding.incorrectus;
    if (b.primary == d.primary && b.accent == d.accent) return p;
    final isDark = p.brightness == Brightness.dark;
    final side = Color.lerp(b.primary, Colors.black, isDark ? 0.55 : 0.38)!;
    return IwPalette(
      brightness: p.brightness,
      bg: p.bg, bg2: p.bg2, panel: p.panel, panelSoft: p.panelSoft, panelHover: p.panelHover,
      ink: p.ink, ink2: p.ink2, muted: p.muted, dim: p.dim, line: p.line, line2: p.line2,
      violet: p.violet, violetSoft: p.violetSoft,
      brand: isDark ? Color.lerp(b.primary, Colors.white, 0.15)! : b.primary,
      accentInk: isDark ? p.accentInk : b.primary,
      orange: b.accent,
      onOrange: b.accent.computeLuminance() > 0.3 ? p.onOrange : Colors.white,
      orangeSoft: p.orangeSoft, orangeInk: p.orangeInk,
      orangeHover: Color.lerp(b.accent, Colors.white, 0.1)!,
      danger: p.danger, dangerSoft: p.dangerSoft, dangerInk: p.dangerInk,
      ok: p.ok, okSoft: p.okSoft, okInk: p.okInk,
      focus: p.focus, field: p.field, soft: p.soft,
      side: side, sideText: p.sideText, sideMuted: p.sideMuted, sideDim: p.sideDim,
      sideAccent: p.sideAccent, sideHover: p.sideHover, sideLine: p.sideLine,
      shadow: p.shadow,
    );
  }

  /// The palette from the brand's colour tokens (theme.css names; checked for contrast by the Panel).
  /// A token the server did not send keeps the Incorrectus value.
  static IwPalette _fromTokens(IwPalette p, Map<String, Color> t) {
    Color c(String k, Color d) => t[k] ?? d;
    final orange = c('--orange', p.orange);
    return IwPalette(
      brightness: p.brightness,
      bg: c('--bg', p.bg), bg2: c('--bg-2', p.bg2), panel: c('--panel', p.panel), panelSoft: c('--panel-soft', p.panelSoft),
      panelHover: c('--panel-hover', p.panelHover),
      ink: c('--ink', p.ink), ink2: c('--ink-2', p.ink2), muted: c('--muted', p.muted), dim: c('--dim', p.dim),
      line: c('--line', p.line), line2: c('--line-2', p.line2),
      violet: c('--violet', p.violet), violetSoft: c('--violet-soft', p.violetSoft), brand: c('--brand', p.brand),
      accentInk: c('--accent-ink', p.accentInk),
      orange: orange, onOrange: c('--on-orange', p.onOrange), orangeSoft: c('--orange-soft', p.orangeSoft),
      orangeInk: c('--orange-ink', p.orangeInk), orangeHover: Color.lerp(orange, Colors.white, 0.1)!,
      danger: p.danger, dangerSoft: p.dangerSoft, dangerInk: p.dangerInk,
      ok: p.ok, okSoft: p.okSoft, okInk: p.okInk,
      focus: c('--focus', p.focus), field: c('--field', p.field), soft: c('--soft', p.soft),
      side: c('--side', p.side), sideText: c('--side-text', p.sideText), sideMuted: c('--side-muted', p.sideMuted),
      sideDim: c('--side-dim', p.sideDim), sideAccent: c('--side-accent', p.sideAccent), sideHover: c('--side-hover', p.sideHover),
      sideLine: p.sideLine, shadow: p.shadow,
    );
  }

  /// `--shadow-card`: 0 10px 30px -18px.
  List<BoxShadow> get cardShadow => [
        BoxShadow(
            color: shadow,
            offset: const Offset(0, 10),
            blurRadius: 30,
            spreadRadius: -18)
      ];

  /// `--shadow-pop`: 0 20px 50px -18px.
  List<BoxShadow> get popShadow => [
        BoxShadow(
            color: shadow.withOpacity(brightness == Brightness.dark ? 0.75 : 0.35),
            offset: const Offset(0, 20),
            blurRadius: 50,
            spreadRadius: -18)
      ];
}

/// `--r-sm`, `--r`, `--r-lg`.
const double kIwRadiusSm = 10;
const double kIwRadius = 14;
const double kIwRadiusLg = 18;

/// `--ease`: cubic-bezier(0.2, 0.7, 0.2, 1).
const Cubic kIwEase = Cubic(0.2, 0.7, 0.2, 1);

/// Numbers, codes and times: the text face with tabular digits (no mono face, no dotted zero).
const List<FontFeature> kIwTabular = [FontFeature.tabularFigures()];

/// Logo of the branding on a white tile (`.side-logo` / `.il-logo` of packages/ui): readable on
/// the dark column and on the page in both themes.
class IwLogoTile extends StatelessWidget {
  final double size;
  const IwLogoTile({Key? key, this.size = 36}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      padding: EdgeInsets.all(size * 0.16),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(size * 0.3),
      ),
      child: Image(
        image: IwBranding.current.logo(Brightness.light),
        fit: BoxFit.contain,
        filterQuality: FilterQuality.medium,
        semanticLabel: IwBranding.current.company,
      ),
    );
  }
}

/// App icon of the branding (the same picture as in the Dock / taskbar).
class IwAppIcon extends StatelessWidget {
  final double size;
  const IwAppIcon({Key? key, this.size = 48}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Image.asset(IwBranding.current.appIconAsset,
        width: size, height: size, filterQuality: FilterQuality.medium);
  }
}

/// "Created by Incorrectus" (link to incorrectus.com), only with the organization's own brand: the brand
/// of the organization is shown, our mark stays (decision of 22.10, like the web apps and e-mails).
class IwCreatedBy extends StatelessWidget {
  const IwCreatedBy({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    if (!IwBranding.current.custom) return const SizedBox.shrink();
    final p = IwPalette.of(context);
    return InkWell(
      borderRadius: BorderRadius.circular(6),
      onTap: () => launchUrl(Uri.parse('https://incorrectus.com')),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Text('Created by Incorrectus',
            style: TextStyle(fontSize: 12.5, fontWeight: FontWeight.w600, color: p.muted)),
      ),
    );
  }
}
