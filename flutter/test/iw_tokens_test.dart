// IW: the palette matches packages/ui theme.css of incorrectus-platform and keeps its contrast.
import 'package:flutter/material.dart';
import 'package:flutter_hbb/iw/iw_tokens.dart';
import 'package:flutter_test/flutter_test.dart';

double contrast(Color a, Color b) {
  final la = a.computeLuminance(), lb = b.computeLuminance();
  final hi = la > lb ? la : lb, lo = la > lb ? lb : la;
  return (hi + 0.05) / (lo + 0.05);
}

/// A translucent color drawn over [base].
Color over(Color top, Color base) => Color.alphaBlend(top, base);

void main() {
  test('light values are the ones of theme.css', () {
    const p = IwPalette.light;
    expect(p.bg, const Color(0xFFEEEBF6));
    expect(p.ink, const Color(0xFF1B1630));
    expect(p.muted, const Color(0xFF6A6480));
    expect(p.violet, const Color(0xFF5B2FC4));
    expect(p.brand, const Color(0xFF431C77));
    expect(p.orange, const Color(0xFFEF7D1D));
    expect(p.onOrange, const Color(0xFF1A0F05));
    expect(p.side, const Color(0xFF2A1252));
    expect(p.focus, const Color(0xFFC45A00));
  });

  test('dark values are the ones of theme.css', () {
    const p = IwPalette.dark;
    expect(p.bg, const Color(0xFF0F0A1C));
    expect(p.panel, const Color(0xFF1A1230));
    expect(p.ink, const Color(0xFFECE9F7));
    expect(p.violet, const Color(0xFFB39BFF));
    expect(p.side, const Color(0xFF1D0D3F));
  });

  for (final p in [IwPalette.light, IwPalette.dark]) {
    final name = p.brightness.name;
    test('$name: text is readable on the page, cards and soft fills', () {
      for (final surface in [p.bg, p.panel, p.panelSoft]) {
        expect(contrast(p.ink, surface), greaterThanOrEqualTo(4.5));
        expect(contrast(p.muted, surface), greaterThanOrEqualTo(4.5));
        expect(contrast(p.dim, surface), greaterThanOrEqualTo(4.5));
      }
      expect(contrast(p.accentInk, p.violetSoft), greaterThanOrEqualTo(4.5));
      expect(contrast(p.orangeInk, p.orangeSoft), greaterThanOrEqualTo(4.5));
      expect(contrast(p.dangerInk, p.dangerSoft), greaterThanOrEqualTo(4.5));
      expect(contrast(p.okInk, p.okSoft), greaterThanOrEqualTo(4.5));
    });

    test('$name: the main action and the dark column are readable', () {
      expect(contrast(p.onOrange, p.orange), greaterThanOrEqualTo(4.5));
      expect(contrast(Colors.white, p.brand), greaterThanOrEqualTo(4.5));
      for (final t in [p.sideText, p.sideMuted, p.sideDim, p.sideAccent]) {
        expect(contrast(t, p.side), greaterThanOrEqualTo(4.5));
        expect(contrast(t, over(p.sideHover, p.side)), greaterThanOrEqualTo(4.5));
      }
    });

    test('$name: controls and the focus ring stand out (3:1)', () {
      expect(contrast(p.violet, p.panel), greaterThanOrEqualTo(3));
      expect(contrast(p.focus, p.bg), greaterThanOrEqualTo(3));
      expect(contrast(p.focus, p.panel), greaterThanOrEqualTo(3));
    });
  }
}
