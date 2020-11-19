#!/usr/bin/env bash

cargo fmt
autoflake . -r --remove-all-unused-imports -i
isort . -rc
black .
