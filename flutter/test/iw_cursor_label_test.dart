// IW: the label next to the client cursor.
import 'package:flutter/material.dart';
import 'package:flutter_hbb/iw/iw_brand.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('IwCursorLabel shows "Klient" on the brand orange', (tester) async {
    await tester.pumpWidget(const Directionality(
      textDirection: TextDirection.ltr,
      child: Center(child: IwCursorLabel()),
    ));
    expect(find.text('Klient'), findsOneWidget);
    final box = tester.widget<Container>(find.byType(Container)).decoration as BoxDecoration;
    expect(box.color, kIwOrange);
  });
}
