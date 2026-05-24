# Veil

End-to-end encryption that wraps the messaging platforms you already use.

Veil encrypts your messages locally before they touch any platform. The platform carries ciphertext it can never read. You pair with contacts in person — one QR scan creates the encryption key, sets up the channel, and establishes the contact.

## Quick Start

```bash
# Backend
uv sync
uv run veil

# Frontend (separate terminal)
cd frontend && npm install && npm run dev
```

Open `http://localhost:5173` — Veil runs entirely on your machine.

## First Use

1. Launch Veil — set your passphrase (encrypts your local keyring)
2. Log in to Telegram when prompted
3. Meet your contact in person — tap "New Contact", show them your QR code
4. They scan it — pairing is automatic (key exchange + Telegram channel creation)
5. Message them through Veil

## How It Works

```
You type → encrypted locally → sent as ciphertext via Telegram → decrypted on their machine
```

The platform never sees your plaintext. Keys are exchanged in person and stored encrypted on your device.

## Project Structure

```
src/veil/       — Python backend (crypto, bridges, identity, API)
frontend/       — Svelte UI
themes/         — Swappable visual themes
docs/           — Architecture and design docs
```

## License

Open source. License TBD.
