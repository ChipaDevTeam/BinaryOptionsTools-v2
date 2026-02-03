import importlib

# Import the Rust module and re-export its attributes
try:
    _rust_module = importlib.import_module(".BinaryOptionsToolsV2", __package__)
except (ImportError, ValueError):
    try:
        # Fallback for when it's not in the package
        _rust_module = importlib.import_module("BinaryOptionsToolsV2")
    except ImportError:
        _rust_module = None

if _rust_module:
    globals().update({k: v for k, v in _rust_module.__dict__.items() if not k.startswith("_")})

from . import tracing, validator
from .pocketoption import *  # noqa: F403

__core_all__ = getattr(_rust_module, "__all__", []) if _rust_module else []
from .pocketoption import __all__ as __pocket_all__

__all__ = __pocket_all__ + ["tracing", "validator"] + __core_all__
