# Veil — Overview

End-to-end encryption that wraps the platforms you already use. The trust stays with you — the platform becomes a dumb pipe carrying ciphertext it can never read.

## Why

End-to-end encryption in commercial messaging apps is mathematically sound, but the trust is misplaced:

- We trust the provider's implementation hasn't been quietly modified
- Regulatory pressure (EU "Chat Control," etc.) may mandate client-side scanning or backdoors
- Metadata remains visible to the platform regardless
- We have no way to verify the encryption is actually happening

Veil solves this by encrypting locally, before anything touches the platform. Even if the platform is compromised, required to scan content, or quietly backdoored — it only sees opaque ciphertext.

## What It Is

A locally-running desktop app that:

1. Encrypts your messages before they leave your machine
2. Sends the ciphertext through existing messaging platforms (Telegram first)
3. Receives and decrypts messages from paired contacts
4. Feels like a normal, beautiful messaging app — not a security tool

You pair with contacts in person via QR code. One scan creates the shared encryption key, sets up a dedicated Telegram channel, and establishes the contact — a single gesture that bootstraps everything.

## What It Is Not

- Not a new messaging platform — it rides on platforms you already use
- Not paranoid-looking security software — it's warm, tactile, and inviting
- Not for everyday casual messages — for when genuine privacy matters
- Not cloud-dependent — everything runs locally, nothing phones home

## The Feeling

Retro-futuristic analog tech. Warm amber on muted earth tones. Art nouveau ornamental framing around interfaces that feel like instruments — communication equipment from a more beautiful timeline. Every interaction has weight and intention. The UI teaches through its form — if something needs a label, it isn't intuitive enough.

## Design Principles

- **Simple, robust, clever** — in that order. Always.
- **Modular** — every component (crypto, bridges, themes, envelopes) is swappable and extensible
- **Trustless** — the platform never sees plaintext. The key never touches a network.
- **Intuitive, not hand-holding** — affordances are clear from shape and placement. No wizards, no tooltips, no confirmation dialogs for routine actions.
- **Open** — built for two, designed for anyone. Open source from day one.

## Scope

**MVP:**
- Telegram bridge (dedicated channel per contact)
- Symmetric key encryption (XChaCha20-Poly1305)
- In-person QR code pairing (one scan)
- Configurable envelope format
- Art nouveau theme
- Desktop only

**Future:**
- Additional bridges (Signal, Discord, WhatsApp)
- Asymmetric crypto with forward secrecy (Double Ratchet)
- Group encryption
- Mobile (PWA or native)
- Community themes
