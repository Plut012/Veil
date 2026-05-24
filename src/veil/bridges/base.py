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
