import json
from datetime import timedelta

from .BinaryOptionsToolsV2 import LogBuilder as RustLogBuilder
from .BinaryOptionsToolsV2 import Logger as RustLogger
from .BinaryOptionsToolsV2 import start_tracing


class LogSubscription:
    def __init__(self, subscription):
        self.subscription = subscription

    def __aiter__(self):
        return self

    async def __anext__(self):
        return json.loads(await anext(self.subscription))

    def __iter__(self):
        return self

    def __next__(self):
        return json.loads(next(self.subscription))


def start_logs(
    path: str, level: str = "DEBUG", terminal: bool = True, layers: list = None
):
    """
    Initialize logging system for the application.

    Args:
        path (str): Path where log files will be stored.
        level (str): Logging level (default is "DEBUG").
        terminal (bool): Whether to display logs in the terminal (default is True).

    Returns:
        None

    Raises:
        Exception: If there's an error starting the logging system.
    """
    if layers is None:
        layers = []
    try:
        start_tracing(path, level, terminal, layers)
    except Exception as e:
        print(f"Error starting logs, {e}")


class Logger:
    """
    A logger class wrapping the RustLogger functionality.

    Attributes:
        logger (RustLogger): The underlying RustLogger instance.
    """

    def __init__(self):
        self.logger = RustLogger()

    def debug(self, message):
        """
        Log a debug message.

        Args:
            message (str): The message to log.
        """
        self.logger.debug(str(message))

    def info(self, message):
        """
        Log an informational message.

        Args:
            message (str): The message to log.
        """
        self.logger.info(str(message))

    def warn(self, message):
        """
        Log a warning message.

        Args:
            message (str): The message to log.
        """
        self.logger.warn(str(message))

    def error(self, message):
        """
        Log an error message.

        Args:
            message (str): The message to log.
        """
        self.logger.error(str(message))


class LogBuilder:
    """
    A builder class for configuring the logs, create log layers and iterators.

    Attributes:
        builder (RustLogBuilder): The underlying RustLogBuilder instance.
    """

    def __init__(self):
        self.builder = RustLogBuilder()

    def create_logs_iterator(
        self, level: str = "DEBUG", timeout: None | timedelta = None
    ) -> LogSubscription:
        """
        Create a new logs iterator with the specified level and timeout.

        Args:
            level (str): The logging level (default is "DEBUG").
            timeout (None | timedelta): Optional timeout for the iterator.

        Returns:
            StreamLogsIterator: A new StreamLogsIterator instance that supports both asynchronous and synchronousiterators.
        """
        return LogSubscription(self.builder.create_logs_iterator(level, timeout))

    def log_file(self, path: str = "logs.log", level: str = "DEBUG"):
        """
        Configure logging to a file.

        Args:
            path (str): The path where logs will be stored (default is "logs.log").
            level (str): The minimum log level for this file handler.
        """
        self.builder.log_file(path, level)

    def terminal(self, level: str = "DEBUG"):
        """
        Configure logging to the terminal.

        Args:
            level (str): The minimum log level for this terminal handler.
        """
        self.builder.terminal(level)

    def build(self):
        """
        Build and initialize the logging configuration. This function should be called only once per execution.
        """
        self.builder.build()
