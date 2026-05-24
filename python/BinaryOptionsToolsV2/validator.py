import sys
from typing import Callable, List


def _get_raw_validator():
    """Get RawValidator class from compiled Rust module via package namespace."""
    pkg = sys.modules.get(__package__ or "")
    if pkg is not None and hasattr(pkg, "RawValidator"):
        return pkg.RawValidator
    import BinaryOptionsToolsV2 as _mod

    return _mod.RawValidator


class Validator:
    """
    A high-level wrapper for RawValidator that provides message validation functionality.

    Example:
        ```python
        validator = Validator.starts_with("Hello")
        assert validator.check("Hello World") == True
        ```
    """

    def __init__(self, raw=None):
        self._validator = raw if raw is not None else _get_raw_validator()()

    @staticmethod
    def regex(pattern: str) -> "Validator":
        return Validator(_get_raw_validator().regex(pattern))

    @staticmethod
    def starts_with(prefix: str) -> "Validator":
        return Validator(_get_raw_validator().starts_with(prefix))

    @staticmethod
    def ends_with(suffix: str) -> "Validator":
        return Validator(_get_raw_validator().ends_with(suffix))

    @staticmethod
    def contains(substring: str) -> "Validator":
        return Validator(_get_raw_validator().contains(substring))

    @staticmethod
    def ne(validator: "Validator") -> "Validator":
        return Validator(_get_raw_validator().ne(validator._validator))

    @staticmethod
    def all(validators: List["Validator"]) -> "Validator":
        return Validator(
            _get_raw_validator().all([item._validator for item in validators])
        )

    @staticmethod
    def any(validators: List["Validator"]) -> "Validator":
        return Validator(
            _get_raw_validator().any([item._validator for item in validators])
        )

    @staticmethod
    def custom(func: Callable[[str], bool]) -> "Validator":
        if not callable(func):
            raise TypeError("func must be callable")
        return Validator(_get_raw_validator().custom(func))

    def check(self, message: str) -> bool:
        return self._validator.check(message)

    @property
    def raw_validator(self):
        return self._validator

    def __eq__(self, other):
        if not isinstance(other, Validator):
            return NotImplemented
        return type(self._validator) is type(other._validator) and str(self._validator) == str(other._validator)

    def __repr__(self):
        return f"Validator({self._validator!r})"
