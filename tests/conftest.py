import sys
import os

# Add the package source directory to sys.path to resolve the package correctly
sys.path.insert(
    0,
    os.path.abspath(os.path.join(os.path.dirname(__file__), "../BinaryOptionsToolsV2")),
)

# Debug helper to verify import source
try:
    import BinaryOptionsToolsV2

    print(
        f"\n[TEST_ENV] BinaryOptionsToolsV2 loaded from: {BinaryOptionsToolsV2.__file__}"
    )
except Exception as e:
    print(f"\n[TEST_ENV] Failed to load BinaryOptionsToolsV2: {e}")
