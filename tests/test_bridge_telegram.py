"""
Tests for Telegram bridge.

Unit tests run without Telegram credentials.
Integration tests require TELEGRAM_API_ID and TELEGRAM_API_HASH env vars.
"""
import pytest
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


# Integration tests (require real Telegram credentials)
# Run with: uv run pytest tests/test_bridge_telegram.py -m integration
#
# @pytest.mark.integration
# class TestTelegramIntegration:
#     """These tests connect to real Telegram. Set env vars first."""
#     ...
