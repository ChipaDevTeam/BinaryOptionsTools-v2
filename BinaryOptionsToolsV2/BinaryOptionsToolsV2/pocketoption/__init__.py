"""
Module for Pocket Option related functionality.

Contains asynchronous and synchronous clients,
as well as specific classes for Pocket Option trading.
"""

__all__ = ["asyncronous", "syncronous", "PocketOptionAsync", "PocketOption"]

from . import asyncronous, syncronous
from .asyncronous import PocketOptionAsync
from .syncronous import PocketOption
