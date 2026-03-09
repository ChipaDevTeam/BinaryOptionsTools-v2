import json
import os
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
    """Helper to get an attribute from the compiled Rust module."""
    try:
        # First try to import from the package
        from . import BinaryOptionsToolsV2 as pkg

        if hasattr(pkg, name):
            return getattr(pkg, name)
    except (ImportError, AttributeError):
        pass

    try:
        from ..BinaryOptionsToolsV2 import BinaryOptionsToolsV2 as mod

        return getattr(mod, name)
    except (ImportError, AttributeError):
        import BinaryOptionsToolsV2 as direct_mod

        return getattr(direct_mod, name)


class Logger:
    """
    A logger class wrapping the RustLogger functionality.

    Attributes:
        logger (RustLogger): The underlying RustLogger instance.
    """

    def __init__(self):
        RustLogger = _get_rust_attr("Logger")
        self.logger = RustLogger()


class LogBuilder:
    """
    A builder class for configuring the logs, create log layers and iterators.

    Attributes:
        builder (RustLogBuilder): The underlying RustLogBuilder instance.
    """

    def __init__(self):
        RustLogBuilder = _get_rust_attr("LogBuilder")
        self.builder = RustLogBuilder()


def start_logs(path: str, level: str = "DEBUG", terminal: bool = True, layers: list = None):
    """
    Initialize logging system for the application.

    Args:
        path (str): Path where log files will be stored.
        level (str): Logging level (default is "DEBUG").
        terminal (bool): Whether to display logs in the terminal (default is True).
        layers (list): Optional list of layers to initialize.

    Returns:
        None

    Raises:
        Exception: If there's an error starting the logging system.
    """
    if layers is None:
        layers = []

    start_tracing = _get_rust_attr("start_tracing")

    try:
        os.makedirs(path, exist_ok=True)
        start_tracing(path, level, terminal, layers)
    except Exception as e:
        print(f"Error starting logs: {e}")
