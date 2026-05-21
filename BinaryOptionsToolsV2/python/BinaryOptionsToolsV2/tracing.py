import json
import os
import sys
import warnings
from datetime import timedelta
from typing import Optional


class LogSubscription:
    def __init__(self, subscription):
        self.subscription = subscription

    def __aiter__(self):
        return self

    async def __anext__(self):
        return json.loads(await self.subscription.__anext__())

    def __iter__(self):
        return self

    def __next__(self):
        return json.loads(next(self.subscription))


def _get_rust_attr(name: str):
    """Get an attribute from the compiled Rust module via package namespace."""
    pkg = sys.modules.get(__package__ or "")
    if pkg is not None and hasattr(pkg, name):
        return getattr(pkg, name)
    import BinaryOptionsToolsV2 as _mod

    return getattr(_mod, name)


class Logger:
    """Wrapper around the Rust Logger for consistent logging."""

    def __init__(self):
        self.logger = _get_rust_attr("Logger")()

    def debug(self, message: str) -> None:
        self.logger.debug(message)

    def info(self, message: str) -> None:
        self.logger.info(message)

    def warn(self, message: str) -> None:
        self.logger.warn(message)

    def error(self, message: str) -> None:
        self.logger.error(message)


class LogBuilder:
    """Builder for configuring log layers and iterators."""

    def __init__(self):
        self.builder = _get_rust_attr("LogBuilder")()

    def log_file(self, path: str, level: str) -> None:
        self.builder.log_file(path, level)

    def terminal(self, level: str) -> None:
        self.builder.terminal(level)

    def build(self) -> None:
        self.builder.build()

    def create_logs_iterator(self, level: str, timeout=None):
        return self.builder.create_logs_iterator(level, timeout)


def start_logs(path: str, level: str = "DEBUG", terminal: bool = True, layers: Optional[list] = None):
    """
    Initialize the logging system.

    Args:
        path: Log file directory.
        level: Logging level (default "DEBUG").
        terminal: Whether to display logs in terminal (default True).
        layers: Optional list of layers to initialize.
    """
    if layers is None:
        layers = []

    start_tracing = _get_rust_attr("start_tracing")
    os.makedirs(path, exist_ok=True)
    try:
        start_tracing(path, level, terminal, layers)
    except Exception as e:
        warnings.warn(f"start_logs: {e}", RuntimeWarning, stacklevel=2)
