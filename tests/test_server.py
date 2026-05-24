"""Tests for app server. Tests config loading and message routing logic."""
import pytest
import json
from pathlib import Path
from veil.app.config import VeilConfig, TelegramConfig


class TestConfig:
    def test_default_config(self):
        config = VeilConfig()
        assert config.host == "127.0.0.1"
        assert config.port == 8900
        assert config.theme == "art-nouveau"

    def test_save_and_load(self, tmp_path):
        path = tmp_path / "config.toml"
        config = VeilConfig(
            display_name="Alice",
            envelope_template="~~ {ciphertext} ~~",
            telegram=TelegramConfig(api_id=123, api_hash="abc"),
        )
        config.save(path)
        loaded = VeilConfig.load(path)
        assert loaded.display_name == "Alice"
        assert loaded.envelope_template == "~~ {ciphertext} ~~"
        assert loaded.telegram.api_id == 123
        assert loaded.telegram.api_hash == "abc"

    def test_load_missing_file(self, tmp_path):
        """Missing config file returns defaults."""
        config = VeilConfig.load(tmp_path / "nonexistent.toml")
        assert config.host == "127.0.0.1"

    def test_binds_localhost_only(self):
        """Default config binds to localhost — not network-accessible."""
        config = VeilConfig()
        assert config.host == "127.0.0.1"
