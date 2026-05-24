# Phase 4: Telegram Bridge

**Status:** Not Started
**Dependencies:** Phase 1 (Crypto Engine — for session encryption), Phase 2 (Identity Store — for passphrase-derived key)
**Output:** `src/veil/bridges/base.py`, `src/veil/bridges/telegram.py`, `tests/test_bridge_telegram.py`

---

## Purpose

The Telegram bridge connects Veil to Telegram using Telethon. It authenticates as the user's Telegram account, sends envelopes to dedicated channels, receives messages from those channels, and creates new channels during pairing.

The bridge never sees plaintext. It moves opaque envelope strings.

---

## Files

### `src/veil/bridges/base.py`

Abstract bridge interface. All platform bridges implement this protocol.

```python
from typing import Protocol, Callable, Awaitable


class Bridge(Protocol):
    """Platform bridge interface. Moves envelopes — never sees plaintext."""

    async def connect(self) -> None:
        """Establish connection to the platform. May require interactive auth on first use."""
        ...

    async def disconnect(self) -> None:
        """Cleanly close the connection."""
        ...

    async def send(self, channel_id: int, envelope: str) -> None:
        """Send an envelope string to a channel."""
        ...

    async def on_receive(self, callback: Callable[[int, str], Awaitable[None]]) -> None:
        """
        Register a callback for incoming messages.

        Args:
            callback: async function(channel_id: int, message_text: str)
                      Called for every new message in monitored channels.
        """
        ...

    async def create_channel(self, user_ids: list[int], name: str) -> int:
        """
        Create a new channel/group and add users.

        Args:
            user_ids: Telegram user IDs to add to the channel.
            name: Display name for the channel.

        Returns:
            Channel ID of the created group.
        """
        ...

    async def get_self_user_id(self) -> int:
        """Return the authenticated user's Telegram ID."""
        ...
```

### `src/veil/bridges/telegram.py`

Telethon-based implementation.

```python
import asyncio
import logging
from pathlib import Path
from typing import Callable, Awaitable

from telethon import TelegramClient, events
from telethon.tl.functions.messages import CreateChatRequest

from veil.bridges.base import Bridge

logger = logging.getLogger(__name__)


class TelegramBridge:
    """Telegram bridge using Telethon user client."""

    def __init__(
        self,
        api_id: int,
        api_hash: str,
        session_path: Path,
        monitored_channels: set[int] | None = None,
    ):
        """
        Args:
            api_id: Telegram API ID (from my.telegram.org).
            api_hash: Telegram API hash.
            session_path: Path to the Telethon session file.
            monitored_channels: Set of channel IDs to listen on.
                                Updated as contacts are added.
        """
        self.api_id = api_id
        self.api_hash = api_hash
        self.session_path = session_path
        self.monitored_channels: set[int] = monitored_channels or set()
        self._client: TelegramClient | None = None
        self._receive_callback: Callable[[int, str], Awaitable[None]] | None = None

    async def connect(self) -> None:
        """
        Connect to Telegram and authenticate.

        First-time use: Telethon will prompt for phone number and code
        via terminal input (interactive). Subsequent uses: session file
        provides automatic re-authentication.
        """
        self._client = TelegramClient(
            str(self.session_path),
            self.api_id,
            self.api_hash,
        )
        await self._client.start()
        logger.info("Telegram bridge connected")

        # Register message handler
        @self._client.on(events.NewMessage)
        async def handler(event):
            chat_id = event.chat_id
            if chat_id in self.monitored_channels and self._receive_callback:
                await self._receive_callback(chat_id, event.raw_text)

    async def disconnect(self) -> None:
        """Disconnect the Telethon client."""
        if self._client:
            await self._client.disconnect()
            logger.info("Telegram bridge disconnected")

    async def send(self, channel_id: int, envelope: str) -> None:
        """Send an envelope message to a Telegram chat."""
        if not self._client:
            raise RuntimeError("Bridge not connected")
        await self._client.send_message(channel_id, envelope)

    async def on_receive(self, callback: Callable[[int, str], Awaitable[None]]) -> None:
        """Register the receive callback. Only one callback at a time."""
        self._receive_callback = callback

    async def create_channel(self, user_ids: list[int], name: str) -> int:
        """
        Create a Telegram group chat and add users.

        Args:
            user_ids: Telegram user IDs to invite.
            name: Group title.

        Returns:
            Chat ID of the created group.
        """
        if not self._client:
            raise RuntimeError("Bridge not connected")

        # Resolve user IDs to input users
        users = []
        for uid in user_ids:
            entity = await self._client.get_input_entity(uid)
            users.append(entity)

        result = await self._client(CreateChatRequest(
            users=users,
            title=name,
        ))

        # Extract chat ID from the result
        chat_id = result.chats[0].id
        self.monitored_channels.add(chat_id)
        logger.info(f"Created Telegram channel: {name} (ID: {chat_id})")
        return chat_id

    async def get_self_user_id(self) -> int:
        """Return the authenticated user's Telegram ID."""
        if not self._client:
            raise RuntimeError("Bridge not connected")
        me = await self._client.get_me()
        return me.id

    def add_monitored_channel(self, channel_id: int) -> None:
        """Start monitoring a channel for incoming messages."""
        self.monitored_channels.add(channel_id)

    def remove_monitored_channel(self, channel_id: int) -> None:
        """Stop monitoring a channel."""
        self.monitored_channels.discard(channel_id)
```

**Implementation notes:**

**Telegram API credentials:**
- Users must register their own app at [my.telegram.org](https://my.telegram.org) to get `api_id` and `api_hash`. These are stored in `~/.veil/config.toml`.
- These are NOT secret in the same way as encryption keys — they identify the application, not the user. But they shouldn't be hardcoded or shared publicly.

**Session management:**
- Telethon creates a `.session` file (SQLite) that stores the auth state.
- Stored at `~/.veil/telegram.session` — path is configurable.
- First connection requires interactive phone number + code entry via terminal.
- Subsequent connections are automatic.
- Future: encrypt the session file at rest using the user's passphrase (Phase 2 storage key). For MVP, rely on filesystem permissions (`0o600`).

**Message handling:**
- The `NewMessage` event handler fires for ALL incoming messages.
- We filter to only `monitored_channels` — messages from non-Veil chats are ignored.
- The callback receives raw channel ID + message text. The app layer handles envelope parsing and decryption.

**Channel creation:**
- Uses `CreateChatRequest` to create a standard group chat (not a supergroup/channel).
- Group name is configurable — could be "Veil" or something the user sets.
- The created chat ID is automatically added to `monitored_channels`.

### `src/veil/bridges/__init__.py`

```python
from veil.bridges.base import Bridge
from veil.bridges.telegram import TelegramBridge
```

### `tests/test_bridge_telegram.py`

Testing the bridge is nuanced — Telethon requires a real Telegram account. The test file provides:
1. Unit tests for logic that doesn't need a connection (channel monitoring set management)
2. Integration test stubs marked with `@pytest.mark.integration` that require credentials

```python
"""
Tests for Telegram bridge.

Unit tests run without Telegram credentials.
Integration tests require TELEGRAM_API_ID and TELEGRAM_API_HASH env vars.
"""
import pytest
from unittest.mock import AsyncMock, MagicMock
from pathlib import Path
from veil.bridges.telegram import TelegramBridge


@pytest.fixture
def bridge(tmp_path):
    """Create a bridge instance (not connected)."""
    return TelegramBridge(
        api_id=12345,
        api_hash="fake_hash",
        session_path=tmp_path / "test.session",
    )


class TestChannelMonitoring:
    def test_add_monitored_channel(self, bridge):
        bridge.add_monitored_channel(100)
        assert 100 in bridge.monitored_channels

    def test_remove_monitored_channel(self, bridge):
        bridge.add_monitored_channel(100)
        bridge.remove_monitored_channel(100)
        assert 100 not in bridge.monitored_channels

    def test_remove_nonexistent_channel(self, bridge):
        """Removing a channel that isn't monitored is a no-op."""
        bridge.remove_monitored_channel(999)  # should not raise

    def test_initial_monitored_channels(self, tmp_path):
        bridge = TelegramBridge(
            api_id=12345,
            api_hash="fake_hash",
            session_path=tmp_path / "test.session",
            monitored_channels={100, 200},
        )
        assert bridge.monitored_channels == {100, 200}


class TestNotConnected:
    @pytest.mark.asyncio
    async def test_send_without_connect_raises(self, bridge):
        with pytest.raises(RuntimeError, match="not connected"):
            await bridge.send(100, "hello")

    @pytest.mark.asyncio
    async def test_create_channel_without_connect_raises(self, bridge):
        with pytest.raises(RuntimeError, match="not connected"):
            await bridge.create_channel([1], "test")

    @pytest.mark.asyncio
    async def test_get_self_without_connect_raises(self, bridge):
        with pytest.raises(RuntimeError, match="not connected"):
            await bridge.get_self_user_id()


# Integration tests (require real Telegram credentials)
# Run with: uv run pytest tests/test_bridge_telegram.py -m integration
#
# @pytest.mark.integration
# class TestTelegramIntegration:
#     """These tests connect to real Telegram. Set env vars first."""
#     ...
```

---

## Configuration

The bridge needs Telegram API credentials. These are stored in `~/.veil/config.toml`:

```toml
[telegram]
api_id = 12345678
api_hash = "your_api_hash_here"
session_path = "~/.veil/telegram.session"
```

The user obtains these from [my.telegram.org](https://my.telegram.org) during first setup.

---

## Acceptance Criteria

- [ ] `TelegramBridge` implements the `Bridge` protocol
- [ ] `connect()` establishes a Telethon client session
- [ ] `send(channel_id, envelope)` delivers a message to the specified chat
- [ ] `on_receive(callback)` registers a handler that fires for messages in monitored channels only
- [ ] Messages from non-monitored channels are ignored
- [ ] `create_channel(user_ids, name)` creates a Telegram group and returns its ID
- [ ] Created channels are automatically added to monitored set
- [ ] `get_self_user_id()` returns the authenticated user's Telegram ID
- [ ] Operations on a disconnected bridge raise `RuntimeError`
- [ ] Unit tests pass without Telegram credentials: `uv run pytest tests/test_bridge_telegram.py -v`
