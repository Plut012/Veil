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
        self.passthrough: bool = False
        self._client: TelegramClient | None = None
        self._receive_callback: Callable[[int, str], Awaitable[None]] | None = None
        self._self_user_id: int | None = None

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
        me = await self._client.get_me()
        self._self_user_id = me.id
        logger.info("Telegram bridge connected")

        @self._client.on(events.NewMessage)
        async def handler(event):
            if event.sender_id == self._self_user_id:
                return  # skip own messages
            chat_id = event.chat_id
            if (self.passthrough or chat_id in self.monitored_channels) and self._receive_callback:
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

        users = []
        for uid in user_ids:
            entity = await self._client.get_input_entity(uid)
            users.append(entity)

        result = await self._client(CreateChatRequest(
            users=users,
            title=name,
        ))

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
