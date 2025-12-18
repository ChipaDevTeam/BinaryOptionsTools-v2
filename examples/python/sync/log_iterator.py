# Import necessary modules
import time
from datetime import timedelta
from multiprocessing import Process

from BinaryOptionsToolsV2.tracing import LogBuilder, Logger


def main():
    """
    Main synchronous function demonstrating the usage of logging system.
    """

    # Create a Logger instance
    logger = Logger()

    # Create a LogBuilder instance
    log_builder = LogBuilder()

    # Create a new logs iterator with INFO level and 10-second timeout
    log_iterator = log_builder.create_logs_iterator(
        level="INFO", timeout=timedelta(seconds=10)
    )

    # Configure logging to write to a file
    # This will create or append to 'logs.log' file with INFO level logs
    log_builder.log_file(path="app_logs.txt", level="INFO")

    # Configure terminal logging for DEBUG level
    log_builder.terminal(level="DEBUG")

    # Build and initialize the logging configuration
    log_builder.build()

    # Create a Logger instance with the built configuration
    logger = Logger()

    # Log some messages at different levels
    logger.debug("This is a debug message")
    logger.info("This is an info message")
    logger.warn("This is a warning message")
    logger.error("This is an error message")

    # Example of logging with variables
    asset = "EURUSD"
    amount = 100
    logger.info(f"Bought {amount} units of {asset}")

    # Demonstrate sync usage
    def log_sync():
        """
        Syncronous logging function demonstrating sync usage.
        """
        logger.debug("This is a synchronous debug message")
        time.sleep(5)  # Simulate some work
        logger.info("Sync operation completed")

    # Run the sync function
    task1 = Process(target=log_sync())
    task1.start()

    # Example of using LogBuilder for creating iterators
    def process_logs(log_iterator):
        """
        Function demonstrating the use of LogSubscription.
        """

        try:
            for log in log_iterator:
                print(f"Received log: {log}")
                # Each log is a dict so we can access the message
                print(f"Log message: {log['message']}")
        except Exception as e:
            print(f"Error processing logs: {e}")

    # Run the logs processing function
    task2 = Process(target=process_logs(log_iterator))
    task2.start()
    # Execute both tasks at the same time
    task1.join()
    task2.join()


if __name__ == "__main__":
    main()
