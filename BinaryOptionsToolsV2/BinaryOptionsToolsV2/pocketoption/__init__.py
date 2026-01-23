"""
Module for Pocket Option related functionality.

Contains asynchronous and synchronous clients,
as well as specific classes for Pocket Option trading.
"""

__all__ = [
    "asyncronous",
    "syncronous",
    "PocketOptionAsync",
    "PocketOption",
    "RawHandler",
    "RawHandlerSync",
]

from . import asyncronous, syncronous
from .asyncronous import PocketOptionAsync, RawHandler
from .syncronous import PocketOption, RawHandlerSync
