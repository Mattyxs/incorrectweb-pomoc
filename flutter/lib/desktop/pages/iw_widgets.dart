// IW: widgets of IncorrectWeb Pomoc (consent banner, organization code, unattended access,
// link confirmation, history). All texts are Polish on purpose.
import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_hbb/models/server_model.dart';
import 'package:flutter_hbb/utils/multi_window_manager.dart';
import 'package:url_launcher/url_launcher.dart';

const Color kIwViolet = Color(0xFF431C77);
const Color kIwOrange = Color(0xFFEF7D1D);

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
    Color color;
    String title;
    String body;
    if (!client.authorized) {
      if (client.iwVerified) {
        color = Colors.green.shade700;
        title = who;
        body = 'chce połączyć się z tym komputerem. Pozwolić?';
      } else {
        color = Colors.red.shade700;
        final org = client.iwOrg.isNotEmpty ? client.iwOrg : 'organizację';
        title = 'Nieznana osoba (nie potwierdzona przez $org)';
        body =
            'Nie akceptuj, jeśli nie rozmawiasz właśnie z konsultantem. Numer: ${client.peerId}';
      }
    } else if (client.iwUnattended) {
      color = kIwOrange;
      title = 'Dostęp bez akceptacji';
      body = 'Konsultant $who połączył się z tym komputerem.';
    } else if (client.iwVerified) {
      color = Colors.green.shade700;
      title = who;
      body = 'połączenie potwierdzone przez organizację.';
    } else {
      return const SizedBox.shrink();
    }
    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 5.0, vertical: 2.0),
      padding: const EdgeInsets.all(8.0),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        border: Border.all(color: color, width: 1.5),
        borderRadius: BorderRadius.circular(8.0),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title,
              style: TextStyle(
                  color: color, fontWeight: FontWeight.bold, fontSize: 13)),
          const SizedBox(height: 2),
          Text(body, style: const TextStyle(fontSize: 12)),
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

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 380),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Wpisz kod organizacji',
                    style: TextStyle(fontSize: 22, fontWeight: FontWeight.bold)),
                const SizedBox(height: 8),
                const Text(
                    'Kod dostaniesz od firmy, która udziela Ci pomocy (np. ABCD-EFGH). Dzięki niemu program połączy się z jej serwerem.'),
                const SizedBox(height: 16),
                TextField(
                  controller: _controller,
                  enabled: !_busy,
                  autofocus: true,
                  textCapitalization: TextCapitalization.characters,
                  decoration: const InputDecoration(
                      border: OutlineInputBorder(), hintText: 'ABCD-EFGH'),
                  onSubmitted: (_) => _lookupAndConfirm(),
                ),
                if (_message.isNotEmpty) ...[
                  const SizedBox(height: 10),
                  Text(_message, style: TextStyle(color: Colors.red.shade700)),
                ],
                const SizedBox(height: 16),
                SizedBox(
                  width: double.infinity,
                  child: ElevatedButton(
                    style: ElevatedButton.styleFrom(
                        backgroundColor: kIwViolet,
                        foregroundColor: Colors.white,
                        padding: const EdgeInsets.symmetric(vertical: 14)),
                    onPressed: _busy ? null : _lookupAndConfirm,
                    child: Text(_busy ? 'Łączenie…' : 'Dalej'),
                  ),
                ),
              ],
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
    if (_mismatch) {
      children.add(_bar(
          Colors.red.shade700,
          'To urządzenie jest już zarejestrowane — poproś konsultanta o reset.',
          null,
          null));
    }
    if (_on) {
      children.add(_bar(kIwOrange, 'Dostęp bez akceptacji jest włączony', 'Wyłącz',
          () async {
        await bind.mainIwSetUnattended(enable: false, password: '');
        if (mounted) setState(() => _on = false);
      }));
    }
    if (children.isEmpty) return const SizedBox.shrink();
    return Column(children: children);
  }

  Widget _bar(Color color, String text, String? action, VoidCallback? onAction) {
    return Container(
      width: double.infinity,
      color: color,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      child: Row(
        children: [
          Expanded(
              child: Text(text,
                  style: const TextStyle(color: Colors.white, fontSize: 13))),
          if (action != null)
            TextButton(
                onPressed: onAction,
                child: Text('· $action',
                    style: const TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('Organizacja',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
        const SizedBox(height: 4),
        Row(children: [
          Expanded(child: Text(org.isEmpty ? 'Nie ustawiono' : org)),
          TextButton(
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
        const Divider(),
        const Text('Dostęp bez akceptacji',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
        const SizedBox(height: 4),
        Text(
            'Połączyć się będą mogli tylko konsultanci ${org.isEmpty ? 'organizacji' : org} wskazani przez administratora, i tylko znając to hasło. Wymaga zainstalowania programu jako usługi.'),
        const SizedBox(height: 8),
        if (on) ...[
          const Text('Włączony.',
              style: TextStyle(color: kIwOrange, fontWeight: FontWeight.bold)),
          TextButton(onPressed: _disable, child: const Text('Wyłącz')),
        ] else ...[
          TextField(
              controller: _pass1,
              obscureText: true,
              decoration: const InputDecoration(
                  labelText: 'Hasło stałe (min. 10 znaków)')),
          TextField(
              controller: _pass2,
              obscureText: true,
              decoration: const InputDecoration(labelText: 'Powtórz hasło')),
          if (_error.isNotEmpty)
            Padding(
                padding: const EdgeInsets.only(top: 6),
                child: Text(_error, style: TextStyle(color: Colors.red.shade700))),
          const SizedBox(height: 8),
          ElevatedButton(
              style: ElevatedButton.styleFrom(
                  backgroundColor: kIwViolet, foregroundColor: Colors.white),
              onPressed: _enable,
              child: const Text('Włącz')),
        ],
        const Divider(),
        const Text('Historia połączeń',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
        const SizedBox(height: 4),
        Container(
          width: double.infinity,
          constraints: const BoxConstraints(maxHeight: 160),
          padding: const EdgeInsets.all(8),
          decoration: BoxDecoration(
              border: Border.all(color: Colors.grey),
              borderRadius: BorderRadius.circular(6)),
          child: SingleChildScrollView(
            child: SelectableText(
                bind.mainIwSessionsLog().isEmpty
                    ? 'Brak połączeń.'
                    : bind.mainIwSessionsLog(),
                style: const TextStyle(fontSize: 12)),
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

/// "O programie": version, license and the link to the source code.
Widget iwAboutBlock() {
  final s = iwStatus();
  return Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text('${s['name'] ?? 'IncorrectWeb Pomoc'} ${s['version'] ?? ''}',
          style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
      const SizedBox(height: 6),
      const Text('Oparty na RustDesk (AGPL-3.0).'),
      const SizedBox(height: 4),
      InkWell(
        onTap: () =>
            launchUrl(Uri.parse((s['source'] ?? '').toString())),
        child: Text('Kod źródłowy: ${s['source'] ?? ''}',
            style: const TextStyle(
                color: Colors.blue, decoration: TextDecoration.underline)),
      ),
    ],
  );
}
