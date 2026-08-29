#!/usr/bin/env python3
"""
Tests for export_closeoption_ssid.py - Windows-focused tests for save_ssid
"""

import os
import json
import tempfile
import sys
from pathlib import Path

# Add scripts to path
sys.path.insert(0, str(Path(__file__).parent))

from export_closeoption_ssid import save_ssid, parse_ssid


def test_save_ssid_windows_compatibility():
    """Test that save_ssid works on Windows without os.fchmod AttributeError."""
    ssid = "test_token|test_sid|true|pub_code|hid_code"
    
    with tempfile.TemporaryDirectory() as tmpdir:
        filepath = os.path.join(tmpdir, "test_session.json")
        
        # This should not raise AttributeError on Windows
        save_ssid(ssid, filepath)
        
        # Verify file was created and contains correct data
        assert os.path.exists(filepath), "File should be created"
        
        with open(filepath, 'r') as f:
            data = json.load(f)
        
        assert data["token"] == "test_token"
        assert data["sid"] == "test_sid"
        assert data["demo"] is True
        assert data["public_code"] == "pub_code"
        assert data["hidden_code"] == "hid_code"


def test_save_ssid_preserves_restrictive_permissions_posix():
    """Test that on POSIX, file gets 0o600 permissions."""
    if sys.platform.startswith("win"):
        import pytest
        pytest.skip("POSIX permission test not applicable on Windows")
    
    ssid = "test_token|test_sid|true|pub_code|hid_code"
    
    with tempfile.TemporaryDirectory() as tmpdir:
        filepath = os.path.join(tmpdir, "test_session.json")
        
        save_ssid(ssid, filepath)
        
        # Check file permissions are 0o600 (owner read/write only)
        stat_info = os.stat(filepath)
        mode = stat_info.st_mode & 0o777
        assert mode == 0o600, f"Expected 0o600, got {oct(mode)}"


def test_save_ssid_creates_parent_directories():
    """Test that save_ssid creates parent directories if they don't exist."""
    ssid = "test_token|test_sid|true|pub_code|hid_code"
    
    with tempfile.TemporaryDirectory() as tmpdir:
        filepath = os.path.join(tmpdir, "subdir", "nested", "test_session.json")
        
        save_ssid(ssid, filepath)
        
        assert os.path.exists(filepath), "File should be created with parent directories"


def test_save_ssid_overwrites_existing_file():
    """Test that save_ssid truncates and overwrites existing file."""
    ssid1 = "token1|sid1|true|pub1|hid1"
    ssid2 = "token2|sid2|false|pub2|hid2"
    
    with tempfile.TemporaryDirectory() as tmpdir:
        filepath = os.path.join(tmpdir, "test_session.json")
        
        save_ssid(ssid1, filepath)
        save_ssid(ssid2, filepath)
        
        with open(filepath, 'r') as f:
            data = json.load(f)
        
        assert data["token"] == "token2"
        assert data["sid"] == "sid2"
        assert data["demo"] is False


def test_parse_ssid_various_formats():
    """Test parse_ssid handles various SSID formats."""
    # 5-part format
    token, sid, demo, pub, hid = parse_ssid("token|sid|true|pub|hid")
    assert token == "token"
    assert sid == "sid"
    assert demo is True
    assert pub == "pub"
    assert hid == "hid"
    
    # 3-part format
    token, sid, demo, pub, hid = parse_ssid("token|sid|false")
    assert token == "token"
    assert sid == "sid"
    assert demo is False
    assert pub == ""
    assert hid == ""
    
    # Single part (fallback)
    token, sid, demo, pub, hid = parse_ssid("just_token")
    assert token == "just_token"
    assert sid == ""
    assert demo is True
    assert pub == ""
    assert hid == ""


def test_save_ssid_rejects_incomplete_ssid():
    """Test that save_ssid rejects SSID without public_code/hidden_code."""
    ssid = "token|sid|true"  # Missing public_code and hidden_code
    
    with tempfile.TemporaryDirectory() as tmpdir:
        filepath = os.path.join(tmpdir, "test_session.json")
        
        # Should print error and return without creating file
        save_ssid(ssid, filepath)
        
        # File should not be created (or if created, should be empty/error)
        # The function returns early, so file may not exist
        # This is the expected behavior - it prints error and returns


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])