import sys
import os

# Add the package source directory to sys.path to resolve the package correctly
# This is necessary because the root directory has the same name as the package directory
sys.path.insert(
    0,
    os.path.abspath(os.path.join(os.path.dirname(__file__), "../BinaryOptionsToolsV2")),
)
