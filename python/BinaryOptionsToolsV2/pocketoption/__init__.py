"""
Module for Pocket Option related functionality.

Contains asynchronous and synchronous clients,
as well as specific classes for Pocket Option trading.
"""

__all__ = [
    "asynchronous",
    "login",
    "login_async",
    "PocketOptionAsync",
    "PocketOption",
    "RawHandler",
    "RawHandlerSync",
    "Validator",
    "Trade",
    "TradeResult",
]
from .tools.login import login, login_async
from . import asynchronous, synchronous as synchronous
from .asynchronous import PocketOptionAsync, RawHandler, Validator
from .synchronous import PocketOption, RawHandlerSync
from .models import Trade, TradeResult
