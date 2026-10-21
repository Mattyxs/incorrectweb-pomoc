// IW: brand constants and small widgets of IncorrectWeb Pomoc. No FFI imports on purpose,
// so widget tests can load this file without the native library.
import 'package:flutter/material.dart';

const Color kIwViolet = Color(0xFF431C77);
const Color kIwOrange = Color(0xFFEF7D1D);

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
