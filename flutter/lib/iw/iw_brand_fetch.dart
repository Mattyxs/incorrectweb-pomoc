// IW: brand of the organization from its own server. The Panel address and the fallback colours come from the
// license server (org.rs: options iw-panel-url, iw-brand-primary, iw-brand-accent); the brand set in Panel →
// Wygląd i marka comes from GET <panel>/api/instance. It is cached in the config directory (branding.json +
// logo files), applied before runApp and refreshed in the background. Anything unexpected = the Incorrectus look.
// No FFI imports on purpose, so the logic is tested without the native library.
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/iw/iw_brand.dart';
import 'package:http/http.dart' as http;

/// Logos bigger than this are ignored (the Panel stores a 512 px PNG).
const int kIwMaxLogoBytes = 1024 * 1024;

final RegExp _assetName = RegExp(r'^(logo-light|logo-dark|mark)-[0-9a-f]{12}\.(png|jpg|webp)$');
final RegExp _hex = RegExp(r'^#[0-9a-f]{6}$');
final RegExp _rgba = RegExp(r'^rgba\((\d{1,3}),\s?(\d{1,3}),\s?(\d{1,3}),\s?(0|1|0?\.\d{1,3})\)$');
final RegExp _tokenKey = RegExp(r'^--[a-z0-9-]{1,30}$');

/// The organization's own brand as the Panel serves it (only what the app uses).
class IwServerBrand {
  final String name;
  final String? hash;
  final Map<String, String> light;
  final Map<String, String> dark;
  final String? logoLight;
  final String? logoDark;

  const IwServerBrand({required this.name, this.hash, required this.light, required this.dark, this.logoLight, this.logoDark});

  Map<String, dynamic> toJson() => {'name': name, 'hash': hash, 'light': light, 'dark': dark, 'logoLight': logoLight, 'logoDark': logoDark};

  static IwServerBrand? fromJson(Object? v) {
    if (v is! Map) return null;
    final name = v['name'];
    if (name is! String) return null;
    return IwServerBrand(
      name: name,
      hash: v['hash'] is String ? v['hash'] as String : null,
      light: _tokens(v['light']),
      dark: _tokens(v['dark']),
      logoLight: _asset(v['logoLight']),
      logoDark: _asset(v['logoDark']),
    );
  }
}

String? _asset(Object? v) => v is String && _assetName.hasMatch(v) ? v : null;

Map<String, String> _tokens(Object? v) {
  final out = <String, String>{};
  if (v is! Map) return out;
  v.forEach((k, x) {
    if (k is String && _tokenKey.hasMatch(k) && x is String && iwParseColor(x) != null) out[k] = x;
  });
  return out;
}

/// `#rrggbb` or `rgba(r, g, b, a)` (the forms of the brand tokens) → Color; anything else → null.
Color? iwParseColor(String v) {
  if (_hex.hasMatch(v)) return Color(0xFF000000 | int.parse(v.substring(1), radix: 16));
  final m = _rgba.firstMatch(v);
  if (m == null) return null;
  final rgb = [1, 2, 3].map((i) => int.parse(m.group(i)!)).toList();
  if (rgb.any((x) => x > 255)) return null;
  return Color.fromRGBO(rgb[0], rgb[1], rgb[2], double.parse(m.group(4)!));
}

/// The own brand from the body of GET /api/instance; null = Incorrectus look (default, old server, junk).
IwServerBrand? iwParseInstance(String body) {
  Object? v;
  try {
    v = jsonDecode(body);
  } catch (_) {
    return null;
  }
  if (v is! Map || v['branding'] is! Map) return null;
  final b = v['branding'] as Map;
  if (b['mode'] != 'custom') return null;
  final name = b['brandName'] is String && (b['brandName'] as String).trim().isNotEmpty ? (b['brandName'] as String).trim() : (v['company'] is Map ? (v['company'] as Map)['name'] : null);
  final tokens = b['tokens'] is Map ? b['tokens'] as Map : const {};
  final logo = b['logo'] is Map && b['legacy'] != true ? b['logo'] as Map : const {};
  return IwServerBrand(
    name: name is String && name.isNotEmpty ? name : 'IncorrectWeb',
    hash: b['hash'] is String ? b['hash'] as String : null,
    light: _tokens(tokens['light']),
    dark: _tokens(tokens['dark']),
    logoLight: _asset(logo['lightPng']) ?? _asset(logo['mark']),
    logoDark: _asset(logo['darkPng']),
  );
}

/// PNG, JPEG or WebP by content, at most [kIwMaxLogoBytes].
bool iwIsImage(Uint8List b) {
  if (b.isEmpty || b.length > kIwMaxLogoBytes) return false;
  bool starts(List<int> sig) => b.length >= sig.length && List.generate(sig.length, (i) => b[i] == sig[i]).every((x) => x);
  if (starts([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) return true;
  if (starts([0xff, 0xd8, 0xff])) return true;
  return b.length >= 12 && ascii.decode(b.sublist(0, 4), allowInvalid: true) == 'RIFF' && ascii.decode(b.sublist(8, 12), allowInvalid: true) == 'WEBP';
}

Map<String, Color>? _colours(Map<String, String> t) {
  final out = <String, Color>{};
  t.forEach((k, v) {
    final c = iwParseColor(v);
    if (c != null) out[k] = c;
  });
  return out.isEmpty ? null : out;
}

/// IwBranding of the organization: its name, colour tokens and logos; the rest from [base].
IwBranding iwBrandingFrom(IwServerBrand s, {Uint8List? light, Uint8List? dark, IwBranding base = IwBranding.incorrectus}) {
  final lt = _colours(s.light);
  final dt = _colours(s.dark);
  return IwBranding(
    name: base.name,
    company: s.name,
    primary: lt?['--brand'] ?? base.primary,
    accent: lt?['--orange'] ?? base.accent,
    logoLightAsset: base.logoLightAsset,
    logoDarkAsset: base.logoDarkAsset,
    appIconAsset: base.appIconAsset,
    logoLightBytes: light,
    logoDarkBytes: dark,
    custom: true,
    lightTokens: lt,
    darkTokens: dt,
  );
}

/// The cache in the config directory: branding.json and the logo files next to it.
class IwBrandCache {
  final Directory dir;
  IwBrandCache(this.dir);

  File get _file => File('${dir.path}/branding.json');

  IwServerBrand? read() {
    try {
      return IwServerBrand.fromJson(jsonDecode(_file.readAsStringSync()));
    } catch (_) {
      return null;
    }
  }

  Uint8List? logo(String? name) {
    if (name == null) return null;
    try {
      final b = File('${dir.path}/brand-$name').readAsBytesSync();
      return iwIsImage(b) ? b : null;
    } catch (_) {
      return null;
    }
  }

  /// The cached brand; null = none (Incorrectus look).
  IwBranding? load() {
    final s = read();
    return s == null ? null : iwBrandingFrom(s, light: logo(s.logoLight), dark: logo(s.logoDark));
  }

  void save(IwServerBrand s, Uint8List? light, Uint8List? dark) {
    dir.createSync(recursive: true);
    for (final e in [MapEntry(s.logoLight, light), MapEntry(s.logoDark, dark)]) {
      if (e.key != null && e.value != null) File('${dir.path}/brand-${e.key}').writeAsBytesSync(e.value!);
    }
    final tmp = File('${_file.path}.tmp')..writeAsStringSync(jsonEncode(s.toJson()));
    tmp.renameSync(_file.path);
    _prune({s.logoLight, s.logoDark});
  }

  void clear() {
    try {
      _file.deleteSync();
    } catch (_) {}
    _prune(const {});
  }

  void _prune(Set<String?> keep) {
    try {
      for (final f in dir.listSync().whereType<File>()) {
        final n = f.uri.pathSegments.last;
        if (n.startsWith('brand-') && !keep.contains(n.substring(6))) f.deleteSync();
      }
    } catch (_) {}
  }
}

/// Applies the cached brand (or the Incorrectus look); call it before runApp.
void iwApplyCachedBrand(Directory dir) {
  IwBranding.apply(IwBrandCache(dir).load() ?? IwBranding.incorrectus);
}

Future<Uint8List?> _image(http.Client c, Uri url) async {
  try {
    final r = await c.get(url).timeout(const Duration(seconds: 8));
    if (r.statusCode != 200) return null;
    return iwIsImage(r.bodyBytes) ? r.bodyBytes : null;
  } catch (_) {
    return null;
  }
}

/// Asks the Panel for the brand. Returns the new IwBranding when it changed (IwBranding.incorrectus when the brand
/// was removed), null when nothing changed or the Panel did not answer (the cache stays). Never throws.
Future<IwBranding?> iwRefreshBrand(String panelUrl, Directory dir, {http.Client? client}) async {
  final base = Uri.tryParse(panelUrl.endsWith('/') ? panelUrl : '$panelUrl/');
  if (base == null || base.scheme != 'https' || base.host.isEmpty) return null;
  final c = client ?? http.Client();
  final cache = IwBrandCache(dir);
  try {
    final r = await c.get(base.resolve('api/instance')).timeout(const Duration(seconds: 8));
    if (r.statusCode != 200) return null;
    final s = iwParseInstance(utf8.decode(r.bodyBytes, allowMalformed: true));
    final old = cache.read();
    if (s == null) {
      if (old == null) return null;
      cache.clear();
      return IwBranding.incorrectus;
    }
    if (old != null && old.hash != null && old.hash == s.hash) return null;
    final light = s.logoLight == null ? null : await _image(c, base.resolve('api/instance/branding/${s.logoLight}'));
    final dark = s.logoDark == null ? null : await _image(c, base.resolve('api/instance/branding/${s.logoDark}'));
    final saved = IwServerBrand(name: s.name, hash: s.hash, light: s.light, dark: s.dark, logoLight: light == null ? null : s.logoLight, logoDark: dark == null ? null : s.logoDark);
    cache.save(saved, light, dark);
    return iwBrandingFrom(saved, light: light, dark: dark);
  } catch (_) {
    return null;
  } finally {
    if (client == null) c.close();
  }
}
