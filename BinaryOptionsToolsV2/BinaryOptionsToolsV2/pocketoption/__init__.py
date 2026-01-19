"""
Module for Pocket Option related functionality.

Contains asynchronous and synchronous clients,
as well as specific classes for Pocket Option trading.
"""

__all__ = [
    "asynchronous",
    "synchronous",
    "PocketOptionAsync",
    "PocketOption",
    "RawHandler",
    "RawHandlerSync",
]

from . import asynchronous, synchronous
from .asynchronous import PocketOptionAsync, RawHandler
from .synchronous import PocketOption, RawHandlerSync
