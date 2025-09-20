from typing import List


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
        v1 = Validator.regex(r"[A-Z]\w+")  # Starts with capital letter
        v2 = Validator.contains("World")    # Contains "World"
        combined = Validator.all([v1, v2])  # Must satisfy both conditions
        assert combined.check("Hello World") == True
        ```
    """

    def __init__(self):
        """Creates a default validator that accepts all messages."""
        from BinaryOptionsToolsV2 import RawValidator

        self._validator = RawValidator()

    @staticmethod
    def regex(pattern: str) -> "Validator":
        """
        Creates a validator that uses regex pattern matching.

        Args:
            pattern: Regular expression pattern

        Returns:
            Validator that matches messages against the pattern

        Example:
            ```python
            # Match messages starting with a number
            validator = Validator.regex(r"^\d+")
            assert validator.check("123 message") == True
            assert validator.check("abc") == False
            ```
        """
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.regex(pattern)
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
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.starts_with(prefix)
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
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.ends_with(suffix)
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
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.contains(substring)
        return v

    @staticmethod
    def ne(validator: "Validator") -> "Validator":
        """
        Creates a validator that negates another validator's result.

        Args:
            validator: Validator whose result should be negated

        Returns:
            Validator that returns True when input validator returns False

        Example:
            ```python
            # Match messages that don't contain "error"
            v = Validator.ne(Validator.contains("error"))
            assert v.check("success message") == True
            assert v.check("error occurred") == False
            ```
        """
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.ne(validator._validator)
        return v

    @staticmethod
    def all(validators: List["Validator"]) -> "Validator":
        """
        Creates a validator that requires all input validators to match.

        Args:
            validators: List of validators that all must match

        Returns:
            Validator that returns True only if all input validators return True

        Example:
            ```python
            # Match messages that start with "Hello" and end with "World"
            v = Validator.all([
                Validator.starts_with("Hello"),
                Validator.ends_with("World")
            ])
            assert v.check("Hello Beautiful World") == True
            assert v.check("Hello Beautiful") == False
            ```
        """
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.all([v._validator for v in validators])
        return v

    @staticmethod
    def any(validators: List["Validator"]) -> "Validator":
        """
        Creates a validator that requires at least one input validator to match.

        Args:
            validators: List of validators where at least one must match

        Returns:
            Validator that returns True if any input validator returns True

        Example:
            ```python
            # Match messages containing either "success" or "completed"
            v = Validator.any([
                Validator.contains("success"),
                Validator.contains("completed")
            ])
            assert v.check("operation successful") == True
            assert v.check("task completed") == True
            assert v.check("in progress") == False
            ```
        """
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.any([v._validator for v in validators])
        return v

    @staticmethod
    def custom(func: callable) -> "Validator":
        """
        Creates a validator that uses a custom function for validation.

        IMPORTANT SAFETY AND USAGE NOTES:
        1. The provided function MUST:
            - Take exactly one string parameter
            - Return a boolean value
            - Be synchronous (not async)
        2. If these requirements are not met, the program will crash with a Rust panic
        that CANNOT be caught with try/except
        3. The function will be called from Rust, so Python exception handling won't work
        4. Custom validators CANNOT be used in async/threaded contexts due to JavaScript
        engine limitations

        Args:
            func: A callable that takes a string message and returns a boolean.
                The function MUST follow the requirements listed above.
                Returns True if the message is valid, False otherwise.

        Returns:
            Validator that uses the custom function for validation

        Raises:
            Rust panic: If the function doesn't meet the requirements or fails during execution.
            This cannot be caught with Python exception handling.

        Example:
            ```python
            # Safe usage - function takes string, returns bool
            def json_checker(msg: str) -> bool:
                try:
                    data = json.loads(msg)
                    return "status" in data and "timestamp" in data
                except:
                    return False

            validator = Validator.custom(json_checker)
            assert validator.check('{"status": "ok", "timestamp": 123}') == True
            assert validator.check('{"error": "failed"}') == False

            # Using lambda (must still take string, return bool)
            contains_both = Validator.custom(
                lambda msg: "success" in msg and "completed" in msg
            )
            assert contains_both.check("operation success - completed") == True

            # UNSAFE - Will crash the program:
            # Wrong return type
            bad_validator1 = Validator.custom(lambda x: "hello")  # Returns str instead of bool

            # No exception handling possible
            def will_crash(msg: str) -> bool:
                raise ValueError("This will crash the program")

            bad_validator2 = Validator.custom(will_crash)
            try:
                bad_validator2.check("any message")  # Will crash despite try/except
            except Exception:
                print("This will never be reached")
            ```
        """
        from BinaryOptionsToolsV2 import RawValidator

        v = Validator()
        v._validator = RawValidator.custom(func)
        return v

    def check(self, message: str) -> bool:
        """
        Checks if a message matches this validator's conditions.

        Args:
            message: String to validate

        Returns:
            True if message matches the validator's conditions, False otherwise
        """
        # return self._validator.check(message)
        raise NotImplementedError(
            "The check method is not implemented in the Validator class. "
            "Please use a RawValidator instance directly or implement this method."
        )

    @property
    def raw_validator(self):
        """
        Returns the underlying RawValidator instance.

        This is mainly used internally by the library but can be useful
        for advanced use cases.
        """
        return self._validator
