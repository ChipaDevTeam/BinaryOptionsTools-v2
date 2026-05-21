import json
import sys
from dataclasses import dataclass, field
from typing import Any, Dict, List


def _get_pyconfig():
    """Get the PyConfig class from the compiled Rust module via package namespace."""
    pkg = sys.modules.get(__package__ or "")
    if pkg is not None and hasattr(pkg, "PyConfig"):
        return pkg.PyConfig
    import BinaryOptionsToolsV2 as _mod

    return _mod.PyConfig


@dataclass
class Config:
    """
    Python wrapper around PyConfig that provides additional functionality
    for configuration management.
    """

    max_allowed_loops: int = 100
    sleep_interval: int = 100
    reconnect_time: int = 5
    connection_initialization_timeout_secs: int = 60
    timeout_secs: int = 30
    urls: List[str] = field(default_factory=list)
    max_subscriptions: int = 4

    # Logging configuration
    terminal_logging: bool = False
    log_level: str = "INFO"

    # Extra duration, used by functions like `check_win`
    extra_duration: int = 5

    def __post_init__(self):
        self.urls = self.urls or []
        self._pyconfig = None
        self._locked = False

    def __setattr__(self, name: str, value: Any) -> None:
        if name.startswith("_") or not hasattr(self, "_locked") or not self._locked:
            super().__setattr__(name, value)
        else:
            raise RuntimeError("Configuration is locked and cannot be modified after being used")

    @property
    def pyconfig(self) -> Any:
        """Returns the PyConfig instance for use in Rust code, then locks config."""
        if self._pyconfig is None:
            self._pyconfig = _get_pyconfig()()
            self._sync_pyconfig()
        self._locked = True
        return self._pyconfig

    def _sync_pyconfig(self):
        """Sync all Python config fields to the Rust PyConfig instance."""
        if self._pyconfig is None:
            self._pyconfig = _get_pyconfig()()

        self._pyconfig.max_allowed_loops = self.max_allowed_loops
        self._pyconfig.sleep_interval = self.sleep_interval
        self._pyconfig.reconnect_time = self.reconnect_time
        self._pyconfig.connection_initialization_timeout_secs = self.connection_initialization_timeout_secs
        self._pyconfig.timeout_secs = self.timeout_secs
        self._pyconfig.urls = self.urls
        self._pyconfig.max_subscriptions = self.max_subscriptions

    def _validate(self):
        """Validate config values, raising ValueError on invalid input."""
        if self.max_allowed_loops < 0:
            raise ValueError("max_allowed_loops must be non-negative")
        if self.sleep_interval < 0:
            raise ValueError("sleep_interval must be non-negative")
        if self.reconnect_time < 1:
            raise ValueError("reconnect_time must be at least 1 second")
        if self.connection_initialization_timeout_secs < 1:
            raise ValueError("connection_initialization_timeout_secs must be at least 1")
        if self.timeout_secs < 1:
            raise ValueError("timeout_secs must be at least 1")
        if self.max_subscriptions < 1:
            raise ValueError("max_subscriptions must be at least 1")

    @classmethod
    def from_dict(cls, config_dict: Dict[str, Any]) -> "Config":
        """Creates a Config instance from a dictionary."""
        cfg = cls(**{k: v for k, v in config_dict.items() if k in cls.__dataclass_fields__})
        cfg._validate()
        return cfg

    @classmethod
    def from_json(cls, json_str: str) -> "Config":
        """Creates a Config instance from a JSON string."""
        return cls.from_dict(json.loads(json_str))

    def to_dict(self) -> Dict[str, Any]:
        """Converts the configuration to a dictionary."""
        return {
            "max_allowed_loops": self.max_allowed_loops,
            "sleep_interval": self.sleep_interval,
            "reconnect_time": self.reconnect_time,
            "connection_initialization_timeout_secs": self.connection_initialization_timeout_secs,
            "timeout_secs": self.timeout_secs,
            "urls": self.urls,
            "max_subscriptions": self.max_subscriptions,
            "terminal_logging": self.terminal_logging,
            "log_level": self.log_level,
            "extra_duration": self.extra_duration,
        }

    def to_json(self) -> str:
        """Converts the configuration to a JSON string."""
        return json.dumps(self.to_dict())

    def update(self, config_dict: Dict[str, Any]) -> None:
        """Updates config from a dictionary. Raises RuntimeError if locked."""
        if self._locked:
            raise RuntimeError("Configuration is locked and cannot be modified after being used")
        for key, value in config_dict.items():
            if hasattr(self, key):
                setattr(self, key, value)
