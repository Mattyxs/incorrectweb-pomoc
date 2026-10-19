//! Client of the device API (docs/connect/remote-api.md section 3): signed requests, fail-closed `verify`,
//! and the retry queue of session reports. The network itself is a [`Transport`] (reqwest in the app).

use super::consent::{VerifiedInfo, VerifyOutcome};
use hbb_common::sodiumoxide::crypto::sign;
use std::{fmt, future::Future, pin::Pin, time::Duration};

pub const VERIFY_TIMEOUT: Duration = Duration::from_secs(5);
pub const CALL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, PartialEq, Eq)]
pub struct SignedRequest {
    pub url: String,
    pub body: String,
    pub ts: i64,
    pub signature: String,
    /// `x-iw-device`; absent for `hello`.
    pub device: Option<String>,
}

// The body of `verify` and `session` holds the intent token, so it never reaches a log line.
impl fmt::Debug for SignedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignedRequest({}, ts {})", self.url, self.ts)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub type TransportFuture<'a> = Pin<Box<dyn Future<Output = Result<HttpResponse, String>> + Send + 'a>>;

pub trait Transport: Send + Sync {
    fn post<'a>(&'a self, req: &'a SignedRequest) -> TransportFuture<'a>;
}

/// Builds a request signed for `endpoint` with the time `ts` (a retry builds a new one).
pub fn build_request(
    sk: &sign::SecretKey,
    api: &str,
    endpoint: &str,
    device: Option<&str>,
    ts: i64,
    body: String,
) -> SignedRequest {
    SignedRequest {
        url: format!("{}/api/remote/device/{endpoint}", api.trim_end_matches('/')),
        signature: super::sign::sign_request(sk, endpoint, ts, &body),
        body,
        ts,
        device: device.map(str::to_owned),
    }
}

fn text(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_owned)
}

/// Reads the answer to `verify`; anything but a clean `ok:true` is "not verified".
pub fn parse_verify(status: u16, body: &str) -> VerifyOutcome {
    if status != 200 {
        return VerifyOutcome::NotVerified;
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return VerifyOutcome::NotVerified;
    };
    if v.get("ok") != Some(&serde_json::Value::Bool(true)) {
        return VerifyOutcome::NotVerified;
    }
    let consultant = v.get("consultant");
    let (Some(name), Some(company), Some(mode)) = (
        consultant.and_then(|c| text(c, "name")),
        text(&v, "company"),
        text(&v, "mode"),
    ) else {
        return VerifyOutcome::NotVerified;
    };
    let ticket = v.get("ticket").filter(|t| t.is_object());
    VerifyOutcome::Verified(VerifiedInfo {
        consultant_name: name,
        consultant_email: consultant.and_then(|c| text(c, "email")).unwrap_or_default(),
        company,
        ticket_id: ticket.and_then(|t| t.get("id")).and_then(|x| x.as_u64()),
        ticket_title: ticket.and_then(|t| text(t, "title")),
        unattended_mode: mode == "unattended",
        unattended_allowed: v.get("unattendedAllowed") == Some(&serde_json::Value::Bool(true)),
    })
}

pub struct DeviceClient<T: Transport> {
    pub api: String,
    pub device_id: String,
    pub sk: sign::SecretKey,
    pub transport: T,
    pub clock: fn() -> i64,
}

impl<T: Transport> DeviceClient<T> {
    fn request(&self, endpoint: &str, body: String) -> SignedRequest {
        build_request(&self.sk, &self.api, endpoint, Some(&self.device_id), (self.clock)(), body)
    }

    /// `POST verify`: fail closed. No intent, timeout, network error, HTTP error: not verified.
    pub async fn verify(&self, intent: &str, controller_id: &str, timeout: Duration) -> VerifyOutcome {
        if intent.is_empty() {
            return VerifyOutcome::NotVerified;
        }
        let body = serde_json::json!({ "intent": intent, "controllerId": controller_id }).to_string();
        let req = self.request("verify", body);
        match hbb_common::tokio::time::timeout(timeout, self.transport.post(&req)).await {
            Ok(Ok(res)) => parse_verify(res.status, &res.body),
            Ok(Err(_)) | Err(_) => VerifyOutcome::NotVerified,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    Done,
    /// No network, 5xx, 429: sign again and retry later.
    Retry,
    /// The server rejected it for good (4xx): do not retry.
    Drop,
}

pub fn classify(res: &Result<HttpResponse, String>) -> Sent {
    match res {
        Ok(r) if r.status == 200 => Sent::Done,
        Ok(r) if r.status == 429 || r.status >= 500 => Sent::Retry,
        Ok(_) => Sent::Drop,
        Err(_) => Sent::Retry,
    }
}

pub const QUEUE_MAX: usize = 100;

/// Session reports waiting for the network (bodies only; each attempt is signed afresh).
#[derive(Default)]
pub struct SessionQueue {
    items: std::collections::VecDeque<String>,
}

impl SessionQueue {
    pub fn push(&mut self, body: String) {
        self.items.push_back(body);
        while self.items.len() > QUEUE_MAX {
            self.items.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn front(&self) -> Option<&String> {
        self.items.front()
    }

    pub fn pop_front(&mut self) -> Option<String> {
        self.items.pop_front()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.items).unwrap_or_else(|_| "[]".to_owned())
    }

    pub fn from_json(s: &str) -> Self {
        let mut q = Self::default();
        for body in serde_json::from_str::<Vec<String>>(s).unwrap_or_default() {
            q.push(body);
        }
        q
    }
}

/// 1 s, 2 s, 4 s, ... up to 300 s.
#[derive(Default)]
pub struct Backoff {
    step: u32,
}

impl Backoff {
    pub fn next_delay(&mut self) -> Duration {
        let secs = (1u64 << self.step.min(20)).min(300);
        self.step = self.step.saturating_add(1);
        Duration::from_secs(secs)
    }

    pub fn reset(&mut self) {
        self.step = 0;
    }
}

pub enum SessionEvent {
    Start { conn_id: String, controller_id: String, method: &'static str, intent: Option<String> },
    End { conn_id: String, controller_id: String, method: &'static str, duration_sec: u64, reason: String },
}

fn cut(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

pub fn session_body(e: &SessionEvent) -> String {
    match e {
        SessionEvent::Start { conn_id, controller_id, method, intent } => {
            let mut v = serde_json::json!({
                "event": "start", "connId": cut(conn_id, 40), "controllerId": cut(controller_id, 16), "method": method,
            });
            if let Some(token) = intent {
                v["intent"] = token.clone().into();
            }
            v.to_string()
        }
        SessionEvent::End { conn_id, controller_id, method, duration_sec, reason } => serde_json::json!({
            "event": "end", "connId": cut(conn_id, 40), "controllerId": cut(controller_id, 16), "method": method,
            "durationSec": (*duration_sec).min(604_800), "endReason": cut(reason, 40),
        })
        .to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelloResult {
    Ok,
    /// 409 key_mismatch: the number is registered with another key; an administrator must reset it.
    KeyMismatch,
    Failed,
}

pub fn parse_hello(status: u16, body: &str) -> HelloResult {
    match status {
        200 => HelloResult::Ok,
        409 if serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| text(&v, "error"))
            .as_deref()
            == Some("key_mismatch") =>
        {
            HelloResult::KeyMismatch
        }
        _ => HelloResult::Failed,
    }
}

pub fn hello_body(id: &str, pk: &sign::PublicKey, hostname: &str, os: &str, version: &str) -> String {
    serde_json::json!({
        "rustdeskId": id,
        "publicKey": super::sign::public_key_b64(pk),
        "hostname": cut(hostname, 120),
        "os": cut(os, 60),
        "appVersion": cut(version, 32),
    })
    .to_string()
}

pub fn state_body(on: bool) -> String {
    serde_json::json!({ "unattended": on }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::tokio;
    use std::sync::Mutex;

    fn keypair() -> (sign::PublicKey, sign::SecretKey) {
        sign::keypair_from_seed(&sign::Seed::from_slice(&[7u8; 32]).unwrap())
    }

    fn now() -> i64 {
        1760000000
    }

    enum Reply {
        Ok(u16, &'static str),
        Err,
        Hang,
    }

    struct Fake {
        reply: Reply,
        seen: Mutex<Vec<SignedRequest>>,
    }

    impl Fake {
        fn new(reply: Reply) -> Self {
            Fake { reply, seen: Mutex::new(Vec::new()) }
        }
    }

    impl Transport for Fake {
        fn post<'a>(&'a self, req: &'a SignedRequest) -> TransportFuture<'a> {
            self.seen.lock().unwrap().push(req.clone());
            Box::pin(async move {
                match &self.reply {
                    Reply::Ok(status, body) => Ok(HttpResponse { status: *status, body: (*body).to_owned() }),
                    Reply::Err => Err("connection refused".to_owned()),
                    Reply::Hang => {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        Err("never".to_owned())
                    }
                }
            })
        }
    }

    fn client(reply: Reply) -> DeviceClient<Fake> {
        DeviceClient {
            api: "https://connect.incorrectus.com".into(),
            device_id: "123456789".into(),
            sk: keypair().1,
            transport: Fake::new(reply),
            clock: now,
        }
    }

    const OK_ATTENDED: &str = r#"{"ok":true,"consultant":{"name":"Jan Kowalski","email":"jan@incorrectus.com"},"company":"Incorrectus","ticket":{"id":1234,"title":"Drukarka"},"mode":"attended","unattendedAllowed":false}"#;
    const OK_UNATTENDED: &str = r#"{"ok":true,"consultant":{"name":"Jan","email":"j@x.pl"},"company":"Incorrectus","ticket":null,"mode":"unattended","unattendedAllowed":true}"#;

    #[test]
    fn build_request_signs_message_for_the_endpoint_and_sets_headers_data() {
        let (pk, sk) = keypair();
        let r = build_request(&sk, "https://connect.incorrectus.com/", "state", Some("123456789"), 1760000000, r#"{"unattended":true}"#.into());
        assert_eq!(r.url, "https://connect.incorrectus.com/api/remote/device/state");
        assert_eq!(r.device.as_deref(), Some("123456789"));
        assert_eq!(r.signature.len(), 128);
        assert!(r.signature.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        let sig = sign::Signature::from_bytes(&hex::decode(&r.signature).unwrap()).unwrap();
        assert!(sign::verify_detached(&sig, br#"iw1.state.1760000000.{"unattended":true}"#, &pk));
        // bound to the route: the same signature does not verify for `session`
        assert!(!sign::verify_detached(&sig, br#"iw1.session.1760000000.{"unattended":true}"#, &pk));
    }

    #[test]
    fn retry_signs_again_with_the_current_time() {
        let (_, sk) = keypair();
        let a = build_request(&sk, "https://c.pl", "session", Some("1"), 1760000000, "{}".into());
        let b = build_request(&sk, "https://c.pl", "session", Some("1"), 1760000001, "{}".into());
        assert_ne!(a.signature, b.signature);
        assert_eq!(b.ts, 1760000001);
    }

    #[test]
    fn request_debug_output_has_no_body() {
        let (_, sk) = keypair();
        let r = build_request(&sk, "https://c.pl", "verify", Some("1"), 1, r#"{"intent":"SECRET-TOKEN"}"#.into());
        assert!(!format!("{r:?}").contains("SECRET-TOKEN"));
    }

    #[test]
    fn parse_verify_reads_a_clean_answer() {
        let VerifyOutcome::Verified(v) = parse_verify(200, OK_ATTENDED) else { panic!("not verified") };
        assert_eq!(v.consultant_name, "Jan Kowalski");
        assert_eq!(v.company, "Incorrectus");
        assert_eq!(v.ticket_id, Some(1234));
        assert_eq!(v.ticket_title.as_deref(), Some("Drukarka"));
        assert!(!v.unattended_mode && !v.unattended_allowed);
        let VerifyOutcome::Verified(u) = parse_verify(200, OK_UNATTENDED) else { panic!("not verified") };
        assert!(u.unattended_mode && u.unattended_allowed);
        assert_eq!(u.ticket_id, None);
    }

    #[test]
    fn parse_verify_everything_else_is_not_verified() {
        for (status, body) in [
            (200, r#"{"ok":false,"reason":"used"}"#),
            (200, r#"{"ok":false,"reason":"expired"}"#),
            (200, "not json"),
            (200, ""),
            (200, r#"{"ok":true}"#),
            (200, r#"{"ok":"true","consultant":{"name":"a","email":"b"},"company":"c","mode":"attended"}"#),
            (401, OK_ATTENDED),
            (500, OK_ATTENDED),
            (429, OK_ATTENDED),
            (302, OK_ATTENDED),
        ] {
            assert_eq!(parse_verify(status, body), VerifyOutcome::NotVerified, "{status} {body}");
        }
    }

    #[test]
    fn missing_unattended_flag_means_not_allowed() {
        let body = r#"{"ok":true,"consultant":{"name":"Jan","email":"j@x.pl"},"company":"I","ticket":null,"mode":"unattended"}"#;
        let VerifyOutcome::Verified(v) = parse_verify(200, body) else { panic!("not verified") };
        assert!(v.unattended_mode);
        assert!(!v.unattended_allowed);
    }

    #[tokio::test]
    async fn verify_sends_a_signed_request_and_returns_the_consultant() {
        let c = client(Reply::Ok(200, OK_ATTENDED));
        let out = c.verify("TOKEN-TOKEN-TOKEN-TOKEN-TOKEN", "555666777", VERIFY_TIMEOUT).await;
        assert!(matches!(out, VerifyOutcome::Verified(_)));
        let seen = c.transport.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].url, "https://connect.incorrectus.com/api/remote/device/verify");
        assert_eq!(seen[0].device.as_deref(), Some("123456789"));
        let body: serde_json::Value = serde_json::from_str(&seen[0].body).unwrap();
        assert_eq!(body["intent"], "TOKEN-TOKEN-TOKEN-TOKEN-TOKEN");
        assert_eq!(body["controllerId"], "555666777");
        let sig = sign::Signature::from_bytes(&hex::decode(&seen[0].signature).unwrap()).unwrap();
        let msg = format!("iw1.verify.{}.{}", seen[0].ts, seen[0].body);
        assert!(sign::verify_detached(&sig, msg.as_bytes(), &keypair().0));
    }

    #[tokio::test]
    async fn verify_timeout_means_not_verified() {
        let c = client(Reply::Hang);
        let started = std::time::Instant::now();
        let out = c.verify("TOKEN-TOKEN-TOKEN-TOKEN-TOKEN", "1", Duration::from_millis(50)).await;
        assert_eq!(out, VerifyOutcome::NotVerified);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn verify_network_error_means_not_verified() {
        let c = client(Reply::Err);
        assert_eq!(c.verify("TOKEN-TOKEN-TOKEN-TOKEN-TOKEN", "1", VERIFY_TIMEOUT).await, VerifyOutcome::NotVerified);
    }

    #[tokio::test]
    async fn verify_without_intent_does_not_call_the_server() {
        let c = client(Reply::Ok(200, OK_ATTENDED));
        assert_eq!(c.verify("", "1", VERIFY_TIMEOUT).await, VerifyOutcome::NotVerified);
        assert!(c.transport.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn classify_decides_retry_or_drop() {
        assert_eq!(classify(&Ok(HttpResponse { status: 200, body: String::new() })), Sent::Done);
        for status in [429u16, 500, 502, 503] {
            assert_eq!(classify(&Ok(HttpResponse { status, body: String::new() })), Sent::Retry, "{status}");
        }
        for status in [400u16, 401, 404, 409] {
            assert_eq!(classify(&Ok(HttpResponse { status, body: String::new() })), Sent::Drop, "{status}");
        }
        assert_eq!(classify(&Err("timeout".into())), Sent::Retry);
    }

    #[test]
    fn queue_keeps_at_most_100_entries_dropping_the_oldest() {
        let mut q = SessionQueue::default();
        for i in 0..105 {
            q.push(format!("{{\"n\":{i}}}"));
        }
        assert_eq!(q.len(), 100);
        assert_eq!(q.front().unwrap(), "{\"n\":5}");
    }

    #[test]
    fn queue_survives_a_save_and_load() {
        let mut q = SessionQueue::default();
        q.push("{\"a\":1}".into());
        q.push("{\"b\":2}".into());
        let back = SessionQueue::from_json(&q.to_json());
        assert_eq!(back.len(), 2);
        assert_eq!(back.front().unwrap(), "{\"a\":1}");
        assert_eq!(SessionQueue::from_json("garbage").len(), 0);
    }

    #[test]
    fn backoff_doubles_from_one_second_and_resets() {
        let mut b = Backoff::default();
        let secs: Vec<u64> = (0..5).map(|_| b.next_delay().as_secs()).collect();
        assert_eq!(secs, vec![1, 2, 4, 8, 16]);
        for _ in 0..20 {
            b.next_delay();
        }
        assert_eq!(b.next_delay().as_secs(), 300);
        b.reset();
        assert_eq!(b.next_delay().as_secs(), 1);
    }

    #[test]
    fn session_body_omits_absent_fields() {
        let start = session_body(&SessionEvent::Start {
            conn_id: "c1".into(),
            controller_id: "555".into(),
            method: "click",
            intent: Some("TOKEN".into()),
        });
        let v: serde_json::Value = serde_json::from_str(&start).unwrap();
        assert_eq!(v["event"], "start");
        assert_eq!(v["connId"], "c1");
        assert_eq!(v["method"], "click");
        assert_eq!(v["intent"], "TOKEN");
        let end = session_body(&SessionEvent::End {
            conn_id: "c1".into(),
            controller_id: "555".into(),
            method: "password",
            duration_sec: 61,
            reason: "closed".into(),
        });
        let v: serde_json::Value = serde_json::from_str(&end).unwrap();
        assert_eq!(v["event"], "end");
        assert_eq!(v["durationSec"], 61);
        assert_eq!(v["endReason"], "closed");
        assert!(v.get("intent").is_none());
    }

    #[test]
    fn hello_answer_is_read_for_ok_and_key_mismatch() {
        assert_eq!(parse_hello(200, r#"{"ok":true,"org":{"name":"Incorrectus"}}"#), HelloResult::Ok);
        assert_eq!(parse_hello(409, r#"{"error":"key_mismatch","message":"x"}"#), HelloResult::KeyMismatch);
        assert_eq!(parse_hello(409, r#"{"error":"other"}"#), HelloResult::Failed);
        assert_eq!(parse_hello(500, ""), HelloResult::Failed);
    }

    #[test]
    fn hello_body_carries_the_public_key_and_state_body_the_flag() {
        let (pk, _) = keypair();
        let v: serde_json::Value =
            serde_json::from_str(&hello_body("123456789", &pk, "PC-JAN", "Windows 11", "1.4.9-iw.1")).unwrap();
        assert_eq!(v["rustdeskId"], "123456789");
        assert_eq!(v["publicKey"], super::super::sign::public_key_b64(&pk));
        assert_eq!(v["appVersion"], "1.4.9-iw.1");
        assert_eq!(state_body(true), r#"{"unattended":true}"#);
    }

    #[test]
    fn hello_fields_are_cut_to_the_server_limits() {
        let (pk, _) = keypair();
        let long = "x".repeat(300);
        let v: serde_json::Value = serde_json::from_str(&hello_body("1", &pk, &long, &long, &long)).unwrap();
        assert_eq!(v["hostname"].as_str().unwrap().chars().count(), 120);
        assert_eq!(v["os"].as_str().unwrap().chars().count(), 60);
        assert_eq!(v["appVersion"].as_str().unwrap().chars().count(), 32);
    }
}
