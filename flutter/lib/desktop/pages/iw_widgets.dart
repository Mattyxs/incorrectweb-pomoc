// IW: widgets of IncorrectWeb Pomoc (consent banner, organization code, unattended access,
// link confirmation, history). All texts are Polish on purpose.
import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/iw/iw_brand.dart';
import 'package:flutter_hbb/iw/iw_tokens.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_hbb/models/server_model.dart';
import 'package:flutter_hbb/utils/multi_window_manager.dart';
import 'package:url_launcher/url_launcher.dart';

// IW: brand constants live in iw_brand.dart (no FFI import, so tests can use them)
export 'package:flutter_hbb/iw/iw_brand.dart';
export 'package:flutter_hbb/iw/iw_tokens.dart';

/// Status from the Rust side: configured, org, unattended, keyMismatch, version, name, source.
Map<String, dynamic> iwStatus() {
  try {
    return Map<String, dynamic>.from(jsonDecode(bind.mainIwStatus()));
  } catch (_) {
    return {};
  }
}

/// Counts down from [seconds] to 0 and rebuilds [builder] every second.
class IwCountdown extends StatefulWidget {
  final int seconds;
  final Widget Function(BuildContext context, int left) builder;
  const IwCountdown({Key? key, required this.seconds, required this.builder})
      : super(key: key);

  @override
  State<IwCountdown> createState() => _IwCountdownState();
}

class _IwCountdownState extends State<IwCountdown> {
  late int _left = widget.seconds;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    if (_left > 0) {
      _timer = Timer.periodic(const Duration(seconds: 1), (t) {
        if (!mounted) return;
        setState(() => _left = _left > 0 ? _left - 1 : 0);
        if (_left == 0) t.cancel();
      });
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, _left);
}

/// Who is connecting: green for a person confirmed by the organization, red for an unknown one,
/// orange for a session that needed no click.
class IwConsentBanner extends StatelessWidget {
  final Client client;
  const IwConsentBanner({Key? key, required this.client}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    if (client.disconnected) return const SizedBox.shrink();
    final who = [
      if (client.iwName.isNotEmpty) client.iwName,
      if (client.iwCompany.isNotEmpty) client.iwCompany,
      if (client.iwTicket != null) 'zgłoszenie #${client.iwTicket}',
    ].join(' · ');
    final p = IwPalette.of(context);
    // IW: tone of the card: ok (confirmed by the organization), danger (unknown), orange (no click)
    Color bar, bg, ink;
    IconData icon;
    String title;
    String body;
    if (!client.authorized) {
      if (client.iwVerified) {
        bar = p.ok;
        bg = p.okSoft;
        ink = p.okInk;
        icon = Icons.verified_user_outlined;
        title = who;
        body = 'chce połączyć się z tym komputerem. Pozwolić?';
      } else {
        bar = p.danger;
        bg = p.dangerSoft;
        ink = p.dangerInk;
        icon = Icons.gpp_maybe_outlined;
        final org = client.iwOrg.isNotEmpty ? client.iwOrg : 'organizację';
        title = 'Nieznana osoba (nie potwierdzona przez $org)';
        body =
            'Nie akceptuj, jeśli nie rozmawiasz właśnie z konsultantem. Numer: ${client.peerId}';
      }
    } else if (client.iwUnattended) {
      bar = p.orange;
      bg = p.orangeSoft;
      ink = p.orangeInk;
      icon = Icons.lock_open_rounded;
      title = 'Dostęp bez akceptacji';
      body = 'Konsultant $who połączył się z tym komputerem.';
    } else if (client.iwVerified) {
      bar = p.ok;
      bg = p.okSoft;
      ink = p.okInk;
      icon = Icons.verified_user_outlined;
      title = who;
      body = 'połączenie potwierdzone przez organizację.';
    } else {
      return const SizedBox.shrink();
    }
    return Container(
      margin: const EdgeInsets.fromLTRB(5, 2, 5, 6),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(kIwRadius),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
              width: 4,
              height: 22,
              margin: const EdgeInsets.only(top: 12),
              decoration: BoxDecoration(
                  color: bar,
                  borderRadius: const BorderRadius.horizontal(
                      right: Radius.circular(4)))),
          const SizedBox(width: 10),
          Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Icon(icon, size: 20, color: ink)),
          const SizedBox(width: 8),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 10),
              child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title,
                    style: TextStyle(
                        color: ink, fontWeight: FontWeight.w700, fontSize: 13.5)),
                const SizedBox(height: 2),
                Text(body,
                    style: TextStyle(fontSize: 12.5, height: 1.4, color: p.ink)),
              ],
            )),
          ),
          const SizedBox(width: 10),
        ],
      ),
    );
  }
}

/// "Wpisz kod organizacji": shown instead of the home page until the program is configured.
class IwOrgPage extends StatefulWidget {
  final VoidCallback onConfigured;
  const IwOrgPage({Key? key, required this.onConfigured}) : super(key: key);

  @override
  State<IwOrgPage> createState() => _IwOrgPageState();
}

class _IwOrgPageState extends State<IwOrgPage> {
  final _controller = TextEditingController();
  String _message = '';
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    // The code in the file name (IncorrectWebPomoc-ABCD-EFGH.exe) is only looked up; the organization
    // is applied after the user confirms its name.
    final fromExe = bind.mainIwOrgCodeFromExe();
    if (fromExe.isNotEmpty) {
      _controller.text = fromExe;
      WidgetsBinding.instance.addPostFrameCallback((_) => _lookupAndConfirm());
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Map<String, dynamic> _decode(String raw) {
    try {
      return Map<String, dynamic>.from(jsonDecode(raw));
    } catch (_) {
      return {};
    }
  }

  /// Looks the code up, asks "Połączyć ten komputer z organizacją <nazwa>?", applies only after "Tak".
  Future<void> _lookupAndConfirm() async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _message = '';
    });
    String raw = '';
    try {
      raw = await bind.mainIwLookupOrgCode(code: _controller.text);
    } catch (_) {}
    var res = _decode(raw);
    if (!mounted) return;
    if (res['ok'] != true) {
      setState(() {
        _busy = false;
        _message = (res['message'] ?? 'Nie udało się połączyć z serwerem.').toString();
      });
      return;
    }
    final prompt = (res['prompt'] ?? '').toString();
    final yes = await showDialog<bool>(
        context: context,
        barrierDismissible: false,
        builder: (c) => AlertDialog(
              title: const Text('Potwierdź organizację'),
              content: Text(prompt),
              actions: [
                TextButton(
                    onPressed: () => Navigator.pop(c, false),
                    child: const Text('Nie')),
                TextButton(
                    onPressed: () => Navigator.pop(c, true),
                    child: const Text('Tak')),
              ],
            ));
    if (!mounted) return;
    if (yes != true) {
      setState(() => _busy = false);
      return;
    }
    try {
      raw = await bind.mainIwApplyOrgCode(code: _controller.text);
    } catch (_) {
      raw = '';
    }
    res = _decode(raw);
    if (!mounted) return;
    if (res['ok'] == true) {
      widget.onConfigured();
      return;
    }
    setState(() {
      _busy = false;
      _message = (res['message'] ?? 'Nie udało się zapisać organizacji.').toString();
    });
  }

  // IW: `.iw-screen` + `.iw-card` of the IncorrectWeb desktop app
  @override
  Widget build(BuildContext context) {
    final p = IwPalette.of(context);
    return Scaffold(
      backgroundColor: p.bg,
      body: Container(
        decoration: BoxDecoration(
          gradient: RadialGradient(
            center: const Alignment(-0.9, -1.2),
            radius: 1.3,
            colors: [p.violet.withOpacity(0.14), p.bg.withOpacity(0)],
          ),
        ),
        child: Container(
          decoration: BoxDecoration(
            gradient: RadialGradient(
              center: const Alignment(1.1, 1.2),
              radius: 1.1,
              colors: [p.orange.withOpacity(0.12), p.bg.withOpacity(0)],
            ),
          ),
          child: Center(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Container(
                width: 440,
                padding: const EdgeInsets.all(32),
                decoration: BoxDecoration(
                  color: p.panel,
                  borderRadius: BorderRadius.circular(kIwRadiusLg),
                  border: Border.all(color: p.line),
                  boxShadow: p.cardShadow,
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(children: [
                      const IwAppIcon(size: 48),
                      const SizedBox(width: 12),
                      Text(IwBranding.current.name,
                          style: TextStyle(
                              fontSize: 15,
                              fontWeight: FontWeight.w600,
                              color: p.muted)),
                    ]),
                    const SizedBox(height: 18),
                    Text('Wpisz kod organizacji',
                        style: TextStyle(
                            fontSize: 22,
                            fontWeight: FontWeight.w600,
                            letterSpacing: -0.2,
                            color: p.ink)),
                    const SizedBox(height: 6),
                    Text(
                        'Kod dostaniesz od firmy, która udziela Ci pomocy (np. ABCD-EFGH). Dzięki niemu program połączy się z jej serwerem.',
                        style: TextStyle(
                            fontSize: 14, height: 1.5, color: p.muted)),
                    const SizedBox(height: 18),
                    TextField(
                      controller: _controller,
                      enabled: !_busy,
                      autofocus: true,
                      textAlign: TextAlign.center,
                      textCapitalization: TextCapitalization.characters,
                      style: TextStyle(
                          fontSize: 22,
                          letterSpacing: 4,
                          fontWeight: FontWeight.w500,
                          color: p.ink,
                          fontFeatures: kIwTabular),
                      decoration: InputDecoration(
                          hintText: 'ABCD-EFGH',
                          contentPadding: const EdgeInsets.symmetric(
                              horizontal: 13, vertical: 14)),
                      onSubmitted: (_) => _lookupAndConfirm(),
                    ),
                    if (_message.isNotEmpty) ...[
                      const SizedBox(height: 10),
                      Container(
                        width: double.infinity,
                        padding: const EdgeInsets.symmetric(
                            horizontal: 12, vertical: 10),
                        decoration: BoxDecoration(
                            color: p.dangerSoft,
                            borderRadius: BorderRadius.circular(kIwRadiusSm)),
                        child: Text(_message,
                            style: TextStyle(color: p.dangerInk, fontSize: 13)),
                      ),
                    ],
                    const SizedBox(height: 18),
                    SizedBox(
                      width: double.infinity,
                      height: 44,
                      child: ElevatedButton(
                        style: ElevatedButton.styleFrom(
                            backgroundColor: p.orange,
                            foregroundColor: p.onOrange,
                            disabledBackgroundColor: p.line2,
                            textStyle: const TextStyle(
                                fontFamily: 'Onest',
                                fontSize: 15,
                                fontWeight: FontWeight.w600)),
                        onPressed: _busy ? null : _lookupAndConfirm,
                        child: Text(_busy ? 'Łączenie…' : 'Dalej'),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Wraps the home page: shows the organization screen until the program is configured.
class IwOrgGate extends StatefulWidget {
  final Widget child;
  const IwOrgGate({Key? key, required this.child}) : super(key: key);

  @override
  State<IwOrgGate> createState() => _IwOrgGateState();
}

class _IwOrgGateState extends State<IwOrgGate> {
  bool _configured = iwStatus()['configured'] == true;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    // Options reach this process from the server through IPC, so look again for a moment.
    if (!_configured) {
      _timer = Timer.periodic(const Duration(seconds: 1), (t) {
        if (iwStatus()['configured'] == true && mounted) {
          setState(() => _configured = true);
          t.cancel();
        }
      });
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_configured) return widget.child;
    return IwOrgPage(onConfigured: () {
      if (mounted) setState(() => _configured = true);
    });
  }
}

/// Banner of the main window: unattended access is on (with the way to switch it off).
class IwUnattendedBar extends StatefulWidget {
  const IwUnattendedBar({Key? key}) : super(key: key);

  @override
  State<IwUnattendedBar> createState() => _IwUnattendedBarState();
}

class _IwUnattendedBarState extends State<IwUnattendedBar> {
  Timer? _timer;
  bool _on = iwStatus()['unattended'] == true;
  bool _mismatch = iwStatus()['keyMismatch'] == true;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 2), (_) {
      final s = iwStatus();
      final on = s['unattended'] == true;
      final mismatch = s['keyMismatch'] == true;
      if ((on != _on || mismatch != _mismatch) && mounted) {
        setState(() {
          _on = on;
          _mismatch = mismatch;
        });
      }
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final children = <Widget>[];
    final p = IwPalette.of(context);
    if (_mismatch) {
      children.add(_bar(p.dangerSoft, p.dangerInk, p.danger,
          'To urządzenie jest już zarejestrowane — poproś konsultanta o reset.',
          null,
          null));
    }
    if (_on) {
      children.add(_bar(p.orangeSoft, p.orangeInk, p.orange,
          'Dostęp bez akceptacji jest włączony', 'Wyłącz',
          () async {
        await bind.mainIwSetUnattended(enable: false, password: '');
        if (mounted) setState(() => _on = false);
      }));
    }
    if (children.isEmpty) return const SizedBox.shrink();
    return Column(children: children);
  }

  // IW: soft band with a short bar of the tone (`.set-banner`)
  Widget _bar(Color bg, Color ink, Color bar, String text, String? action,
      VoidCallback? onAction) {
    return Container(
      width: double.infinity,
      color: bg,
      padding: const EdgeInsets.fromLTRB(0, 4, 12, 4),
      child: Row(
        children: [
          Container(width: 4, height: 20, color: bar),
          const SizedBox(width: 12),
          Expanded(
              child: Text(text,
                  style: TextStyle(
                      color: ink, fontSize: 13, fontWeight: FontWeight.w600))),
          if (action != null)
            TextButton(
                onPressed: onAction,
                style: TextButton.styleFrom(foregroundColor: ink),
                child: Text(action,
                    style: const TextStyle(
                        fontWeight: FontWeight.w700,
                        decoration: TextDecoration.underline))),
        ],
      ),
    );
  }
}

/// Settings -> Access without confirmation, Organization, History.
class IwSettingsSection extends StatefulWidget {
  const IwSettingsSection({Key? key}) : super(key: key);

  @override
  State<IwSettingsSection> createState() => _IwSettingsSectionState();
}

class _IwSettingsSectionState extends State<IwSettingsSection> {
  final _pass1 = TextEditingController();
  final _pass2 = TextEditingController();
  String _error = '';
  Map<String, dynamic> _status = iwStatus();

  void _refresh() => setState(() => _status = iwStatus());

  @override
  void dispose() {
    _pass1.dispose();
    _pass2.dispose();
    super.dispose();
  }

  Future<void> _enable() async {
    if (_pass1.text != _pass2.text) {
      setState(() => _error = 'Hasła nie są takie same.');
      return;
    }
    final err =
        await bind.mainIwSetUnattended(enable: true, password: _pass1.text);
    _pass1.clear();
    _pass2.clear();
    setState(() => _error = err);
    _refresh();
  }

  Future<void> _disable() async {
    await bind.mainIwSetUnattended(enable: false, password: '');
    setState(() => _error = '');
    _refresh();
  }

  @override
  Widget build(BuildContext context) {
    final org = (_status['org'] ?? '').toString();
    final on = _status['unattended'] == true;
    final p = IwPalette.of(context);
    final h = TextStyle(fontSize: 14.5, fontWeight: FontWeight.w600, color: p.ink);
    final lead = TextStyle(fontSize: 13.5, height: 1.45, color: p.muted);
    final log = bind.mainIwSessionsLog();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Organizacja', style: h),
        const SizedBox(height: 6),
        Row(children: [
          Expanded(
              child: Text(org.isEmpty ? 'Nie ustawiono' : org,
                  style: TextStyle(fontSize: 14, color: p.ink2))),
          OutlinedButton(
              onPressed: () async {
                final ok = await showDialog<bool>(
                    context: context,
                    builder: (c) => AlertDialog(
                          title: const Text('Zmienić organizację?'),
                          content: const Text(
                              'Program zapomni obecny kod i poprosi o nowy. Dostęp bez akceptacji zostanie wyłączony.'),
                          actions: [
                            TextButton(
                                onPressed: () => Navigator.pop(c, false),
                                child: const Text('Anuluj')),
                            TextButton(
                                onPressed: () => Navigator.pop(c, true),
                                child: const Text('Zmień')),
                          ],
                        ));
                if (ok == true) {
                  await bind.mainIwSetUnattended(enable: false, password: '');
                  bind.mainIwClearOrg();
                  _refresh();
                }
              },
              child: const Text('Zmień')),
        ]),
        const SizedBox(height: 14),
        Divider(height: 1, color: p.line),
        const SizedBox(height: 14),
        Row(children: [
          Expanded(child: Text('Dostęp bez akceptacji', style: h)),
          if (on)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 3),
              decoration: BoxDecoration(
                  color: p.orangeSoft, borderRadius: BorderRadius.circular(20)),
              child: Text('Włączony',
                  style: TextStyle(
                      color: p.orangeInk,
                      fontSize: 12.5,
                      fontWeight: FontWeight.w700)),
            ),
        ]),
        const SizedBox(height: 6),
        Text(
            'Połączyć się będą mogli tylko konsultanci ${org.isEmpty ? 'organizacji' : org} wskazani przez administratora, i tylko znając to hasło. Wymaga zainstalowania programu jako usługi.',
            style: lead),
        const SizedBox(height: 10),
        if (on) ...[
          OutlinedButton(onPressed: _disable, child: const Text('Wyłącz')),
        ] else ...[
          TextField(
              controller: _pass1,
              obscureText: true,
              decoration: const InputDecoration(
                  labelText: 'Hasło stałe (min. 10 znaków)')),
          const SizedBox(height: 8),
          TextField(
              controller: _pass2,
              obscureText: true,
              decoration: const InputDecoration(labelText: 'Powtórz hasło')),
          if (_error.isNotEmpty)
            Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(_error,
                    style: TextStyle(color: p.danger, fontSize: 13))),
          const SizedBox(height: 10),
          ElevatedButton(onPressed: _enable, child: const Text('Włącz')),
        ],
        const SizedBox(height: 14),
        Divider(height: 1, color: p.line),
        const SizedBox(height: 14),
        Text('Historia połączeń', style: h),
        const SizedBox(height: 8),
        Container(
          width: double.infinity,
          constraints: const BoxConstraints(maxHeight: 160),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
              color: p.panelSoft,
              border: Border.all(color: p.line),
              borderRadius: BorderRadius.circular(kIwRadiusSm)),
          child: SingleChildScrollView(
            child: SelectableText(log.isEmpty ? 'Brak połączeń.' : log,
                style: TextStyle(
                    fontSize: 12.5,
                    height: 1.5,
                    color: log.isEmpty ? p.muted : p.ink2,
                    fontFeatures: kIwTabular)),
          ),
        ),
      ],
    );
  }
}

/// "Połączyć z 123456789?" for a link of a consultant. Returns whether to connect.
Future<bool> iwConfirmLink(BuildContext context, String id) async {
  final ok = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (c) => AlertDialog(
            title: Text('Połączyć z $id?'),
            content: const Text(
                'Otrzymano link do połączenia z komputerem klienta. Połącz tylko wtedy, gdy to Ty go otworzyłeś w zgłoszeniu.'),
            actions: [
              TextButton(
                  onPressed: () => Navigator.pop(c, false),
                  child: const Text('Anuluj')),
              TextButton(
                  onPressed: () => Navigator.pop(c, true),
                  child: const Text('Połącz')),
            ],
          ));
  return ok == true;
}

/// A link of a consultant (`iwpomoc://connect/<id>?intent=<token>`): ask first, then connect.
Future<void> iwHandleLink(String link) async {
  Map<String, dynamic> res = {};
  try {
    res = Map<String, dynamic>.from(
        jsonDecode(bind.mainIwHandleLink(url: link, store: false)));
  } catch (_) {}
  if (res['ok'] != true) return; // other paths of the scheme are ignored
  final id = res['id'].toString();
  final context = globalKey.currentContext;
  if (context == null) return;
  windowOnTop(null);
  if (iwStatus()['configured'] != true) {
    await showDialog(
        context: context,
        builder: (c) => AlertDialog(
              title: const Text('Najpierw wpisz kod organizacji'),
              content: const Text(
                  'Program nie jest jeszcze skonfigurowany. Wpisz kod organizacji i otwórz link ponownie.'),
              actions: [
                TextButton(
                    onPressed: () => Navigator.pop(c), child: const Text('OK'))
              ],
            ));
    return;
  }
  if (await iwConfirmLink(context, id)) {
    // the token is kept only now, for the login that follows
    bind.mainIwHandleLink(url: link, store: true);
    Future.delayed(Duration.zero, () => rustDeskWinManager.newRemoteDesktop(id));
  }
}

/// "O programie": brand, version, license and origin (the program is a fork of RustDesk, AGPL-3.0),
/// and the link to the source code.
Widget iwAboutBlock(BuildContext context,
    {String version = '',
    String buildDate = '',
    String fingerprint = '',
    String license = ''}) {
  final s = iwStatus();
  final p = IwPalette.of(context);
  final source = (s['source'] ?? '').toString();
  final iwVersion = (s['version'] ?? '').toString(); // e.g. 1.4.9-iw.4
  final v = iwVersion.isNotEmpty ? iwVersion : version;
  Widget row(String label, String value) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 3),
        child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          SizedBox(
              width: 130,
              child: Text(label, style: TextStyle(color: p.muted, fontSize: 13.5))),
          Expanded(
              child: SelectableText(value,
                  style: TextStyle(
                      color: p.ink, fontSize: 13.5, fontFeatures: kIwTabular))),
        ]),
      );
  return Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Row(children: [
        const IwAppIcon(size: 56),
        const SizedBox(width: 14),
        Expanded(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(IwBranding.current.name,
                style: TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.w700,
                    letterSpacing: -0.3,
                    color: p.ink)),
            const SizedBox(height: 2),
            Text(IwBranding.current.company,
                style: TextStyle(fontSize: 13.5, color: p.muted)),
          ]),
        ),
      ]),
      const SizedBox(height: 14),
      if (v.isNotEmpty) row(translate('Version'), v),
      if (buildDate.isNotEmpty) row(translate('Build Date'), buildDate),
      if (fingerprint.isNotEmpty) row(translate('Fingerprint'), fingerprint),
      const SizedBox(height: 14),
      Container(
        width: double.infinity,
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: p.panelSoft,
          borderRadius: BorderRadius.circular(kIwRadiusSm),
          border: Border.all(color: p.line),
        ),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text('Licencja i pochodzenie',
              style: TextStyle(
                  fontWeight: FontWeight.w600, fontSize: 14, color: p.ink)),
          const SizedBox(height: 6),
          Text(
              '${IwBranding.current.name} to wolne oprogramowanie na licencji GNU AGPL-3.0. '
              'Powstał na bazie programu RustDesk (Copyright © Purslane Tech Pte. Ltd.), '
              'zmiany wprowadziła firma ${IwBranding.current.company}.',
              style: TextStyle(fontSize: 13.5, height: 1.45, color: p.ink2)),
          if (source.isNotEmpty) ...[
            const SizedBox(height: 8),
            InkWell(
              borderRadius: BorderRadius.circular(6),
              onTap: () => launchUrl(Uri.parse(source)),
              child: Text('Kod źródłowy: $source',
                  style: TextStyle(
                      fontSize: 13.5,
                      color: p.violet,
                      fontWeight: FontWeight.w500,
                      decoration: TextDecoration.underline,
                      decorationColor: p.violet)),
            ),
          ],
          if (license.trim().isNotEmpty) ...[
            const SizedBox(height: 8),
            SelectableText(license,
                style: TextStyle(fontSize: 12, height: 1.4, color: p.muted)),
          ],
        ]),
      ),
    ],
  );
}

// ---------------------------------------------------------------------------------------------
// IW: pieces of the dark column (`.side` of packages/ui): brand row, fields, icon buttons, footer.

/// Logo tile, product name and the organization (or the company of the branding).
class IwSideBrand extends StatelessWidget {
  const IwSideBrand({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final p = IwPalette.of(context);
    final org = (iwStatus()['org'] ?? '').toString();
    return Row(children: [
      const IwLogoTile(size: 38),
      const SizedBox(width: 11),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(IwBranding.current.name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                    fontSize: 15.5,
                    fontWeight: FontWeight.w700,
                    letterSpacing: -0.2,
                    color: Colors.white)),
            const SizedBox(height: 2),
            Text(org.isNotEmpty ? org : IwBranding.current.company,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 12.5, color: p.sideDim)),
          ],
        ),
      ),
    ]);
  }
}

/// A value of this computer on the dark column: label, large value, small actions.
class IwSideField extends StatelessWidget {
  final String label;
  final Listenable listenable;
  final WidgetBuilder valueBuilder;
  final List<Widget> actions;
  const IwSideField(
      {Key? key,
      required this.label,
      required this.listenable,
      required this.valueBuilder,
      this.actions = const []})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    final p = IwPalette.of(context);
    return Container(
      margin: const EdgeInsets.fromLTRB(12, 0, 12, 8),
      padding: const EdgeInsets.fromLTRB(14, 10, 6, 10),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.06),
        borderRadius: BorderRadius.circular(kIwRadius),
        border: Border.all(color: p.sideLine),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label,
              style: TextStyle(
                  fontSize: 12.5, fontWeight: FontWeight.w500, color: p.sideMuted)),
          const SizedBox(height: 4),
          Row(children: [
            Expanded(
                child: AnimatedBuilder(
                    animation: listenable,
                    builder: (context, _) => valueBuilder(context))),
            ...actions,
          ]),
        ],
      ),
    );
  }
}

/// 32 px icon button for the dark column.
class IwSideIconButton extends StatelessWidget {
  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;
  const IwSideIconButton(
      {Key? key, required this.icon, required this.tooltip, required this.onPressed})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    final p = IwPalette.of(context);
    return Tooltip(
      message: tooltip,
      waitDuration: const Duration(milliseconds: 400),
      child: Material(
        type: MaterialType.transparency,
        child: InkWell(
          onTap: onPressed,
          borderRadius: BorderRadius.circular(8),
          hoverColor: p.sideHover,
          focusColor: p.sideAccent.withOpacity(0.3),
          child: SizedBox(
            width: 32,
            height: 32,
            child: Icon(icon, size: 18, color: p.sideDim, semanticLabel: tooltip),
          ),
        ),
      ),
    );
  }
}

/// Bottom of the dark column: the connection status and the way to the settings.
class IwSideFooter extends StatelessWidget {
  final Widget status;
  final VoidCallback? onSettings;
  const IwSideFooter({Key? key, required this.status, this.onSettings})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    final p = IwPalette.of(context);
    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 12),
      padding: const EdgeInsets.only(top: 6, bottom: 10),
      decoration: BoxDecoration(border: Border(top: BorderSide(color: p.sideLine))),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          DefaultTextStyle.merge(
              style: TextStyle(color: p.sideText, fontSize: 13.5), child: status),
          if (onSettings != null)
            Material(
              type: MaterialType.transparency,
              child: InkWell(
                onTap: onSettings,
                borderRadius: BorderRadius.circular(kIwRadiusSm),
                hoverColor: p.sideHover,
                focusColor: p.sideAccent.withOpacity(0.3),
                child: SizedBox(
                  height: 38,
                  child: Row(children: [
                    const SizedBox(width: 12),
                    Icon(Icons.settings_outlined, size: 19, color: p.sideDim),
                    const SizedBox(width: 12),
                    Text(translate('Settings'),
                        style: TextStyle(fontSize: 14, color: p.sideMuted)),
                  ]),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
