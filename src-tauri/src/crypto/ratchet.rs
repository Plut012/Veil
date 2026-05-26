use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

use crate::crypto::engine::{decrypt, encrypt, CryptoError};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const VERSION_RATCHET: u8 = 0x01;
pub const MAX_SKIP: u64 = 100;
const COUNTER_SIZE: usize = 8; // u64 big-endian
const HEADER_SIZE: usize = 1 + COUNTER_SIZE; // version + counter
const MIN_SEALED_LEN: usize = HEADER_SIZE + 24 + 16; // header + nonce + MAC

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetState {
    #[serde(with = "crate::crypto::keys::base64_serde")]
    pub send_chain_key: [u8; 32],
    #[serde(with = "crate::crypto::keys::base64_serde")]
    pub recv_chain_key: [u8; 32],
    pub send_counter: u64,
    pub recv_counter: u64,
    #[serde(default)]
    pub skipped_keys: Vec<SkippedKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedKey {
    pub counter: u64,
    #[serde(with = "crate::crypto::keys::base64_serde")]
    pub key: [u8; 32],
}

#[derive(Debug, thiserror::Error)]
pub enum RatchetError {
    #[error("unknown ratchet version: {0}")]
    UnknownVersion(u8),
    #[error("ratcheted message too short")]
    MessageTooShort,
    #[error("message already consumed or expired")]
    DuplicateOrExpired,
    #[error("too many skipped messages (>{MAX_SKIP})")]
    TooManySkipped,
    #[error("decryption failed: {0}")]
    DecryptionFailed(#[from] CryptoError),
}

// ---------------------------------------------------------------------------
// Core derivation
// ---------------------------------------------------------------------------

/// Derive `(next_chain_key, message_key)` from the current chain key.
pub fn advance_chain(chain_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let hk = Hkdf::<Sha256>::new(None, chain_key);
    let mut output = [0u8; 64];
    hk.expand(b"veil-chain-advance", &mut output)
        .expect("64 bytes is valid for HKDF-SHA256");

    let mut next_chain_key = [0u8; 32];
    let mut message_key = [0u8; 32];
    next_chain_key.copy_from_slice(&output[..32]);
    message_key.copy_from_slice(&output[32..]);
    output.zeroize();

    (next_chain_key, message_key)
}

/// Initialize a ratchet from the shared root key established during pairing.
/// Initiator and joiner get mirrored send/receive chains.
pub fn init_ratchet(root_key: &[u8; 32], is_initiator: bool) -> RatchetState {
    let hk = Hkdf::<Sha256>::new(None, root_key);

    let mut chain_a = [0u8; 32];
    let mut chain_b = [0u8; 32];
    hk.expand(b"veil-ratchet-chain-a", &mut chain_a)
        .expect("32 bytes is valid for HKDF-SHA256");
    hk.expand(b"veil-ratchet-chain-b", &mut chain_b)
        .expect("32 bytes is valid for HKDF-SHA256");

    let (send_chain_key, recv_chain_key) = if is_initiator {
        (chain_a, chain_b)
    } else {
        (chain_b, chain_a)
    };

    RatchetState {
        send_chain_key,
        recv_chain_key,
        send_counter: 0,
        recv_counter: 0,
        skipped_keys: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Encrypt / Decrypt
// ---------------------------------------------------------------------------

/// Encrypt with the ratchet: advance the send chain, encrypt with the
/// disposable message key, prepend version + counter header.
///
/// Wire format: `[0x01][counter 8B BE][nonce 24B][ciphertext+MAC]`
pub fn ratchet_encrypt(ratchet: &mut RatchetState, plaintext: &[u8]) -> Vec<u8> {
    let (next_chain_key, mut message_key) = advance_chain(&ratchet.send_chain_key);
    ratchet.send_chain_key.zeroize();
    ratchet.send_chain_key = next_chain_key;

    let sealed = encrypt(plaintext, &message_key);
    message_key.zeroize();

    let counter = ratchet.send_counter;
    ratchet.send_counter += 1;

    let mut output = Vec::with_capacity(HEADER_SIZE + sealed.len());
    output.push(VERSION_RATCHET);
    output.extend_from_slice(&counter.to_be_bytes());
    output.extend_from_slice(&sealed);
    output
}

/// Decrypt a ratcheted message: parse header, resolve the message key
/// (from current chain, skipped cache, or by fast-forwarding), decrypt.
pub fn ratchet_decrypt(
    ratchet: &mut RatchetState,
    sealed: &[u8],
) -> Result<Vec<u8>, RatchetError> {
    if sealed.len() < MIN_SEALED_LEN {
        return Err(RatchetError::MessageTooShort);
    }

    let version = sealed[0];
    if version != VERSION_RATCHET {
        return Err(RatchetError::UnknownVersion(version));
    }

    let counter = u64::from_be_bytes(sealed[1..9].try_into().unwrap());
    let payload = &sealed[HEADER_SIZE..];

    let mut message_key = resolve_message_key(ratchet, counter)?;
    let plaintext = decrypt(payload, &message_key)?;
    message_key.zeroize();

    // Trim skipped cache if over limit
    while ratchet.skipped_keys.len() > MAX_SKIP as usize {
        let mut removed = ratchet.skipped_keys.remove(0);
        removed.key.zeroize();
    }

    Ok(plaintext)
}

/// Resolve the message key for a given counter value.
fn resolve_message_key(
    ratchet: &mut RatchetState,
    counter: u64,
) -> Result<[u8; 32], RatchetError> {
    if counter < ratchet.recv_counter {
        // Look in skipped cache
        let idx = ratchet
            .skipped_keys
            .iter()
            .position(|sk| sk.counter == counter);
        match idx {
            Some(i) => {
                let sk = ratchet.skipped_keys.remove(i);
                Ok(sk.key)
            }
            None => Err(RatchetError::DuplicateOrExpired),
        }
    } else if counter == ratchet.recv_counter {
        // Expected next message
        let (next_chain, message_key) = advance_chain(&ratchet.recv_chain_key);
        ratchet.recv_chain_key.zeroize();
        ratchet.recv_chain_key = next_chain;
        ratchet.recv_counter += 1;
        Ok(message_key)
    } else {
        // Fast-forward: counter > recv_counter
        let skip_count = counter - ratchet.recv_counter;
        if skip_count > MAX_SKIP {
            return Err(RatchetError::TooManySkipped);
        }

        // Cache skipped message keys
        for _ in 0..skip_count {
            let (next_chain, skipped_key) = advance_chain(&ratchet.recv_chain_key);
            ratchet.recv_chain_key.zeroize();
            ratchet.recv_chain_key = next_chain;
            ratchet.skipped_keys.push(SkippedKey {
                counter: ratchet.recv_counter,
                key: skipped_key,
            });
            ratchet.recv_counter += 1;
        }

        // Now recv_counter == counter — derive this message's key
        let (next_chain, message_key) = advance_chain(&ratchet.recv_chain_key);
        ratchet.recv_chain_key.zeroize();
        ratchet.recv_chain_key = next_chain;
        ratchet.recv_counter += 1;
        Ok(message_key)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::generate_key;

    fn init() {
        sodiumoxide::init().expect("sodiumoxide::init() failed");
    }

    #[test]
    fn test_advance_chain_deterministic() {
        let key = [0xABu8; 32];
        let (next1, msg1) = advance_chain(&key);
        let (next2, msg2) = advance_chain(&key);
        assert_eq!(next1, next2);
        assert_eq!(msg1, msg2);
    }

    #[test]
    fn test_advance_chain_different_inputs() {
        let key_a = [0x01u8; 32];
        let key_b = [0x02u8; 32];
        let (next_a, msg_a) = advance_chain(&key_a);
        let (next_b, msg_b) = advance_chain(&key_b);
        assert_ne!(next_a, next_b);
        assert_ne!(msg_a, msg_b);
    }

    #[test]
    fn test_init_ratchet_role_symmetry() {
        init();
        let root = generate_key();
        let initiator = init_ratchet(&root, true);
        let joiner = init_ratchet(&root, false);

        assert_eq!(
            initiator.send_chain_key, joiner.recv_chain_key,
            "initiator send must equal joiner recv"
        );
        assert_eq!(
            initiator.recv_chain_key, joiner.send_chain_key,
            "initiator recv must equal joiner send"
        );
        assert_ne!(
            initiator.send_chain_key, initiator.recv_chain_key,
            "send and recv chains must differ"
        );
    }

    #[test]
    fn test_roundtrip_single_message() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        let plaintext = b"hello, forward secrecy!";
        let sealed = ratchet_encrypt(&mut sender, plaintext);
        let decrypted = ratchet_decrypt(&mut receiver, &sealed).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_roundtrip_multiple_messages() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        for i in 0..10u32 {
            let msg = format!("message {i}");
            let sealed = ratchet_encrypt(&mut sender, msg.as_bytes());
            let decrypted = ratchet_decrypt(&mut receiver, &sealed).unwrap();
            assert_eq!(decrypted, msg.as_bytes());
        }
        assert_eq!(sender.send_counter, 10);
        assert_eq!(receiver.recv_counter, 10);
    }

    #[test]
    fn test_bidirectional_messages() {
        init();
        let root = generate_key();
        let mut alice = init_ratchet(&root, true);
        let mut bob = init_ratchet(&root, false);

        // Alice → Bob
        let sealed = ratchet_encrypt(&mut alice, b"hi bob");
        let decrypted = ratchet_decrypt(&mut bob, &sealed).unwrap();
        assert_eq!(decrypted, b"hi bob");

        // Bob → Alice
        let sealed = ratchet_encrypt(&mut bob, b"hi alice");
        let decrypted = ratchet_decrypt(&mut alice, &sealed).unwrap();
        assert_eq!(decrypted, b"hi alice");

        // Alice → Bob again
        let sealed = ratchet_encrypt(&mut alice, b"how are you");
        let decrypted = ratchet_decrypt(&mut bob, &sealed).unwrap();
        assert_eq!(decrypted, b"how are you");
    }

    #[test]
    fn test_out_of_order_delivery() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        let msg0 = ratchet_encrypt(&mut sender, b"msg0");
        let msg1 = ratchet_encrypt(&mut sender, b"msg1");
        let msg2 = ratchet_encrypt(&mut sender, b"msg2");

        // Deliver out of order: 2, 0, 1
        assert_eq!(ratchet_decrypt(&mut receiver, &msg2).unwrap(), b"msg2");
        assert_eq!(ratchet_decrypt(&mut receiver, &msg0).unwrap(), b"msg0");
        assert_eq!(ratchet_decrypt(&mut receiver, &msg1).unwrap(), b"msg1");
    }

    #[test]
    fn test_duplicate_message_rejected() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        let sealed = ratchet_encrypt(&mut sender, b"once only");
        ratchet_decrypt(&mut receiver, &sealed).unwrap();

        let result = ratchet_decrypt(&mut receiver, &sealed);
        assert!(
            matches!(result, Err(RatchetError::DuplicateOrExpired)),
            "duplicate should fail: {result:?}"
        );
    }

    #[test]
    fn test_max_skip_exceeded() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        // Encrypt MAX_SKIP + 2 messages, only try to decrypt the last one
        for _ in 0..(MAX_SKIP + 2) {
            ratchet_encrypt(&mut sender, b"skip");
        }
        let last = ratchet_encrypt(&mut sender, b"too far");

        let result = ratchet_decrypt(&mut receiver, &last);
        assert!(
            matches!(result, Err(RatchetError::TooManySkipped)),
            "should reject: {result:?}"
        );
    }

    #[test]
    fn test_wrong_key_fails() {
        init();
        let root_a = generate_key();
        let root_b = generate_key();
        let mut sender = init_ratchet(&root_a, true);
        let mut receiver = init_ratchet(&root_b, false);

        let sealed = ratchet_encrypt(&mut sender, b"secret");
        let result = ratchet_decrypt(&mut receiver, &sealed);
        assert!(
            matches!(result, Err(RatchetError::DecryptionFailed(_))),
            "wrong key should fail: {result:?}"
        );
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        let mut sealed = ratchet_encrypt(&mut sender, b"important");
        // Tamper a byte in the ciphertext region (after header + nonce)
        let tamper_idx = HEADER_SIZE + 30;
        if tamper_idx < sealed.len() {
            sealed[tamper_idx] ^= 0xFF;
        }

        let result = ratchet_decrypt(&mut receiver, &sealed);
        assert!(
            matches!(result, Err(RatchetError::DecryptionFailed(_))),
            "tampered data should fail: {result:?}"
        );
    }

    #[test]
    fn test_version_byte_present() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);

        let sealed = ratchet_encrypt(&mut sender, b"test");
        assert_eq!(sealed[0], VERSION_RATCHET);
    }

    #[test]
    fn test_counter_monotonic() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);

        for expected in 0u64..5 {
            let sealed = ratchet_encrypt(&mut sender, b"x");
            let counter = u64::from_be_bytes(sealed[1..9].try_into().unwrap());
            assert_eq!(counter, expected);
        }
    }

    #[test]
    fn test_skipped_cache_trimmed() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        // Send MAX_SKIP messages, only decrypt the last to force caching
        let mut messages = Vec::new();
        for _ in 0..MAX_SKIP {
            messages.push(ratchet_encrypt(&mut sender, b"skip"));
        }
        let last = ratchet_encrypt(&mut sender, b"last");

        // Decrypting the last causes MAX_SKIP keys to be cached
        ratchet_decrypt(&mut receiver, &last).unwrap();
        assert!(
            receiver.skipped_keys.len() <= MAX_SKIP as usize,
            "cache should not exceed MAX_SKIP"
        );
    }

    #[test]
    fn test_message_too_short() {
        init();
        let root = generate_key();
        let mut receiver = init_ratchet(&root, false);

        let result = ratchet_decrypt(&mut receiver, &[0x01, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(
            matches!(result, Err(RatchetError::MessageTooShort)),
            "short message should fail: {result:?}"
        );
    }

    #[test]
    fn test_unknown_version_rejected() {
        init();
        let root = generate_key();
        let mut sender = init_ratchet(&root, true);
        let mut receiver = init_ratchet(&root, false);

        let mut sealed = ratchet_encrypt(&mut sender, b"test");
        sealed[0] = 0xFF; // corrupt version byte

        let result = ratchet_decrypt(&mut receiver, &sealed);
        assert!(
            matches!(result, Err(RatchetError::UnknownVersion(0xFF))),
            "unknown version should fail: {result:?}"
        );
    }

    #[test]
    fn test_ratchet_state_serde_roundtrip() {
        init();
        let root = generate_key();
        let state = init_ratchet(&root, true);

        let json = serde_json::to_string(&state).expect("serialize");
        let decoded: RatchetState = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded.send_chain_key, state.send_chain_key);
        assert_eq!(decoded.recv_chain_key, state.recv_chain_key);
        assert_eq!(decoded.send_counter, state.send_counter);
        assert_eq!(decoded.recv_counter, state.recv_counter);
    }
}
