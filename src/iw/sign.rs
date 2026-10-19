//! Ed25519 signing of device API requests (docs/connect/remote-api.md section 2).

use hbb_common::sodiumoxide::crypto::sign;

use hbb_common::base64::{engine::general_purpose::STANDARD, Engine};

/// `"iw1." + endpoint + "." + ts + "." + body`, the exact bytes the server verifies.
pub fn signed_message(endpoint: &str, ts: i64, body: &str) -> Vec<u8> {
    format!("iw1.{endpoint}.{ts}.{body}").into_bytes()
}

/// Lowercase hex of the detached Ed25519 signature (128 characters).
pub fn sign_request(sk: &sign::SecretKey, endpoint: &str, ts: i64, body: &str) -> String {
    let sig = sign::sign_detached(&signed_message(endpoint, ts, body), sk);
    hex::encode(sig.to_bytes())
}

/// Standard base64 with padding of the 32 raw public key bytes (44 characters).
pub fn public_key_b64(pk: &sign::PublicKey) -> String {
    STANDARD.encode(pk.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::base64::{engine::general_purpose::STANDARD, Engine};

    // The shared vector from docs/connect/remote-api.md section 2.
    const SEED_B64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
    const PUBLIC_B64: &str = "A6EHv/POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=";
    const BODY: &str = r#"{"unattended":true}"#;
    const SIGNATURE: &str = "86c03c0d19b7029c4d4945294a2d541b34d5854a0e2ff8e440dc1b18363abce9b95b565c00d37929c5d97eca04cfdebf57e427e76b2b5e9c09ee6ce4b2270e0e";

    fn vector_key() -> (sign::PublicKey, sign::SecretKey) {
        let seed = STANDARD.decode(SEED_B64).unwrap();
        let seed = sign::Seed::from_slice(&seed).unwrap();
        sign::keypair_from_seed(&seed)
    }

    #[test]
    fn sign_request_matches_server_vector() {
        let (pk, sk) = vector_key();
        assert_eq!(public_key_b64(&pk), PUBLIC_B64);
        assert_eq!(
            signed_message("state", 1760000000, BODY),
            br#"iw1.state.1760000000.{"unattended":true}"#.to_vec()
        );
        assert_eq!(sign_request(&sk, "state", 1760000000, BODY), SIGNATURE);
    }
}
