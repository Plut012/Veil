# Phase 7: Frontend

**Status:** Not Started
**Dependencies:** Phase 6 (App Server — provides WebSocket API)
**Output:** `frontend/` — complete Svelte application

---

## Purpose

The frontend is a Svelte app that provides the messaging UI. It communicates with the backend exclusively via WebSocket. It never touches crypto, keys, or Telegram directly — it is a renderer with input.

The initial theme is art nouveau: warm amber tones, muted earth colors, ornamental framing, retro-futuristic analog warmth. Intuitive, not hand-holding.

---

## Design Philosophy

- **The UI teaches through form.** If an element needs a tooltip or label, redesign it.
- **Warm and tactile.** Interactions have weight. Messages don't pop in — they arrive.
- **Retro-futuristic analog.** Think communication equipment from a more beautiful timeline. Dials, not dropdowns. Indicators, not badges.
- **Minimal chrome.** The conversation is the focus. Controls recede until needed.
- **No hand-holding.** No onboarding wizard. No "getting started" guide. The interface is self-evident.

---

## Project Setup

```bash
cd frontend
npm create svelte@latest .  # Skeleton project, TypeScript
npm install
```

**Dependencies:**
- `svelte` + `@sveltejs/kit` — framework
- `html5-qrcode` — QR code scanning via webcam (for pairing)
- No UI framework — custom components with scoped CSS

---

## Components

### Layout

```
┌─────────────────────────────────────────────────────┐
│                    Veil                              │
├──────────────┬──────────────────────────────────────┤
│              │                                      │
│  Contact     │          Messages                    │
│  List        │                                      │
│              │  ┌────────────────────────────────┐   │
│  ○ Alice     │  │ Alice: decrypted message       │   │
│  ○ Bob       │  │ You: decrypted reply           │   │
│              │  │ Alice: another message          │   │
│              │  └────────────────────────────────┘   │
│              │                                      │
│              │  ┌────────────────────────────────┐   │
│  [+ pair]    │  │ compose...                     │   │
│              │  └────────────────────────────────┘   │
├──────────────┴──────────────────────────────────────┤
│  ⚙ settings                                        │
└─────────────────────────────────────────────────────┘
```

### Component Tree

```
App.svelte
├── Sidebar.svelte
│   ├── ContactList.svelte
│   │   └── ContactItem.svelte (one per contact)
│   └── PairButton.svelte
├── ChatView.svelte
│   ├── MessageList.svelte
│   │   └── Message.svelte (one per message)
│   └── Compose.svelte
├── PairingModal.svelte
│   ├── InitiateView.svelte (shows QR code)
│   └── JoinView.svelte (camera scanner)
└── SettingsPanel.svelte
    ├── EnvelopeConfig.svelte
    └── ThemeSelector.svelte
```

### File Map

```
frontend/src/
├── lib/
│   ├── components/
│   │   ├── Sidebar.svelte
│   │   ├── ContactList.svelte
│   │   ├── ContactItem.svelte
│   │   ├── ChatView.svelte
│   │   ├── MessageList.svelte
│   │   ├── Message.svelte
│   │   ├── Compose.svelte
│   │   ├── PairingModal.svelte
│   │   ├── SettingsPanel.svelte
│   │   └── EnvelopeConfig.svelte
│   ├── stores/
│   │   ├── connection.ts      — WebSocket connection state
│   │   ├── contacts.ts        — contact list store
│   │   ├── messages.ts        — message store (per contact)
│   │   └── ui.ts              — UI state (selected contact, modals)
│   └── api/
│       └── websocket.ts       — WebSocket client, message types
├── routes/
│   ├── +layout.svelte         — theme loading, global styles
│   └── +page.svelte           — main app page
├── app.css                    — CSS variables, base styles
└── app.html
```

---

## Stores

### `connection.ts`

```typescript
// WebSocket connection state
type ConnectionState = "connecting" | "connected" | "disconnected" | "error";

// Writable store: connectionState
// Derived: isConnected
```

### `contacts.ts`

```typescript
interface Contact {
  contact_id: string;
  display_name: string;
  telegram_channel_id: number;
}

// Writable store: contacts (Contact[])
// Functions: setContacts, addContact
```

### `messages.ts`

```typescript
interface Message {
  contact_id: string;
  text: string;
  direction: "in" | "out";
  timestamp: string;
}

// Writable store: messages (Map<contact_id, Message[]>)
// Functions: addMessage, getMessagesForContact
```

### `ui.ts`

```typescript
// Writable stores:
//   selectedContactId: string | null
//   showPairingModal: boolean
//   showSettings: boolean
//   pairingRole: "initiate" | "join" | null
```

---

## WebSocket Client (`api/websocket.ts`)

```typescript
class VeilSocket {
  private ws: WebSocket;
  private handlers: Map<string, (data: any) => void>;

  connect(url: string): void;
  disconnect(): void;
  send(message: object): void;

  // Typed send helpers
  sendMessage(contactId: string, text: string): void;
  requestContacts(): void;
  initiatePairing(): void;
  completePairing(qrData: string): void;
  updateEnvelope(template: string): void;
  setTheme(themeId: string): void;

  // Event registration
  on(type: string, handler: (data: any) => void): void;
}
```

**Auto-reconnect:** If the WebSocket drops, attempt reconnection with exponential backoff (1s, 2s, 4s, max 30s).

---

## Theme System

### CSS Variables (defined in `app.css`, overridden by theme)

```css
:root {
  /* Art Nouveau defaults */
  --color-bg:           #2a2520;
  --color-bg-surface:   #352f29;
  --color-bg-elevated:  #3f3832;
  --color-text:         #d4c5b3;
  --color-text-muted:   #8a7e71;
  --color-accent:       #c4956a;
  --color-accent-dim:   #8a6848;
  --color-border:       #4a4238;
  --color-msg-in:       #3a342e;
  --color-msg-out:      #4a3f34;
  --color-success:      #7a9a6a;
  --color-error:        #9a5a4a;

  --font-body:          "IBM Plex Sans", sans-serif;
  --font-mono:          "IBM Plex Mono", monospace;

  --radius-sm:          4px;
  --radius-md:          8px;
  --spacing-xs:         4px;
  --spacing-sm:         8px;
  --spacing-md:         16px;
  --spacing-lg:         24px;

  --transition-fast:    120ms ease;
  --transition-normal:  250ms ease;
}
```

### Theme Loading

The active theme's CSS file is loaded dynamically via a `<link>` tag in `+layout.svelte`. Switching themes swaps the stylesheet — no page reload.

```
themes/art-nouveau/theme.css    → overrides CSS variables
themes/art-nouveau/assets/      → ornamental borders, textures
```

---

## Key Interactions

### Sending a Message
1. User types in `Compose` and presses Enter (or the send affordance)
2. `Compose` calls `veilSocket.sendMessage(selectedContactId, text)`
3. Backend encrypts, wraps, sends via Telegram
4. Backend echoes `{ type: "message", direction: "out" }` back
5. `messages` store updates, `MessageList` renders the new message

### Receiving a Message
1. Backend receives from Telegram, decrypts, broadcasts via WebSocket
2. `{ type: "message", direction: "in" }` arrives
3. `messages` store updates, `MessageList` renders

### Pairing (Initiator)
1. User taps pair button → selects "Show QR"
2. `PairingModal` opens with `InitiateView`
3. Frontend sends `{ type: "initiate_pairing" }`
4. Backend returns `{ type: "pairing_qr", qr_image: "<base64 PNG>" }`
5. QR code displayed on screen
6. When joiner completes their side, backend sends `{ type: "pairing_complete" }`
7. Modal closes, new contact appears in list

### Pairing (Joiner)
1. User taps pair button → selects "Scan QR"
2. `PairingModal` opens with `JoinView` (camera activated)
3. `html5-qrcode` decodes the QR → JSON string
4. Frontend sends `{ type: "complete_pairing", qr_data: "..." }`
5. Backend creates channel, sends handshake, saves contact
6. Backend sends `{ type: "pairing_complete" }`
7. Modal closes, new contact appears in list

---

## Acceptance Criteria

- [ ] SvelteKit project initializes and builds
- [ ] WebSocket connects to backend on page load
- [ ] Contact list renders from backend state
- [ ] Selecting a contact shows conversation view
- [ ] Typing and sending a message works end-to-end
- [ ] Incoming messages appear in real-time
- [ ] Pairing modal: initiator flow shows QR code
- [ ] Pairing modal: joiner flow activates camera and scans QR
- [ ] New contacts appear in sidebar after pairing
- [ ] Settings panel allows envelope template editing
- [ ] Theme CSS variables produce warm, muted, art nouveau aesthetic
- [ ] No tooltips, no onboarding, no hand-holding — interface is self-evident
- [ ] Frontend never touches crypto or Telegram directly
- [ ] Auto-reconnect on WebSocket disconnect
