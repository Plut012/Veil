# Future Feature: Absolute Mode (One-Time Pad Encryption)

**Status:** Shelved — revisit after Standard ratchet is proven
**Dependencies:** Symmetric ratchet (Phase 9) must be solid first
**Priority:** Low — vibey long-term goal, needs refinement before implementation

---

## Vision

A second encryption mode: **one-time pad (OTP)**. Information-theoretically unbreakable. Requires in-person pad exchange. Not a replacement for Standard — a companion for users who want the theoretical maximum.

**Philosophy:** Two modes, no middle option. Standard is excellent on its own. Absolute is for those who want provable security at the cost of convenience.

---

## Core Concept

- At pairing, users physically transfer a pad of CSPRNG output (split into send/receive halves).
- Each message consumes pad bytes: `ciphertext[i] = plaintext[i] XOR pad[offset + i]`.
- Consumed bytes are zeroed and fsynced before ciphertext is transmitted. **Never reused.**
- Authentication via Poly1305 MAC keyed from a separate consumed pad region (32 bytes/message).
- Pad is encrypted at rest using existing Argon2id-derived storage key.

---

## Open Questions (Discuss Before Building)

### 1. Pad Transfer Mechanism
The original plan proposed animated QR streaming for 5 MB. **The math doesn't work** — at ~1.2 KB per QR (error correction H, version 40), 5 MB requires ~4,100 codes. At a realistic 10 codes/second scan rate, that's ~7 minutes of perfect alignment. Options to explore:

- **Smaller pad** (~256 KB): ~200 codes, ~20 seconds. Enough for ~250K characters of text — months of messaging. Most practical for QR.
- **Local WiFi Direct / Bluetooth:** More bandwidth, more platform complexity.
- **USB drive / file transfer:** Simple but manual. "Bring a USB stick" feature.
- **NFC tap (if hardware supports):** Small payload but could bootstrap a local encrypted channel for the bulk transfer.

### 2. Pad Size vs. Usability
How much pad is enough? 5 MB was the original target. Consider:
- Average text message: ~100 bytes
- 256 KB pad ≈ 2,500 messages (+ 32 bytes MAC overhead each)
- 1 MB pad ≈ 10,000 messages
- 5 MB pad ≈ 50,000 messages

Text-only restriction is inherent — a single image would drain the pad.

### 3. Exhaustion Behavior
Two options per contact (chosen at pairing):
- **Strict:** Refuse to send when pad is empty. Prompt to top up in person.
- **Graceful:** Fall back to Standard mode with a persistent banner until pad refresh.

Graceful fallback creates a quasi-third mode (degraded Absolute). Is that acceptable or does it violate the two-modes philosophy?

### 4. Top-Up Mechanics
Can new pad data be **appended** to an existing pad without re-pairing? If so, what's the protocol? If not, top-up = full re-pair.

---

## UI Requirements (When Built)

- **Pad meter** in chat header: "Pad remaining: 78%"
- Warning at 15% (amber), critical at 5% (red)
- "Top Up Absolute Pad" as a first-class menu action
- Attachments disallowed in Absolute chats — clear inline message explaining why
- Mode is persistent per contact, visible in contact details

---

## Module Impacts (Rust)

| Module | Change |
|---|---|
| `crypto/` | New `otp.rs` — OTP encrypt/decrypt, pad I/O, secure consumption, Poly1305-from-pad MAC |
| `identity/contact.rs` | Add mode enum, OTP pad state (offset, size, path) per contact |
| `identity/pairing.rs` | Mode selection in QR payload, pad transfer step |
| `app/commands.rs` | Route encryption through mode-appropriate engine |
| Frontend | Pad meter widget, exhaustion banners, attachment guard, mode selection in pairing UI |

---

## Philosophy Guardrails

1. Two modes only. No "balanced" third option.
2. No silent failures — pad exhaustion surfaces in UI.
3. Standard must be excellent standalone. Absolute is additive, not a fix.
4. Mode is per-contact, set at pairing. No mid-conversation switching.
5. Pad bytes never reused. Consumed = zeroed + fsynced before send.
6. Error correction H always on QR codes.
