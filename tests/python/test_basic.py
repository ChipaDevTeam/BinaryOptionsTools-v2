import BinaryOptionsToolsV2


def test_module_import():
    """Verify that the module can be imported and exposes expected attributes."""
    assert BinaryOptionsToolsV2 is not None
    # Check for some expected classes or modules based on __init__.py and lib.rs
    assert hasattr(BinaryOptionsToolsV2, "PocketOption")
    assert hasattr(BinaryOptionsToolsV2, "PocketOptionAsync")


def test_simple_math():
    """A dummy test to ensure pytest is working."""
    assert 1 + 1 == 2
