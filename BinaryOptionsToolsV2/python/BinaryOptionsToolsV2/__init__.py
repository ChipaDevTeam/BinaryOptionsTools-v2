import importlib
import os
import sys

# Import the Rust module and re-export its attributes
try:
    _rust_module = importlib.import_module(".BinaryOptionsToolsV2", __package__)
except (ImportError, ValueError):
    try:
        # Fallback for when it's not in the package
        _rust_module = importlib.import_module("BinaryOptionsToolsV2")
        # Ensure we didn't just import the package itself
        if _rust_module is sys.modules.get(__package__):
            _rust_module = None
    except ImportError:
        _rust_module = None

if _rust_module:
    # Update globals with Rust module attributes
    globals().update({k: v for k, v in _rust_module.__dict__.items() if not k.startswith("_")})
else:
    # This is often okay during development/type checking, but bad for tests
    if os.environ.get("PYTEST_CURRENT_TEST"):
        print(f"[ERROR] Rust extension module 'BinaryOptionsToolsV2' not found! __package__={__package__}")
        print(f"[DEBUG] sys.path: {sys.path}")

# Import submodules for re-export
from . import tracing as tracing  # noqa: E402
from . import validator as validator  # noqa: E402
from .pocketoption import *  # noqa: F403, E402
from .pocketoption import __all__ as __pocket_all__  # noqa: E402

# Collect all core attributes for __all__
_core_names = [
    "RawPocketOption",
    "RawValidator",
    "RawHandler",
    "RawHandle",
    "Logger",
    "LogBuilder",
    "PyConfig",
    "PyBot",
    "PyStrategy",
    "PyContext",
    "PyVirtualMarket",
    "StreamLogsIterator",
    "StreamLogsLayer",
    "StreamIterator",
    "RawStreamIterator",
    "start_tracing",
]
__core_all__ = [name for name in _core_names if name in globals()]

__all__ = list(set(__pocket_all__ + ["tracing", "validator"] + __core_all__))
