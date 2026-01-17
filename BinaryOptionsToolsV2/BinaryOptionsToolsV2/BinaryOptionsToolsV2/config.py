# from BinaryOptionsToolsV2 import PyConfig
import json
from dataclasses import dataclass
from typing import Any, Dict, List


class PyConfig:
    """
    Temporary Python replacement for the Rust PyConfig class.
    This class mimics the behavior of the Rust PyConfig until it's available.

    This acts as an advanced dictionary with attribute access and provides
    the same interface that the Rust PyConfig would have.

    Attributes:
        max_allowed_loops (int): Maximum number of allowed loops
        sleep_interval (int): Sleep interval in milliseconds
        reconnect_time (int): Reconnection time in seconds
        connection_initialization_timeout_secs (int): Connection timeout in seconds
        timeout_secs (int): General timeout in seconds
        urls (List[str]): List of WebSocket URLs
    """

    def __init__(self):
        self.max_allowed_loops: int = 100
        self.sleep_interval: int = 100
        self.reconnect_time: int = 5
        self.connection_initialization_timeout_secs: int = 30
        self.timeout_secs: int = 30
        self.urls: List[str] = []

    def __repr__(self) -> str:
        return (
            f"PyConfig(max_allowed_loops={self.max_allowed_loops}, "
            f"sleep_interval={self.sleep_interval}, "
            f"reconnect_time={self.reconnect_time}, "
            f"connection_initialization_timeout_secs={self.connection_initialization_timeout_secs}, "
            f"timeout_secs={self.timeout_secs}, "
            f"urls={self.urls})"
        )

    def to_dict(self) -> Dict[str, Any]:
        """Convert PyConfig to dictionary"""
        return {
            "max_allowed_loops": self.max_allowed_loops,
            "sleep_interval": self.sleep_interval,
            "reconnect_time": self.reconnect_time,
            "connection_initialization_timeout_secs": self.connection_initialization_timeout_secs,
            "timeout_secs": self.timeout_secs,
            "urls": self.urls.copy(),
        }

    def update_from_dict(self, data: Dict[str, Any]) -> None:
        """Update PyConfig from dictionary"""
        for key, value in data.items():
            if hasattr(self, key):
                setattr(self, key, value)


@dataclass
class Config:
    """
    Python wrapper around PyConfig that provides additional functionality
    for configuration management.
    """

    max_allowed_loops: int = 100
    sleep_interval: int = 100
    reconnect_time: int = 5
    connection_initialization_timeout_secs: int = 30
    timeout_secs: int = 30
    urls: List[str] = field(default_factory=list)

    # Extra duration, used by functions like `check_win`
    extra_duration: int = 5

    def __post_init__(self):
        self.urls = self.urls or []
        self._pyconfig = None
        self._locked = False

    def __setattr__(self, name: str, value: Any) -> None:
        """Override setattr to check for locked state"""
        # Allow setting private attributes and during initialization
        if name.startswith("_") or not hasattr(self, "_locked") or not self._locked:
            super().__setattr__(name, value)
        else:
            raise RuntimeError(
                "Configuration is locked and cannot be modified after being used"
            )

    @property
    def pyconfig(self) -> PyConfig:
        """
        Returns the PyConfig instance for use in Rust code.
        Once this is accessed, the configuration becomes locked.
        """
        if self._pyconfig is None:
            self._pyconfig = PyConfig()
            self._update_pyconfig()
        self._locked = True
        return self._pyconfig

    def _update_pyconfig(self):
        """Updates the internal PyConfig with current values"""
        if self._locked:
            raise RuntimeError(
                "Configuration is locked and cannot be modified after being used"
            )

        if self._pyconfig is None:
            self._pyconfig = PyConfig()

        self._pyconfig.max_allowed_loops = self.max_allowed_loops
        self._pyconfig.sleep_interval = self.sleep_interval
        self._pyconfig.reconnect_time = self.reconnect_time
        self._pyconfig.connection_initialization_timeout_secs = (
            self.connection_initialization_timeout_secs
        )
        self._pyconfig.timeout_secs = self.timeout_secs
        self._pyconfig.urls = self.urls.copy()

    @classmethod
    def from_dict(cls, config_dict: Dict[str, Any]) -> "Config":
        """
        Creates a Config instance from a dictionary.

        Args:
            config_dict: Dictionary containing configuration values

        Returns:
            Config instance
        """
        return cls(
            **{k: v for k, v in config_dict.items() if k in Config.__dataclass_fields__}
        )

    @classmethod
    def from_json(cls, json_str: str) -> "Config":
        """
        Creates a Config instance from a JSON string.

        Args:
            json_str: JSON string containing configuration values

        Returns:
            Config instance
        """
        return cls.from_dict(json.loads(json_str))

    def to_dict(self) -> Dict[str, Any]:
        """
        Converts the configuration to a dictionary.

        Returns:
            Dictionary containing all configuration values
        """
        return {
            "max_allowed_loops": self.max_allowed_loops,
            "sleep_interval": self.sleep_interval,
            "reconnect_time": self.reconnect_time,
            "connection_initialization_timeout_secs": self.connection_initialization_timeout_secs,
            "timeout_secs": self.timeout_secs,
            "urls": self.urls,
        }

    def to_json(self) -> str:
        """
        Converts the configuration to a JSON string.

        Returns:
            JSON string containing all configuration values
        """
        return json.dumps(self.to_dict())

    def update(self, config_dict: Dict[str, Any]) -> None:
        """
        Updates the configuration with values from a dictionary.

        Args:
            config_dict: Dictionary containing new configuration values
        """
        if self._locked:
            raise RuntimeError(
                "Configuration is locked and cannot be modified after being used"
            )

        for key, value in config_dict.items():
            if hasattr(self, key):
                setattr(self, key, value)
