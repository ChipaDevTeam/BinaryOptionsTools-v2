//! Free "try demo" (guest) session support for PocketOption.
//!
//! PocketOption's "Try demo" flow never contacts a token endpoint: the guest
//! client **generates** a 10-character id locally, caches it in `localStorage`
//! under `po-td-token` and echoes it in the auth frame:
//!
//! ```text
//! 42["auth",{"token":"9PRxPzq6mQ","balance":50000,"isFastHistory":true}]
//! ```
//!
//! [`Ssid::parse`] accepts that payload directly, so this module only has to mint
//! a token the way the browser does.
//!
//! Note: as of October 2026 the demo servers accept the frame but never confirm
//! the session, so [`crate::pocketoption::PocketOption::new_try_demo`] reports
//! `NotAuthorized` instead of handing back a client the broker ignores.

use rand::RngExt;

use crate::pocketoption::error::{PocketError, PocketResult};
use crate::pocketoption::ssid::{Ssid, TryDemo};

/// Alphabet used by PocketOption's own guest-token generator (a nanoid).
///
/// Copied verbatim from the platform's `try-demo` bundle. Bytes are reduced with
/// `& 63` before indexing, exactly like the original.
const TOKEN_ALPHABET: &[u8; 64] =
    b"useandom-26T198340PX75pxJACKVERYMINDBUSHWOLF_GQZbfghjklqvwyzrict";

/// Length of a PocketOption guest token.
const TOKEN_LENGTH: usize = 10;

/// Starting virtual balance the platform gives a fresh guest session.
pub const DEFAULT_TRY_DEMO_BALANCE: u64 = 50_000;

/// Mints a fresh guest token, mirroring PocketOption's own generator.
pub fn generate_try_demo_token() -> String {
    let bytes: [u8; TOKEN_LENGTH] = rand::rng().random();
    bytes
        .iter()
        .map(|b| TOKEN_ALPHABET[(*b & 63) as usize] as char)
        .collect()
}

/// Builds the guest auth payload for an existing `token`.
///
/// Handy when replaying a token the browser already cached
/// (`localStorage['po-td-token']`).
pub fn try_demo_ssid_from_token(token: impl ToString, balance: u64) -> PocketResult<Ssid> {
    if balance == 0 {
        return Err(PocketError::General(
            "Try-demo balance must be at least 1".into(),
        ));
    }

    let guest = TryDemo::new(token)?
        .with_balance(balance)
        .with_fast_history(true);
    Ok(Ssid::TryDemo(guest))
}

/// Builds the guest auth payload for replaying a cached `token`.
///
/// No balance is sent: a replayed guest keeps the balance the server already
/// associated with its token, and sending a synthetic value would reset it.
pub fn replay_try_demo_ssid(token: impl ToString) -> PocketResult<Ssid> {
    let guest = TryDemo::new(token)?.with_fast_history(true);
    Ok(Ssid::TryDemo(guest))
}

/// Mints a token and builds the guest auth payload for `balance`.
pub fn generate_try_demo_ssid(balance: u64) -> PocketResult<Ssid> {
    try_demo_ssid_from_token(generate_try_demo_token(), balance)
}

/// Builds a guest [`Ssid`] using the platform's default demo balance.
pub fn default_try_demo_ssid() -> PocketResult<Ssid> {
    generate_try_demo_ssid(DEFAULT_TRY_DEMO_BALANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_token_matches_the_platform_format() {
        for _ in 0..50 {
            let token = generate_try_demo_token();
            assert_eq!(token.len(), TOKEN_LENGTH);
            assert!(
                token.bytes().all(|b| TOKEN_ALPHABET.contains(&b)),
                "token `{token}` used a character outside the platform alphabet"
            );
        }
    }

    #[test]
    fn generated_tokens_are_not_reused() {
        let first = generate_try_demo_token();
        assert!(
            (0..20).all(|_| generate_try_demo_token() != first),
            "generator repeated a token"
        );
    }

    #[test]
    fn generated_ssid_matches_the_captured_auth_frame() {
        let ssid = default_try_demo_ssid().unwrap();
        let frame = ssid.to_string();

        assert!(ssid.demo(), "guest sessions are demo sessions");
        assert_eq!(ssid.session_id().len(), TOKEN_LENGTH);
        assert!(frame.starts_with("42[\"auth\",{\"token\":\""));
        assert!(frame.ends_with("\",\"balance\":50000,\"isFastHistory\":true}]"));

        // Round-trips back into an equivalent guest session.
        let reparsed = Ssid::parse(&frame).unwrap();
        assert_eq!(reparsed.session_id(), ssid.session_id());
        assert_eq!(reparsed.to_string(), frame);
    }

    #[test]
    fn replayed_token_omits_balance() {
        let ssid = replay_try_demo_ssid("9PRxPzq6mQ").unwrap();
        let frame = ssid.to_string();

        // Replaying must not reset the guest balance with a synthetic value.
        assert!(!frame.contains("\"balance\""), "replay frame was {frame}");
        assert!(frame.contains("\"token\":\"9PRxPzq6mQ\""));
        assert_eq!(ssid.session_id(), "9PRxPzq6mQ");
        assert!(ssid.demo());
    }

    #[test]
    fn custom_balance_is_recorded() {
        let ssid = generate_try_demo_ssid(1_000).unwrap();
        assert!(ssid.to_string().contains("\"balance\":1000"));
    }

    #[test]
    fn zero_balance_is_rejected() {
        assert!(generate_try_demo_ssid(0).is_err());
    }
}
