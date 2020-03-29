# python-dhall

`python-dhall` contains (WIP) [Dhall][dhall-lang] bindings for Python using the [rust][dhall-rust] implementation.

The binding implements a `loads` function that returns a python object similar to JSON:

```shell
python3 -c 'import dhall; print(dhall.loads("""{ version = 21 + 21, name = "a test", req = ["itemA", "itemB"], bool = True && False }"""))'
{'bool': False, 'name': 'a test', 'req': ['itemA', 'itemB'], 'version': 42}
```

[dhall-rust]: https://github.com/Nadrieril/dhall-rust
[dhall-lang]: https://dhall-lang.org
