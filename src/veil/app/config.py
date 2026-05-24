from dataclasses import dataclass, field
from pathlib import Path

import tomli
import tomli_w

VEIL_DIR = Path.home() / ".veil"
CONFIG_PATH = VEIL_DIR / "config.toml"


@dataclass
class TelegramConfig:
    api_id: int = 0
    api_hash: str = ""
    session_path: str = str(VEIL_DIR / "telegram.session")


@dataclass
class VeilConfig:
    telegram: TelegramConfig = field(default_factory=TelegramConfig)
    envelope_template: str = ""
    display_name: str = "Veil User"
    theme: str = "art-nouveau"
    host: str = "127.0.0.1"
    port: int = 8900

    @classmethod
    def load(cls, path: Path = CONFIG_PATH) -> "VeilConfig":
        """Load config from TOML file. Returns defaults if file doesn't exist."""
        if not path.exists():
            return cls()
        with open(path, "rb") as f:
            data = tomli.load(f)
        config = cls()
        if "telegram" in data:
            config.telegram = TelegramConfig(**data["telegram"])
        config.envelope_template = data.get("envelope_template", "")
        config.display_name = data.get("display_name", "Veil User")
        config.theme = data.get("theme", "art-nouveau")
        config.host = data.get("host", "127.0.0.1")
        config.port = data.get("port", 8900)
        return config

    def save(self, path: Path = CONFIG_PATH) -> None:
        """Write current config to TOML file."""
        data = {
            "display_name": self.display_name,
            "envelope_template": self.envelope_template,
            "theme": self.theme,
            "host": self.host,
            "port": self.port,
            "telegram": {
                "api_id": self.telegram.api_id,
                "api_hash": self.telegram.api_hash,
                "session_path": self.telegram.session_path,
            },
        }
        path.parent.mkdir(mode=0o700, exist_ok=True)
        with open(path, "wb") as f:
            tomli_w.dump(data, f)
