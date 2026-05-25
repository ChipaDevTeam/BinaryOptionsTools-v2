import importlib
import os
import sys
from .config import Config as Config
from . import tracing as tracing
from . import validator as validator
from .pocketoption import PocketOptionAsync, PocketOption, __all__ as __pocket_all__  # noqa: F401

# Import the Rust module and re-export its attributes
_rust_module = None
try:
    _rust_module = importlib.import_module(".BinaryOptionsToolsV2", __package__)
except (ImportError, ValueError):
    try:
        _rust_module = importlib.import_module("BinaryOptionsToolsV2")
        if _rust_module is sys.modules.get(__package__):
            _rust_module = None
    except ImportError:
        pass

if _rust_module is not None:
    globals().update({k: v for k, v in _rust_module.__dict__.items() if not k.startswith("_")})
elif os.environ.get("PYTEST_CURRENT_TEST"):
    print(f"[ERROR] Rust extension module not found (__package__={__package__})")

# Names expected from the Rust cdylib; only those actually loaded will be available
_rust_exported_names = [
    "RawPocketOption", "RawValidator", "RawHandler", "RawHandle",
    "Logger", "LogBuilder", "PyConfig", "PyBot", "PyStrategy",
    "PyContext", "PyVirtualMarket", "Action",
    "StreamLogsIterator", "StreamLogsLayer", "StreamIterator",
    "RawStreamIterator", "start_tracing",
]
__rust_all__ = [n for n in _rust_exported_names if n in globals()]

__all__ = list(set(__pocket_all__ + ["tracing", "validator", "PocketOptionAsync", "PocketOption"] + __rust_all__))
