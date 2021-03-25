#!/usr/bin/env bash

BASE=$(dirname "$0")
PROJECT=$(realpath "$BASE/../..")

cat "$PROJECT/Cargo.toml" | grep version | head -n 1 | sed -e "s/version = //g" | sed -e "s/\"//g"
