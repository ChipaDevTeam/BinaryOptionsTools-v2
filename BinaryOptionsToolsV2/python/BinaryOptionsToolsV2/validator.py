from typing import Callable, List


def _get_raw_validator():
    """Helper to get the RawValidator class from the compiled Rust module."""
    try:
        # First try to import from the package (which should have re-exported it in __init__.py)
        from . import RawValidator

        return RawValidator
    except (ImportError, AttributeError):
        # Fallback to direct import if package initialization is incomplete
        try:
            from ..BinaryOptionsToolsV2 import RawValidator

            return RawValidator
        except ImportError:
            import BinaryOptionsToolsV2

            return getattr(BinaryOptionsToolsV2, "RawValidator")


class Validator:
    """
    A high-level wrapper for RawValidator that provides message validation functionality.

    This class provides various methods to validate WebSocket messages using different
    strategies like regex matching, prefix/suffix checking, and logical combinations.

    Example:
        ```python
        # Simple validation
        validator = Validator.starts_with("Hello")
        assert validator.check("Hello World") == True

        # Combined validation
        v1 = Validator.regex(r"[A-Z]\\w+")  # Starts with capital letter
        v2 = Validator.contains("World")    # Contains "World"
        combined = Validator.all([v1, v2])  # Must satisfy both conditions
        assert combined.check("Hello World") == True
        ```
    """

    def __init__(self):
        """Creates a default validator that accepts all messages."""
        self._validator = _get_raw_validator()()

    @staticmethod
    def regex(pattern: str) -> "Validator":
        """
        Creates a validator that uses regex pattern matching.

        Args:
            pattern: Regular expression pattern

        Returns:
            Validator that matches messages against the pattern
        """
        v = Validator()
        v._validator = _get_raw_validator().regex(pattern)
        return v

    @staticmethod
    def starts_with(prefix: str) -> "Validator":
        """
        Creates a validator that checks if messages start with a specific prefix.

        Args:
            prefix: String that messages should start with

        Returns:
            Validator that matches messages starting with prefix
        """
        v = Validator()
        v._validator = _get_raw_validator().starts_with(prefix)
        return v

    @staticmethod
    def ends_with(suffix: str) -> "Validator":
        """
        Creates a validator that checks if messages end with a specific suffix.

        Args:
            suffix: String that messages should end with

        Returns:
            Validator that matches messages ending with suffix
        """
        v = Validator()
        v._validator = _get_raw_validator().ends_with(suffix)
        return v

    @staticmethod
    def contains(substring: str) -> "Validator":
        """
        Creates a validator that checks if messages contain a specific substring.

        Args:
            substring: String that should be present in messages

        Returns:
            Validator that matches messages containing substring
        """
        v = Validator()
        v._validator = _get_raw_validator().contains(substring)
        return v

    @staticmethod
    def ne(validator: "Validator") -> "Validator":
        """
        Creates a validator that negates another validator's result.

        Args:
            validator: Validator whose result should be negated

        Returns:
            Validator that returns True when input validator returns False
        """
        v = Validator()
        v._validator = _get_raw_validator().ne(validator._validator)
        return v

    @staticmethod
    def all(validators: List["Validator"]) -> "Validator":
        """
        Creates a validator that requires all input validators to match.

        Args:
            validators: List of validators that all must match

        Returns:
            Validator that returns True only if all input validators return True
        """
        v = Validator()
        v._validator = _get_raw_validator().all([item._validator for item in validators])
        return v

    @staticmethod
    def any(validators: List["Validator"]) -> "Validator":
        """
        Creates a validator that requires at least one input validator to match.

        Args:
            validators: List of validators where at least one must match

        Returns:
            Validator that returns True if any input validator returns True
        """
        v = Validator()
        v._validator = _get_raw_validator().any([item._validator for item in validators])
        return v

    @staticmethod
    def custom(func: Callable[[str], bool]) -> "Validator":
        """
        Creates a validator that uses a custom Python function for validation.

        IMPORTANT USAGE NOTES:
        1. The provided function MUST:
            - Take exactly one string parameter (the WebSocket message).
            - Return a boolean value.
            - Be synchronous (not async).
        2. If the function raises an exception, the validator will silently return False.
        3. Custom validators should be used carefully in multi-threaded contexts due to
           Python Global Interpreter Lock (GIL) constraints.

        Args:
            func: A callable that takes a string message and returns a boolean.
                Returns True if the message is valid, False otherwise.

        Returns:
            Validator: A new validator instance using the provided callback.
        """
        v = Validator()
        v._validator = _get_raw_validator().custom(func)
        return v

    def check(self, message: str) -> bool:
        """
        Checks if a message matches this validator's conditions.

        Args:
            message: String to validate

        Returns:
            True if message matches the validator's conditions, False otherwise
        """
        return self._validator.check(message)

    @property
    def raw_validator(self):
        """
        Returns the underlying RawValidator instance.

        This is mainly used internally by the library but can be useful
        for advanced use cases.
        """
        return self._validator
