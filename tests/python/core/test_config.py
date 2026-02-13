import pytest
from BinaryOptionsToolsV2.config import Config


def test_config_initialization():
    cfg = Config(max_allowed_loops=200, log_level="DEBUG")
    assert cfg.max_allowed_loops == 200
    assert cfg.log_level == "DEBUG"
    assert cfg.urls == []


def test_config_locking():
    cfg = Config()
    cfg.max_allowed_loops = 150
    # Accessing pyconfig should lock it
    pycfg = cfg.pyconfig
    assert pycfg.max_allowed_loops == 150

    with pytest.raises(RuntimeError, match="locked"):
        cfg.max_allowed_loops = 200

    with pytest.raises(RuntimeError, match="locked"):
        cfg.update({"sleep_interval": 50})


def test_config_from_dict():
    data = {"max_allowed_loops": 300, "invalid_key": "ignore me"}
    cfg = Config.from_dict(data)
    assert cfg.max_allowed_loops == 300
    assert not hasattr(cfg, "invalid_key")


def test_config_from_json():
    json_data = '{"reconnect_time": 10, "log_level": "WARN"}'
    cfg = Config.from_json(json_data)
    assert cfg.reconnect_time == 10
    assert cfg.log_level == "WARN"


def test_config_to_dict_json():
    cfg = Config(reconnect_time=7)
    d = cfg.to_dict()
    assert d["reconnect_time"] == 7

    j = cfg.to_json()
    assert '"reconnect_time": 7' in j


def test_config_update():
    cfg = Config()
    cfg.update({"timeout_secs": 45, "log_level": "ERROR"})
    assert cfg.timeout_secs == 45
    assert cfg.log_level == "ERROR"
