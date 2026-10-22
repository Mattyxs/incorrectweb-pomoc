// IW: brand of the organization from its Panel (GET <panel>/api/instance), cached on disk and applied before
// runApp; Incorrectus look when there is none or the server does not answer.
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/iw/iw_brand.dart';
import 'package:flutter_hbb/iw/iw_brand_fetch.dart';
import 'package:flutter_hbb/iw/iw_tokens.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

const keys = [
  '--bg', '--bg-2', '--panel', '--panel-2', '--panel-soft', '--panel-hover', '--mine', '--ink', '--ink-2', '--muted',
  '--dim', '--line', '--line-2', '--violet', '--violet-soft', '--lilac', '--brand', '--orange', '--on-orange',
  '--orange-soft', '--orange-ink', '--focus', '--side', '--side-ink', '--side-text', '--side-muted', '--side-dim',
  '--side-accent', '--side-hover', '--soft', '--soft-hover', '--chip', '--chip-on', '--tab-ink', '--accent-ink',
  '--mine-ink', '--badge', '--badge-ink', '--shadow-card', '--shadow-pop', '--p-call', '--field', '--text-body',
  '--row-read', '--mine-btn',
];

Map<String, String> tokens(String v, [Map<String, String> over = const {}]) => {
      for (final k in keys) k: k.startsWith('--shadow') ? '0 10px 30px -18px rgba(1, 2, 3, 0.2)' : v,
      ...over,
    };

final png = Uint8List.fromList([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3]);
final png2 = Uint8List.fromList([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 9, 9, 9]);

String instance({String mode = 'custom', String hash = '0a1b2c3d4e5f', String? lightPng = 'logo-light-9f8e7d6c5b4a.png', String? darkPng = 'logo-dark-111111111111.png'}) =>
    jsonEncode({
      'company': {'name': 'Smart-Biznes', 'hasLogo': false},
      'branding': {
        'mode': mode,
        'hash': hash,
        'brandName': 'Smart-Biznes',
        'legacy': false,
        'inputs': {'primary': '#2f5b93', 'accent': '#c99700', 'neutral': 'tinted'},
        'tokens': {
          'light': tokens('#2f5b93', {'--bg': '#eef1f6', '--side-hover': 'rgba(255, 255, 255, 0.08)'}),
          'dark': tokens('#4a78b8', {'--field': '#0d1626'}),
        },
        'logo': {'light': 'logo-light-0a1b2c3d4e5f.svg', 'lightPng': lightPng, 'dark': null, 'darkPng': darkPng, 'mark': null},
        'slides': [],
      },
    });

MockClient server(Map<String, http.Response Function()> routes, [List<String>? asked]) => MockClient((req) async {
      asked?.add(req.url.toString());
      final r = routes[req.url.toString()];
      return r == null ? http.Response('nie ma', 404) : r();
    });

void main() {
  late Directory dir;
  setUp(() {
    dir = Directory.systemTemp.createTempSync('iw-brand-');
    IwBranding.apply(IwBranding.incorrectus);
  });
  tearDown(() {
    IwBranding.apply(IwBranding.incorrectus);
    dir.deleteSync(recursive: true);
  });

  test('instance with own brand: name, tokens and logo files; default brand gives none', () {
    final s = iwParseInstance(instance())!;
    expect(s.name, 'Smart-Biznes');
    expect(s.hash, '0a1b2c3d4e5f');
    expect(s.light['--brand'], '#2f5b93');
    expect(s.logoLight, 'logo-light-9f8e7d6c5b4a.png');
    expect(s.logoDark, 'logo-dark-111111111111.png');
    expect(iwParseInstance(instance(mode: 'default')), isNull);
    expect(iwParseInstance('<html>'), isNull);
    // Only brand file names: no paths, no other servers.
    expect(iwParseInstance(instance(lightPng: '../../etc/passwd'))!.logoLight, isNull);
  });

  test('tokens from server override palette (light and dark, rgba too)', () {
    final s = iwParseInstance(instance())!;
    IwBranding.apply(iwBrandingFrom(s));
    expect(IwBranding.current.custom, isTrue);
    expect(IwBranding.current.company, 'Smart-Biznes');
    expect(IwPalette.lightBranded.bg, const Color(0xFFEEF1F6));
    expect(IwPalette.lightBranded.brand, const Color(0xFF2F5B93));
    expect(IwPalette.lightBranded.sideHover, const Color(0x14FFFFFF));
    expect(IwPalette.darkBranded.field, const Color(0xFF0D1626));
    expect(IwPalette.darkBranded.panel, const Color(0xFF4A78B8));
    // Without server tokens: exactly the Incorrectus palette.
    IwBranding.apply(IwBranding.incorrectus);
    expect(IwPalette.lightBranded.bg, IwPalette.light.bg);
  });

  test('bad image ignored: SVG, too big, not an image', () {
    expect(iwIsImage(png), isTrue);
    expect(iwIsImage(Uint8List.fromList(utf8.encode('<svg onload="x">'))), isFalse);
    expect(iwIsImage(Uint8List(0)), isFalse);
    final big = Uint8List(1024 * 1024 + 1)..setRange(0, png.length, png);
    expect(iwIsImage(big), isFalse);
  });

  test('refresh downloads the logos once per hash and caches them; a bad logo is dropped', () async {
    final asked = <String>[];
    final client = server({
      'https://panel.firma.pl/api/instance': () => http.Response(instance(), 200),
      'https://panel.firma.pl/api/instance/branding/logo-light-9f8e7d6c5b4a.png': () => http.Response.bytes(png, 200),
      'https://panel.firma.pl/api/instance/branding/logo-dark-111111111111.png': () => http.Response('<svg/>', 200),
    }, asked);
    final b = await iwRefreshBrand('https://panel.firma.pl', dir, client: client);
    expect(b!.logoLightBytes, png);
    expect(b.logoDarkBytes, isNull);
    expect(asked, hasLength(3));
    // Same hash: nothing new to apply, no logo requests.
    asked.clear();
    expect(await iwRefreshBrand('https://panel.firma.pl', dir, client: client), isNull);
    expect(asked, ['https://panel.firma.pl/api/instance']);
  });

  test('branding cached applied before runApp: colours and logo from disk, without the network', () async {
    final client = server({
      'https://panel.firma.pl/api/instance': () => http.Response(instance(darkPng: null), 200),
      'https://panel.firma.pl/api/instance/branding/logo-light-9f8e7d6c5b4a.png': () => http.Response.bytes(png2, 200),
    });
    await iwRefreshBrand('https://panel.firma.pl', dir, client: client);
    IwBranding.apply(IwBranding.incorrectus);
    iwApplyCachedBrand(dir);
    expect(IwBranding.current.custom, isTrue);
    expect(IwBranding.current.logoLightBytes, png2);
    expect(IwPalette.lightBranded.brand, const Color(0xFF2F5B93));
  });

  test('default when server unreachable or without a cache; brand removed in the Panel = Incorrectus again', () async {
    iwApplyCachedBrand(dir);
    expect(identical(IwBranding.current, IwBranding.incorrectus), isTrue);
    final down = MockClient((_) async => throw const SocketException('brak sieci'));
    expect(await iwRefreshBrand('https://panel.firma.pl', dir, client: down), isNull);
    expect(IwBranding.current.custom, isFalse);

    final custom = server({
      'https://panel.firma.pl/api/instance': () => http.Response(instance(lightPng: null, darkPng: null), 200),
    });
    expect((await iwRefreshBrand('https://panel.firma.pl', dir, client: custom))!.custom, isTrue);
    final back = server({'https://panel.firma.pl/api/instance': () => http.Response(instance(mode: 'default'), 200)});
    final b = await iwRefreshBrand('https://panel.firma.pl', dir, client: back);
    expect(identical(b, IwBranding.incorrectus), isTrue);
    iwApplyCachedBrand(dir);
    expect(IwBranding.current.custom, isFalse);
  });

  test('broken cache file: Incorrectus look', () {
    File('${dir.path}/branding.json').writeAsStringSync('{nie json');
    iwApplyCachedBrand(dir);
    expect(identical(IwBranding.current, IwBranding.incorrectus), isTrue);
  });

  testWidgets('Created by Incorrectus only with the own brand', (t) async {
    await t.pumpWidget(const MaterialApp(home: Scaffold(body: IwCreatedBy())));
    expect(find.text('Created by Incorrectus'), findsNothing);
    IwBranding.apply(iwBrandingFrom(iwParseInstance(instance())!));
    await t.pumpWidget(const MaterialApp(home: Scaffold(body: IwCreatedBy(key: ValueKey(2)))));
    expect(find.text('Created by Incorrectus'), findsOneWidget);
  });
}
