//! IW: glue between the pure logic in `src/iw` and the rest of the app: runtime state, the HTTP transport,
//! the background worker (hello, state, queued session reports), organization setup, safe defaults.

use crate::iw::{
    self,
    api::{
        self, Backoff, DeviceClient, HelloResult, HttpResponse, SessionEvent, SessionQueue, Sent, SignedRequest,
        Transport, TransportFuture,
    },
    consent::{VerifiedInfo, VerifyOutcome},
    link::{self, IntentStore},
    org::{self, OrgError},
};
use hbb_common::{
    config::{self, keys, Config},
    log,
    sodiumoxide::crypto::sign,
    tokio,
};
use serde_json::json;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

pub const OPT_API: &str = "iw-api";
pub const OPT_ORG_NAME: &str = "iw-org-name";
pub const OPT_ORG_CODE: &str = "iw-org-code";
pub const OPT_UNATTENDED: &str = "iw-unattended";
pub const OPT_KEY_MISMATCH: &str = "iw-key-mismatch";
const DEFAULT_LICENSE_URL: &str = "https://license.incorrectus.com";
const HELLO_EVERY: Duration = Duration::from_secs(24 * 3600);

lazy_static::lazy_static! {
    static ref INTENTS: Mutex<IntentStore> = Mutex::new(IntentStore::default());
    static ref QUEUE: Mutex<SessionQueue> = Mutex::new(load_queue());
    /// The organization the user is asked about; applied only by `apply_org_code` after "yes".
    static ref PENDING_ORG: Mutex<org::PendingOrg> = Mutex::new(org::PendingOrg::default());
}

/// What one connection knows about the controller; lives in `Connection`.
#[derive(Default)]
pub struct IwConn {
    pub verify: Option<VerifyOutcome>,
    pub intent: Option<String>,
    /// Reject when nobody clicks "Allow" before this moment.
    pub deadline: Option<Instant>,
    pub method: &'static str,
    pub unattended: bool,
    pub conn_id: String,
    pub started: Option<Instant>,
    /// The one-time password was used: replace it when the session ends.
    pub rotate_temporary_password: bool,
}

impl IwConn {
    pub fn verified_info(&self) -> Option<&VerifiedInfo> {
        match self.verify.as_ref() {
            Some(VerifyOutcome::Verified(v)) => Some(v),
            _ => None,
        }
    }

    /// JSON for the consent window (`Client.iw`); empty values mean "unknown person".
    pub fn to_json(&self) -> String {
        let v = self.verified_info();
        json!({
            "verified": v.is_some(),
            "name": v.map(|v| v.consultant_name.as_str()).unwrap_or(""),
            "company": v.map(|v| v.company.as_str()).unwrap_or(""),
            "ticket": v.and_then(|v| v.ticket_id),
            "ticketTitle": v.and_then(|v| v.ticket_title.as_deref()).unwrap_or(""),
            "unattended": self.unattended,
            "method": self.method,
            "org": Config::get_option(OPT_ORG_NAME),
        })
        .to_string()
    }
}

fn now_secs() -> i64 {
    hbb_common::chrono::Utc::now().timestamp()
}

// ---- safe defaults ------------------------------------------------------------------------------

/// Called once at start of every process.
pub fn init() {
    {
        let mut d = config::DEFAULT_SETTINGS.write().unwrap();
        for (k, v) in [
            ("verification-method", "use-temporary-password"),
            ("temporary-password-length", "8"),
            ("enable-lan-discovery", iw::policy::DEFAULT_LAN_DISCOVERY),
        ] {
            d.entry(k.to_owned()).or_insert_with(|| v.to_owned());
        }
    }
    // IW: fit to window and show the client cursor for computers without a saved choice
    {
        let mut d = config::DEFAULT_DISPLAY_SETTINGS.write().unwrap();
        for (k, v) in iw::policy::DISPLAY_DEFAULTS {
            d.entry((*k).to_owned()).or_insert_with(|| (*v).to_owned());
        }
    }
    {
        let mut o = config::OVERWRITE_SETTINGS.write().unwrap();
        for (k, v) in [(keys::OPTION_ALLOW_REMOTE_CONFIG_MODIFICATION, "N"), ("allow-hide-cm", "N")] {
            o.insert(k.to_owned(), v.to_owned());
        }
    }
    {
        let mut b = config::BUILTIN_SETTINGS.write().unwrap();
        for k in [
            keys::OPTION_HIDE_NETWORK_SETTINGS,
            keys::OPTION_HIDE_SERVER_SETTINGS,
            keys::OPTION_HIDE_PROXY_SETTINGS,
            keys::OPTION_HIDE_WEBSOCKET_SETTINGS,
            keys::OPTION_HIDE_SECURITY_SETTINGS,
            keys::OPTION_HIDE_HELP_CARDS,
            keys::OPTION_HIDE_POWERED_BY_ME,
        ] {
            b.insert(k.to_owned(), "Y".to_owned());
        }
        // no device registration, heartbeat or audit posts to a RustDesk API server
        b.insert(keys::OPTION_REGISTER_DEVICE.to_owned(), "N".to_owned());
    }
    #[cfg(windows)]
    if is_gui_launch() {
        register_url_scheme();
    }
}

#[cfg(windows)]
fn is_gui_launch() -> bool {
    let mut args = std::env::args().skip(1);
    match args.next() {
        None => true,
        Some(a) => a.to_lowercase().starts_with(&format!("{}://", link::SCHEME)),
    }
}

/// HKCU registration of `iwpomoc://` (no administrator rights, works for the portable program).
/// A copy started from Downloads or a temporary folder does not take over an existing registration.
#[cfg(windows)]
fn register_url_scheme() {
    use winreg::{enums::*, RegKey};
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let hcu = RegKey::predef(HKEY_CURRENT_USER);
    let base = format!("Software\\Classes\\{}", link::SCHEME);
    let command_path = format!("{base}\\shell\\open\\command");
    let existing: Option<String> = hcu
        .open_subkey(&command_path)
        .ok()
        .and_then(|k| k.get_value::<String, _>("").ok());
    let (_, install_dir, _, _) = crate::platform::windows::get_install_info();
    if !iw::scheme::should_register(&exe.to_string_lossy(), &install_dir, existing.as_deref()) {
        return;
    }
    if let Ok((key, _)) = hcu.create_subkey(&base) {
        key.set_value("", &format!("URL:{}", iw::DISPLAY_NAME)).ok();
        key.set_value("URL Protocol", &"").ok();
    }
    if let Ok((key, _)) = hcu.create_subkey(&command_path) {
        key.set_value("", &format!("\"{}\" \"%1\"", exe.display())).ok();
    }
}

// ---- organization --------------------------------------------------------------------------------

/// Code from the name of the downloaded file (`IncorrectWebPomoc-ABCD-EFGH.exe`), if any.
pub fn org_code_from_exe() -> String {
    if let Ok(name) = std::env::var(crate::common::PORTABLE_APPNAME_RUNTIME_ENV_KEY) {
        if let Some(code) = org::code_from_filename(&name) {
            return code;
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| org::code_from_filename(&p.to_string_lossy()))
        .unwrap_or_default()
}

fn license_url() -> String {
    std::env::var("IW_LICENSE_URL").unwrap_or_else(|_| DEFAULT_LICENSE_URL.to_owned())
}

/// First step: asks the license server who owns the code. Nothing is applied; the answer is kept for
/// `apply_org_code`. `{"ok":true,"name":..,"prompt":..}` or `{"ok":false,"message":..}` (Polish text).
pub fn lookup_org_code(input: &str) -> String {
    let result = || -> Result<(String, org::OrgConfig), String> {
        let code = org::normalize_code(input).ok_or("To nie wygląda na kod organizacji. Kod ma postać ABCD-EFGH.")?;
        let url = format!("{}/connect/{}", license_url().trim_end_matches('/'), code);
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(format!("IncorrectWebPomoc/{}", iw::IW_VERSION))
            .build()
            .map_err(|_| "Nie udało się połączyć z serwerem.".to_owned())?;
        let res = client.get(url).send().map_err(|_| "Nie udało się połączyć z serwerem. Sprawdź internet.".to_owned())?;
        if res.status().as_u16() == 404 {
            return Err("Nie znaleziono organizacji o tym kodzie.".to_owned());
        }
        if !res.status().is_success() {
            return Err("Serwer odpowiedział błędem. Spróbuj za chwilę.".to_owned());
        }
        let text = res.text().map_err(|_| "Nieczytelna odpowiedź serwera.".to_owned())?;
        match org::parse_license_response(&text) {
            Ok(cfg) => Ok((code, cfg)),
            Err(OrgError::NoRemote) => Err("Ta organizacja nie ma włączonej zdalnej pomocy.".to_owned()),
            Err(OrgError::Invalid) => Err("Nieprawidłowa odpowiedź serwera.".to_owned()),
        }
    };
    match result() {
        Ok((code, cfg)) => {
            let out = json!({ "ok": true, "name": cfg.name, "prompt": org::confirm_prompt(&cfg.name) });
            PENDING_ORG.lock().unwrap().put(&code, cfg);
            out.to_string()
        }
        Err(message) => json!({ "ok": false, "message": message }).to_string(),
    }
}

/// Second step, after the user answered "yes": applies exactly the organization `lookup_org_code` showed.
pub fn apply_org_code(input: &str) -> String {
    let Some(code) = org::normalize_code(input) else {
        return json!({ "ok": false, "message": "To nie wygląda na kod organizacji." }).to_string();
    };
    let Some(cfg) = PENDING_ORG.lock().unwrap().take(&code) else {
        return json!({ "ok": false, "message": "Najpierw sprawdź kod organizacji." }).to_string();
    };
    for (k, v) in cfg.options() {
        crate::ui_interface::set_option(k.to_owned(), v);
    }
    crate::ui_interface::set_option(OPT_ORG_CODE.to_owned(), code);
    crate::ui_interface::set_option(OPT_KEY_MISMATCH.to_owned(), String::new());
    json!({ "ok": true, "name": cfg.name }).to_string()
}

/// For the UI process: option values come through the cached copy of the server config.
pub fn status_json() -> String {
    let get = |k: &str| crate::ui_interface::get_option(k.to_owned());
    json!({
        "configured": !get(OPT_API).is_empty() && !get("custom-rendezvous-server").is_empty(),
        "org": get(OPT_ORG_NAME),
        "unattended": get(OPT_UNATTENDED) == "Y",
        "keyMismatch": get(OPT_KEY_MISMATCH) == "Y",
        "version": iw::IW_VERSION,
        "name": iw::DISPLAY_NAME,
        "source": iw::SOURCE_URL,
    })
    .to_string()
}

/// Clears the organization (Settings -> Organization -> Change).
pub fn clear_org() {
    for k in [
        "custom-rendezvous-server",
        "relay-server",
        "key",
        OPT_API,
        OPT_ORG_NAME,
        OPT_ORG_CODE,
        OPT_KEY_MISMATCH,
    ] {
        crate::ui_interface::set_option(k.to_owned(), String::new());
    }
}

// ---- unattended access -------------------------------------------------------------------------

/// Server side: switched on in the program and a permanent password exists.
pub fn unattended_enabled() -> bool {
    Config::get_option(OPT_UNATTENDED) == "Y" && Config::has_permanent_password()
}

/// UI side. Returns an empty string on success or a Polish error message.
pub fn set_unattended(enable: bool, password: &str) -> String {
    if enable {
        if password.chars().count() < 10 {
            return "Hasło stałe musi mieć co najmniej 10 znaków.".to_owned();
        }
        if !crate::ui_interface::set_permanent_password_with_result(password.to_owned()) {
            return "Nie udało się zapisać hasła.".to_owned();
        }
        crate::ui_interface::set_option("verification-method".to_owned(), "use-both-passwords".to_owned());
        crate::ui_interface::set_option(OPT_UNATTENDED.to_owned(), "Y".to_owned());
    } else {
        crate::ui_interface::set_option(OPT_UNATTENDED.to_owned(), String::new());
        crate::ui_interface::set_option("verification-method".to_owned(), String::new());
        crate::ui_interface::set_permanent_password_with_result(String::new());
    }
    String::new()
}

// ---- intent of the iwpomoc link (controlling side) ----------------------------------------------------

/// Checks a link; with `store` (after the user confirmed) keeps its token for the next login to the peer.
pub fn handle_link(url: &str, store: bool) -> String {
    match link::parse_link(url) {
        Some(l) => {
            if let (true, Some(token)) = (store, l.intent.as_deref()) {
                INTENTS.lock().unwrap().put(&l.id, token, Instant::now());
            }
            json!({ "ok": true, "id": l.id, "intent": l.intent.is_some() }).to_string()
        }
        None => json!({ "ok": false }).to_string(),
    }
}

/// Token for the login request to `peer_id` (once, within two minutes).
pub fn take_intent(peer_id: &str) -> String {
    INTENTS.lock().unwrap().take(peer_id, Instant::now()).unwrap_or_default()
}

// ---- device API ----------------------------------------------------------------------------------

pub struct HttpTransport {
    client: reqwest::Client,
}

impl HttpTransport {
    fn new() -> Option<Self> {
        reqwest::Client::builder()
            .timeout(api::CALL_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(format!("IncorrectWebPomoc/{}", iw::IW_VERSION))
            .build()
            .ok()
            .map(|client| Self { client })
    }
}

impl Transport for HttpTransport {
    fn post<'a>(&'a self, req: &'a SignedRequest) -> TransportFuture<'a> {
        Box::pin(async move {
            let mut builder = self
                .client
                .post(&req.url)
                .header("content-type", "application/json")
                .header("x-iw-ts", req.ts.to_string())
                .header("x-iw-sig", &req.signature)
                .body(req.body.clone());
            if let Some(device) = &req.device {
                builder = builder.header("x-iw-device", device);
            }
            let res = builder.send().await.map_err(|e| e.without_url().to_string())?;
            let status = res.status().as_u16();
            let body = res.text().await.map_err(|e| e.without_url().to_string())?;
            Ok(HttpResponse { status, body })
        })
    }
}

/// The device key is the Ed25519 pair RustDesk keeps for this computer (shared by the user and service
/// processes through the config), so the key and the ID always change together.
fn device_keys() -> Option<(sign::PublicKey, sign::SecretKey)> {
    let (sk, pk) = Config::get_key_pair();
    Some((sign::PublicKey::from_slice(&pk)?, sign::SecretKey::from_slice(&sk)?))
}

fn device_client() -> Option<DeviceClient<HttpTransport>> {
    let api = Config::get_option(OPT_API);
    let id = Config::get_id();
    if api.is_empty() || id.is_empty() {
        return None;
    }
    Some(DeviceClient {
        api,
        device_id: id,
        sk: device_keys()?.1,
        transport: HttpTransport::new()?,
        clock: now_secs,
    })
}

/// `verify` for one connection; fail closed when the program is not configured.
pub async fn verify(intent: &str, controller_id: &str) -> VerifyOutcome {
    match device_client() {
        Some(c) => c.verify(intent, controller_id, api::VERIFY_TIMEOUT).await,
        None => VerifyOutcome::NotVerified,
    }
}

// ---- session reports, local log -----------------------------------------------------------------

fn queue_path() -> std::path::PathBuf {
    Config::path("iw-queue.json")
}

fn load_queue() -> SessionQueue {
    std::fs::read_to_string(queue_path()).map(|s| SessionQueue::from_json(&s)).unwrap_or_default()
}

fn save_queue(q: &SessionQueue) {
    if let Err(e) = std::fs::write(queue_path(), q.to_json()) {
        log::warn!("iw: cannot save the session queue: {e}");
    }
}

/// Queues a report; the worker sends it (and resends after network errors).
pub fn report_session(event: SessionEvent) {
    let mut q = QUEUE.lock().unwrap();
    q.push(api::session_body(&event));
    save_queue(&q);
}

// ---- background worker (server process) -----------------------------------------------------------

/// Starts hello (at start and every 24 h), state sync and the session report queue.
pub fn start_background() {
    std::thread::spawn(|| {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("iw: no runtime for the worker: {e}");
                return;
            }
        };
        rt.block_on(worker());
    });
}

async fn worker() {
    let mut hello_key = String::new();
    let mut next_hello = Instant::now();
    let mut hello_backoff = Backoff::default();
    let mut reported_unattended: Option<bool> = None;
    let mut registered = false;
    let mut queue_backoff = Backoff::default();
    let mut next_queue = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let Some(client) = device_client() else {
            continue;
        };
        let key = format!("{}|{}", client.api, client.device_id);
        if key != hello_key {
            hello_key = key;
            next_hello = Instant::now();
            reported_unattended = None;
            registered = false;
        }
        if Instant::now() >= next_hello {
            let res = hello(&client).await;
            match res {
                HelloResult::Ok => {
                    registered = true;
                    Config::set_option(OPT_KEY_MISMATCH.to_owned(), String::new());
                    hello_backoff.reset();
                    next_hello = Instant::now() + HELLO_EVERY;
                }
                HelloResult::KeyMismatch => {
                    log::warn!("iw: this device is registered with another key; an administrator must reset it");
                    Config::set_option(OPT_KEY_MISMATCH.to_owned(), "Y".to_owned());
                    next_hello = Instant::now() + Duration::from_secs(600);
                }
                HelloResult::Failed => next_hello = Instant::now() + hello_backoff.next_delay(),
            }
        }
        let unattended = unattended_enabled();
        if registered && reported_unattended != Some(unattended) {
            let req = api::build_request(
                &client.sk,
                &client.api,
                "state",
                Some(&client.device_id),
                now_secs(),
                api::state_body(unattended),
            );
            if matches!(api::classify(&client.transport.post(&req).await), Sent::Done) {
                reported_unattended = Some(unattended);
            }
        }
        if Instant::now() >= next_queue {
            let body = QUEUE.lock().unwrap().front().cloned();
            if let Some(body) = body {
                let req = api::build_request(&client.sk, &client.api, "session", Some(&client.device_id), now_secs(), body);
                match api::classify(&client.transport.post(&req).await) {
                    Sent::Done | Sent::Drop => {
                        let mut q = QUEUE.lock().unwrap();
                        q.pop_front();
                        save_queue(&q);
                        queue_backoff.reset();
                    }
                    Sent::Retry => next_queue = Instant::now() + queue_backoff.next_delay(),
                }
            }
        }
    }
}

async fn hello(client: &DeviceClient<HttpTransport>) -> HelloResult {
    let Some((pk, _)) = device_keys() else {
        return HelloResult::Failed;
    };
    let body = api::hello_body(
        &client.device_id,
        &pk,
        &hbb_common::whoami::devicename(),
        &format!("{} {}", hbb_common::whoami::platform(), hbb_common::whoami::distro()),
        iw::IW_VERSION,
    );
    let req = api::build_request(&client.sk, &client.api, "hello", None, now_secs(), body);
    match client.transport.post(&req).await {
        Ok(res) => api::parse_hello(res.status, &res.body),
        Err(_) => HelloResult::Failed,
    }
}

// ---- local session log (written by the connection manager, visible in the program) -----------------

fn sessions_log_path() -> std::path::PathBuf {
    Config::log_path().join("iw-sessions.log")
}

/// Appends one line for a finished session; `iw` is the JSON the connection manager received.
pub fn log_session(iw: &str, controller_id: &str, duration_sec: u64) {
    use std::io::Write;
    let v: serde_json::Value = serde_json::from_str(iw).unwrap_or_default();
    let text = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_owned();
    let (name, company) = (text("name"), text("company"));
    let person = match (name.is_empty(), company.is_empty()) {
        (true, _) => String::new(),
        (false, true) => name,
        (false, false) => format!("{name} ({company})"),
    };
    let method = if v.get("unattended") == Some(&serde_json::Value::Bool(true)) {
        "unattended".to_owned()
    } else {
        text("method")
    };
    let when = hbb_common::chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = iw::sessionlog::format_line(&when, &person, controller_id, &method, duration_sec);
    let path = sessions_log_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            writeln!(f, "{line}").ok();
        }
        Err(e) => log::warn!("iw: cannot write the sessions log: {e}"),
    }
}

/// Connection manager: a session that needed no click began; tell the user.
pub fn cm_client_authorized(iw: &str) {
    let v: serde_json::Value = serde_json::from_str(iw).unwrap_or_default();
    if v.get("unattended") == Some(&serde_json::Value::Bool(true)) {
        let text = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_owned();
        notify_unattended(&text("name"), &text("company"));
    }
}

/// The last lines of the log for the "Connection history" screen.
pub fn sessions_log_tail(max_lines: usize) -> String {
    let text = std::fs::read_to_string(sessions_log_path()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(max_lines)..].join("\n")
}

/// System notification for a session that did not ask for a click (unattended access).
pub fn notify_unattended(person: &str, company: &str) {
    let text = format!("Konsultant {person} ({company}) połączył się z tym komputerem.");
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("osascript")
            .args(iw::notify::osascript_args(iw::DISPLAY_NAME, &text))
            .spawn()
            .ok();
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let call = iw::notify::powershell_toast(iw::DISPLAY_NAME, &text);
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(&call.args).creation_flags(0x0800_0000);
        for (k, v) in &call.env {
            cmd.env(k, v);
        }
        cmd.spawn().ok();
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    log::info!("iw: {text}");
}
