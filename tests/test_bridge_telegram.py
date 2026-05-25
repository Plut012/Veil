"""
Tests for Telegram bridge.

Unit tests run without Telegram credentials.
Integration tests require TELEGRAM_API_ID and TELEGRAM_API_HASH env vars.
"""
import pytest
from pathlib import Path
from unittest.mock import AsyncMock
from veil.bridges.telegram import TelegramBridge


@pytest.fixture
def bridge(tmp_path):
    """Create a bridge instance (not connected)."""
    return TelegramBridge(
        api_id=12345,
        api_hash="fake_hash",
        session_path=tmp_path / "test.session",
    )


def make_event(sender_id: int, chat_id: int, text: str = "hello"):
    """Create a minimal fake Telegram event."""
    class FakeEvent:
        pass
    e = FakeEvent()
    e.sender_id = sender_id
    e.chat_id = chat_id
    e.raw_text = text
    return e


async def simulate_handler(bridge: TelegramBridge, event) -> bool:
    """
    Mirror the handler logic from TelegramBridge.connect() without needing
    a real Telethon client.  Returns True if the callback would have fired.
    """
    if event.sender_id == bridge._self_user_id:
        return False
    chat_id = event.chat_id
    if (bridge.passthrough or chat_id in bridge.monitored_channels) and bridge._receive_callback:
        await bridge._receive_callback(chat_id, event.raw_text)
        return True
    return False


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

    def test_add_multiple_channels(self, bridge):
        bridge.add_monitored_channel(1)
        bridge.add_monitored_channel(2)
        bridge.add_monitored_channel(3)
        assert bridge.monitored_channels == {1, 2, 3}

    def test_add_duplicate_channel(self, bridge):
        """Adding the same channel twice is idempotent."""
        bridge.add_monitored_channel(100)
        bridge.add_monitored_channel(100)
        assert len([c for c in bridge.monitored_channels if c == 100]) == 1

    def test_empty_initial_set(self, bridge):
        assert bridge.monitored_channels == set()


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

    @pytest.mark.asyncio
    async def test_on_receive_without_connect_does_not_raise(self, bridge):
        """Registering a callback before connecting is allowed."""
        async def callback(channel_id: int, text: str) -> None:
            pass
        await bridge.on_receive(callback)  # should not raise
        assert bridge._receive_callback is callback


class TestPassthrough:
    def test_passthrough_defaults_false(self, bridge):
        assert bridge.passthrough is False

    def test_passthrough_can_be_set(self, bridge):
        bridge.passthrough = True
        assert bridge.passthrough is True

    def test_passthrough_can_be_cleared(self, bridge):
        bridge.passthrough = True
        bridge.passthrough = False
        assert bridge.passthrough is False

    @pytest.mark.asyncio
    async def test_passthrough_fires_callback_for_unmonitored_channel(self, bridge):
        """With passthrough=True, messages from unknown channels reach the callback."""
        received = []

        async def callback(channel_id: int, text: str) -> None:
            received.append((channel_id, text))

        await bridge.on_receive(callback)
        bridge._self_user_id = 1  # not the sender
        bridge.passthrough = True

        event = make_event(sender_id=999, chat_id=555, text="handshake")
        await simulate_handler(bridge, event)

        assert received == [(555, "handshake")]

    @pytest.mark.asyncio
    async def test_no_passthrough_drops_unmonitored_channel(self, bridge):
        """Without passthrough, messages from unknown channels are dropped."""
        received = []

        async def callback(channel_id: int, text: str) -> None:
            received.append((channel_id, text))

        await bridge.on_receive(callback)
        bridge._self_user_id = 1
        bridge.passthrough = False

        event = make_event(sender_id=999, chat_id=555, text="handshake")
        await simulate_handler(bridge, event)

        assert received == []

    @pytest.mark.asyncio
    async def test_monitored_channel_fires_without_passthrough(self, bridge):
        """Monitored channels still fire when passthrough is off."""
        received = []

        async def callback(channel_id: int, text: str) -> None:
            received.append((channel_id, text))

        await bridge.on_receive(callback)
        bridge._self_user_id = 1
        bridge.passthrough = False
        bridge.add_monitored_channel(100)

        event = make_event(sender_id=999, chat_id=100, text="msg")
        await simulate_handler(bridge, event)

        assert received == [(100, "msg")]


class TestSelfMessageFiltering:
    def test_self_user_id_defaults_none(self, bridge):
        assert bridge._self_user_id is None

    @pytest.mark.asyncio
    async def test_own_message_is_dropped(self, bridge):
        """Messages where sender_id == _self_user_id are silently dropped."""
        received = []

        async def callback(channel_id: int, text: str) -> None:
            received.append((channel_id, text))

        await bridge.on_receive(callback)
        bridge._self_user_id = 42
        bridge.passthrough = True  # would fire for anyone else

        event = make_event(sender_id=42, chat_id=100, text="my own message")
        fired = await simulate_handler(bridge, event)

        assert fired is False
        assert received == []

    @pytest.mark.asyncio
    async def test_other_user_message_is_not_dropped(self, bridge):
        """Messages from a different sender are not filtered out."""
        received = []

        async def callback(channel_id: int, text: str) -> None:
            received.append((channel_id, text))

        await bridge.on_receive(callback)
        bridge._self_user_id = 42
        bridge.passthrough = True

        event = make_event(sender_id=99, chat_id=100, text="their message")
        fired = await simulate_handler(bridge, event)

        assert fired is True
        assert received == [(100, "their message")]

    @pytest.mark.asyncio
    async def test_own_message_dropped_even_in_monitored_channel(self, bridge):
        """Self-message filtering takes priority over monitored channels."""
        received = []

        async def callback(channel_id: int, text: str) -> None:
            received.append((channel_id, text))

        await bridge.on_receive(callback)
        bridge._self_user_id = 42
        bridge.passthrough = False
        bridge.add_monitored_channel(100)

        event = make_event(sender_id=42, chat_id=100, text="my own message")
        fired = await simulate_handler(bridge, event)

        assert fired is False
        assert received == []


# Integration tests (require real Telegram credentials)
# Run with: uv run pytest tests/test_bridge_telegram.py -m integration
#
# @pytest.mark.integration
# class TestTelegramIntegration:
#     """These tests connect to real Telegram. Set env vars first."""
#     ...
