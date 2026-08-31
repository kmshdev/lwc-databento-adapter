#!/bin/sh
set -eu

if [ "$#" -ne 0 ]; then
  printf '%s\n' 'not run: this task discovers account-aware metadata and accepts no arguments'
  exit 0
fi

if [ -z "${DATABENTO_API_KEY:-}" ]; then
  printf '%s\n' 'not run: DATABENTO_API_KEY is not available'
  exit 0
fi

exec cargo test --workspace --all-features --test real_databento -- --ignored --nocapture
