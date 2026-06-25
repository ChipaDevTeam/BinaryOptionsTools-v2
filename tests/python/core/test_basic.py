import importlib
import os
import sys
from unittest.mock import MagicMock, patch

import BinaryOptionsToolsV2


def test_module_import():
    """Verify that the module can be imported and exposes expected attributes."""
    assert BinaryOptionsToolsV2 is not None
    assert hasattr(BinaryOptionsToolsV2, "PocketOption")
    assert hasattr(BinaryOptionsToolsV2, "PocketOptionAsync")
    assert hasattr(BinaryOptionsToolsV2, "RawPocketOption")


def test_init_import_fallbacks():
    """Test import error pathways in __init__.py by reloading the module with different mocks."""
    original_import_module = importlib.import_module

    # Case 1: First import throws ImportError, fallback succeeds
    mock_rust = MagicMock()
    mock_rust.__dict__ = {"some_rust_attr_mock": "value"}
    
    def side_effect(name, package=None):
        if name == ".BinaryOptionsToolsV2":
            raise ImportError("mock fail")
        if name == "BinaryOptionsToolsV2":
            return mock_rust
        return original_import_module(name, package)

    with patch("importlib.import_module", side_effect=side_effect):
        importlib.reload(BinaryOptionsToolsV2)
        assert getattr(BinaryOptionsToolsV2, "some_rust_attr_mock", None) == "value"

    # Case 2: First import throws ValueError, fallback succeeds
    def side_effect_val(name, package=None):
        if name == ".BinaryOptionsToolsV2":
            raise ValueError("mock fail")
        if name == "BinaryOptionsToolsV2":
            return mock_rust
        return original_import_module(name, package)

    with patch("importlib.import_module", side_effect=side_effect_val):
        importlib.reload(BinaryOptionsToolsV2)
        assert getattr(BinaryOptionsToolsV2, "some_rust_attr_mock", None) == "value"

    # Case 3: Fallback matches current package, resulting in None
    def side_effect_self(name, package=None):
        if name == ".BinaryOptionsToolsV2":
            raise ImportError("mock fail")
        if name == "BinaryOptionsToolsV2":
            # Temporarily simulate that the returned module is the package itself to trigger the recursion guard
            return sys.modules.get("BinaryOptionsToolsV2")
        return original_import_module(name, package)

    with patch("importlib.import_module", side_effect=side_effect_self):
        importlib.reload(BinaryOptionsToolsV2)
        assert getattr(BinaryOptionsToolsV2, "_rust_module", None) is None

    # Case 4: Both fail, PYTEST_CURRENT_TEST is set
    def side_effect_fail_all(name, package=None):
        if name in (".BinaryOptionsToolsV2", "BinaryOptionsToolsV2"):
            raise ImportError("mock fail")
        return original_import_module(name, package)

    with patch("importlib.import_module", side_effect=side_effect_fail_all), \
         patch.dict("os.environ", {"PYTEST_CURRENT_TEST": "1"}), \
         patch("builtins.print") as mock_print:
        importlib.reload(BinaryOptionsToolsV2)
        mock_print.assert_any_call("[ERROR] Rust extension module not found (__package__=BinaryOptionsToolsV2)")

    # Restore the module to a clean state by reloading without mocks
    importlib.reload(BinaryOptionsToolsV2)
