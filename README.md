# dhall-python

`dhall-python` contains (WIP) [Dhall][dhall-lang] bindings for Python using the [rust][dhall-rust] implementation.

Install using pip:

```shell
python3 -m pip install --user dhall
```

Supports Windows, Mac OS, and Linux (with libssl.so.1 and libcrypto.so.1)

## Usage

python-dhall implements a similar API to Python's [json
module](https://docs.python.org/3/library/json.html):

```python
>>> import dhall
>>> dhall.dumps({"keyA": 81, "keyB": True, "keyC": "value"})
'{ keyA = 81, keyB = True, keyC = "value" }'
>>> dhall.loads("""{ keyA = 81, keyB = True, keyC = "value" }""")
{'keyA': 81, 'keyB': True, 'keyC': 'value'}
```

# License

python-dhall is licensed under either of

- Apache License, Version 2.0, (LICENSE-APACHE or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in python-dhall by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

All contributions are welcome! If you spot any bugs, or have any requests, 
issues and PRs are always welcome.

## Developer guide

This project uses [poetry](https://python-poetry.org/docs/) for managing the development environment. If you don't have it installed, run

```
curl -sSL https://raw.githubusercontent.com/python-poetry/poetry/master/get-poetry.py | python
export PATH="$HOME/.poetry/bin:$PATH"
```

The project requires the `nightly` version of Rust.

Install it via `rustup`:

```
rustup install nightly
```

If you have already installed the `nightly` version, make sure it is up-to-date:

```
rustup update nightly
```

After that, you can compile the current version of dhall-python and execute all tests and benchmarks with the following commands:

```
make install
make test
make bench
```

🤫 Pssst!... run `make help` to learn more.


# Links

- [dhall-rust](https://github.com/Nadrieril/dhall-rust)
- [dhall-lang](https://dhall-lang.org)
