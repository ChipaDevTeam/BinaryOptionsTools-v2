"""Guards that the synchronous client mirrors the asynchronous client's API.

The sync client is a thin thread bridge over ``PocketOptionAsync``; if a method
is added or renamed on one side only, the documented surface of the two clients
silently diverges. These tests read the class definitions directly so they do
not need a live connection.
"""

import ast
import pathlib

from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync
from BinaryOptionsToolsV2.pocketoption.synchronous import PocketOption

PACKAGE = (
    pathlib.Path(__file__).resolve().parents[3]
    / "python"
    / "BinaryOptionsToolsV2"
    / "pocketoption"
)

# Sync-only methods: event-loop plumbing and underlying-client accessors.
SYNC_ONLY = {"client", "close", "config", "loop"}


def _methods(path: pathlib.Path, class_name: str):
    tree = ast.parse(path.read_text(encoding="utf-8"))
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            return [
                child
                for child in node.body
                if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef))
                and not child.name.startswith("_")
            ]
    raise AssertionError(f"{class_name} not found in {path}")


def _public_methods(path: pathlib.Path, class_name: str) -> set[str]:
    return {fn.name for fn in _methods(path, class_name)}


def test_sync_client_mirrors_async_client():
    api = _public_methods(PACKAGE / "asynchronous.py", "PocketOptionAsync")
    sync = _public_methods(PACKAGE / "synchronous.py", "PocketOption")
    assert api - sync == set()
    assert sync - api - SYNC_ONLY == set()


def test_overlapping_methods_are_documented():
    api = _public_methods(PACKAGE / "asynchronous.py", "PocketOptionAsync")
    sync = _public_methods(PACKAGE / "synchronous.py", "PocketOption")
    for name in sorted(api & sync):
        assert getattr(PocketOptionAsync, name).__doc__, (
            f"PocketOptionAsync.{name} has no docstring"
        )
        assert getattr(PocketOption, name).__doc__, (
            f"PocketOption.{name} has no docstring"
        )


def test_argument_taking_methods_are_documented_or_deprecated():
    """Every public method that takes arguments is either fully documented or a
    deprecated shim that points at its replacement."""
    offenders = []
    for path, class_name in (
        (PACKAGE / "asynchronous.py", "PocketOptionAsync"),
        (PACKAGE / "synchronous.py", "PocketOption"),
    ):
        for fn in _methods(path, class_name):
            params = [a.arg for a in fn.args.args if a.arg != "self"]
            if not params:
                continue
            doc = (ast.get_docstring(fn) or "").strip()
            if "Args:" in doc or doc.startswith("Deprecated:"):
                continue
            offenders.append(f"{class_name}.{fn.name}")
    assert not offenders, f"undocumented methods: {offenders}"
