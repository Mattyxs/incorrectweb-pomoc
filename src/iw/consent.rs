//! Consent policy of the controlled computer: when a connection needs a click, a password, or neither.
//! Fail closed: anything that is not a clean "verified" answer from the server is "not verified".

use std::time::Duration;

/// How long a click may be awaited when the consultant tried to get in without one.
pub const UNATTENDED_FALLBACK_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedInfo {
    pub consultant_name: String,
    pub consultant_email: String,
    pub company: String,
    pub ticket_id: Option<u64>,
    pub ticket_title: Option<String>,
    /// The intent asked for unattended access.
    pub unattended_mode: bool,
    /// The server confirmed the consultant may skip the click (only meaningful with `unattended_mode`).
    pub unattended_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyOutcome {
    Verified(VerifiedInfo),
    /// No intent, `ok:false`, timeout, network error, HTTP error, broken answer: all the same.
    NotVerified,
}

/// What the controller proved with its password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credential {
    /// The login carried no password.
    None,
    Temporary,
    Permanent,
    Wrong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Let the controller in without a click (unattended access, every condition met).
    AuthorizeUnattended,
    /// Not verified and no password yet: ask for the one-time password.
    AskPassword,
    WrongPassword,
    /// Show the consent window; connect only after "Allow".
    Click {
        verified: bool,
        /// Reject when nobody clicks within this time.
        wait_limit: Option<Duration>,
        /// `click` (verified link) or `password` (password proven, then click).
        method: &'static str,
    },
}

pub fn decide(verify: &VerifyOutcome, credential: Credential, local_unattended_enabled: bool) -> Decision {
    let password_proven = matches!(credential, Credential::Temporary | Credential::Permanent);
    let method = if password_proven { "password" } else { "click" };
    match verify {
        VerifyOutcome::Verified(v) => {
            if v.unattended_mode && v.unattended_allowed && local_unattended_enabled {
                match credential {
                    Credential::Permanent => return Decision::AuthorizeUnattended,
                    // the consultant still has to prove the permanent password
                    Credential::None => return Decision::AskPassword,
                    Credential::Wrong => return Decision::WrongPassword,
                    Credential::Temporary => {}
                }
            }
            // The link vouches for the consultant; the password content is irrelevant here.
            let tried_unattended = v.unattended_mode || credential == Credential::Permanent;
            Decision::Click {
                verified: true,
                wait_limit: tried_unattended.then_some(UNATTENDED_FALLBACK_WAIT),
                method,
            }
        }
        VerifyOutcome::NotVerified => match credential {
            Credential::None => Decision::AskPassword,
            Credential::Wrong => Decision::WrongPassword,
            Credential::Temporary => Decision::Click { verified: false, wait_limit: None, method },
            Credential::Permanent => Decision::Click {
                verified: false,
                wait_limit: Some(UNATTENDED_FALLBACK_WAIT),
                method,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(unattended_mode: bool, unattended_allowed: bool) -> VerifyOutcome {
        VerifyOutcome::Verified(VerifiedInfo {
            consultant_name: "Jan Kowalski".into(),
            consultant_email: "jan@incorrectus.com".into(),
            company: "Incorrectus".into(),
            ticket_id: Some(1234),
            ticket_title: Some("Drukarka".into()),
            unattended_mode,
            unattended_allowed,
        })
    }

    fn click(verified: bool, wait: Option<Duration>, method: &'static str) -> Decision {
        Decision::Click { verified, wait_limit: wait, method }
    }

    #[test]
    fn verified_attended_link_needs_only_the_click() {
        assert_eq!(decide(&info(false, false), Credential::None, false), click(true, None, "click"));
        // unattended switched on in the program does not matter for an attended intent
        assert_eq!(decide(&info(false, false), Credential::None, true), click(true, None, "click"));
    }

    #[test]
    fn manual_id_without_verification_asks_for_the_one_time_password() {
        assert_eq!(decide(&VerifyOutcome::NotVerified, Credential::None, false), Decision::AskPassword);
        assert_eq!(decide(&VerifyOutcome::NotVerified, Credential::Wrong, false), Decision::WrongPassword);
    }

    #[test]
    fn manual_id_with_the_one_time_password_still_needs_the_click() {
        assert_eq!(
            decide(&VerifyOutcome::NotVerified, Credential::Temporary, false),
            click(false, None, "password")
        );
    }

    #[test]
    fn unattended_requires_allowed_flag_enabled_program_and_permanent_password() {
        assert_eq!(decide(&info(true, true), Credential::Permanent, true), Decision::AuthorizeUnattended);
    }

    #[test]
    fn unattended_denied_without_allowed_flag() {
        // unattended_denied_without_allowed_flag: falls back to a click that must come within 30 s
        assert_eq!(
            decide(&info(true, false), Credential::Permanent, true),
            click(true, Some(UNATTENDED_FALLBACK_WAIT), "password")
        );
    }

    #[test]
    fn unattended_denied_on_network_error() {
        // unattended_denied_on_network_error: the verify failed, so it is NotVerified
        let d = decide(&VerifyOutcome::NotVerified, Credential::Permanent, true);
        assert_eq!(d, click(false, Some(UNATTENDED_FALLBACK_WAIT), "password"));
        assert_ne!(d, Decision::AuthorizeUnattended);
    }

    #[test]
    fn unattended_denied_when_the_program_has_it_switched_off() {
        let d = decide(&info(true, true), Credential::Permanent, false);
        assert_ne!(d, Decision::AuthorizeUnattended);
        assert_eq!(d, click(true, Some(UNATTENDED_FALLBACK_WAIT), "password"));
    }

    #[test]
    fn unattended_denied_with_the_one_time_password_or_no_password() {
        assert_ne!(decide(&info(true, true), Credential::Temporary, true), Decision::AuthorizeUnattended);
        assert_ne!(decide(&info(true, true), Credential::None, true), Decision::AuthorizeUnattended);
        assert_ne!(decide(&info(true, true), Credential::Wrong, true), Decision::AuthorizeUnattended);
    }

    #[test]
    fn allowed_flag_without_unattended_mode_in_the_intent_is_ignored() {
        assert_ne!(decide(&info(false, true), Credential::Permanent, true), Decision::AuthorizeUnattended);
    }

    #[test]
    fn allowed_unattended_intent_asks_for_the_permanent_password_first() {
        assert_eq!(decide(&info(true, true), Credential::None, true), Decision::AskPassword);
        assert_eq!(decide(&info(true, true), Credential::Wrong, true), Decision::WrongPassword);
    }

    #[test]
    fn unattended_intent_that_cannot_be_unattended_waits_for_click_with_limit() {
        assert_eq!(
            decide(&info(true, false), Credential::None, true),
            click(true, Some(UNATTENDED_FALLBACK_WAIT), "click")
        );
        assert_eq!(
            decide(&info(true, true), Credential::None, false),
            click(true, Some(UNATTENDED_FALLBACK_WAIT), "click")
        );
    }

    #[test]
    fn verified_intent_ignores_a_wrong_password_but_never_authorizes_by_it() {
        let d = decide(&info(false, false), Credential::Wrong, false);
        assert_eq!(d, click(true, None, "click"));
    }
}
