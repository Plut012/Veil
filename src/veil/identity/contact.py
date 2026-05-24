from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class Contact:
    contact_id: str              # UUID — unique identifier
    display_name: str            # human-readable name (from pairing)
    key: bytes                   # 32-byte symmetric encryption key
    telegram_channel_id: int     # Telegram group/chat ID for this contact
    telegram_user_id: int        # contact's Telegram user ID
    created_at: datetime = field(default_factory=datetime.utcnow)
    envelope_template: str = ""  # contact's envelope template (for parsing their messages)
