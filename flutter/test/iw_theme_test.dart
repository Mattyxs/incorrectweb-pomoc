// IW: brand colors keep their contrast. common.dart is not imported (it loads the native library),
// the values in MyTheme are pinned by iw-tests/src/source_guard.rs.
import 'package:flutter/material.dart';
import 'package:flutter_hbb/iw/iw_brand.dart';
import 'package:flutter_test/flutter_test.dart';

double contrast(Color a, Color b) {
  final la = a.computeLuminance(), lb = b.computeLuminance();
  final hi = la > lb ? la : lb, lo = la > lb ? lb : la;
  return (hi + 0.05) / (lo + 0.05);
}

void main() {
  test('brand colors', () {
    expect(kIwViolet, const Color(0xFF431C77));
    expect(kIwOrange, const Color(0xFFEF7D1D));
  });

  test('violet on the light background and the lighter violet on the dark one are readable', () {
    expect(contrast(kIwViolet, Colors.white), greaterThanOrEqualTo(4.5));
    expect(contrast(kIwVioletOnDark, kIwDarkBackground), greaterThanOrEqualTo(4.5));
  });

  test('orange on the dark background is readable', () {
    expect(contrast(kIwOrange, kIwDarkBackground), greaterThanOrEqualTo(4.5));
  });
}
