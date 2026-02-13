from BinaryOptionsToolsV2.validator import Validator


def test_validator_starts_with():
    v = Validator.starts_with("Hello")
    assert v.check("Hello World") is True
    assert v.check("Hi World") is False


def test_validator_ends_with():
    v = Validator.ends_with("World")
    assert v.check("Hello World") is True
    assert v.check("Hello") is False


def test_validator_contains():
    v = Validator.contains("Beautiful")
    assert v.check("Hello Beautiful World") is True
    assert v.check("Hello World") is False


def test_validator_regex():
    v = Validator.regex(r"^\d{3}-\d{3}$")
    assert v.check("123-456") is True
    assert v.check("123-45") is False
    assert v.check("abc-def") is False


def test_validator_all():
    v1 = Validator.starts_with("Hello")
    v2 = Validator.contains("World")
    v_all = Validator.all([v1, v2])

    assert v_all.check("Hello World") is True
    assert v_all.check("Hello") is False
    assert v_all.check("World") is False


def test_validator_any():
    v1 = Validator.starts_with("Hello")
    v2 = Validator.starts_with("Hi")
    v_any = Validator.any([v1, v2])

    assert v_any.check("Hello World") is True
    assert v_any.check("Hi World") is True
    assert v_any.check("Hey World") is False


def test_validator_ne():
    v = Validator.ne(Validator.contains("Error"))
    assert v.check("Success") is True
    assert v.check("Error occurred") is False


def test_validator_custom():
    def my_check(msg: str) -> bool:
        return len(msg) > 5

    v = Validator.custom(my_check)
    assert v.check("123456") is True
    assert v.check("12345") is False


def test_validator_complex_combination():
    # Starts with { or [, and contains "id"
    v_start = Validator.any([Validator.starts_with("{"), Validator.starts_with("[")])
    v_id = Validator.contains('"id"')
    v_complex = Validator.all([v_start, v_id])

    assert v_complex.check('{"id": 1}') is True
    assert v_complex.check('[{"id": 1}]') is True
    assert v_complex.check('{"name": "test"}') is False
    assert v_complex.check("id is 1") is False
