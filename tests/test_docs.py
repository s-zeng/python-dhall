import dhall


def test_docs():
    assert (
        dhall.dumps({"keyA": 81, "keyB": True, "keyC": "value"})
        == '{ keyA = 81, keyB = True, keyC = "value" }'
    )
    assert dhall.loads("""{ keyA = 81, keyB = True, keyC = "value" }""") == {
        "keyA": 81,
        "keyB": True,
        "keyC": "value",
    }
