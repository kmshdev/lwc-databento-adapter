# ADR: Centralize shared configuration in mise

## Status

Accepted on 2026-08-30.

## Decision

`mise.toml` owns committed tool versions, non-secret loopback defaults, and
repeatable task orchestration. Ignored `mise.local.toml` owns the local
`DATABENTO_API_KEY` only. Code and scripts retain safe defaults for non-secret
configuration so they can run outside an activated mise shell.

## Consequences

Mise must be explicitly trusted before its tasks or environment are used. The
offline quality graph does not require a Databento key. The manual provider
inventory returns `not run` when the key is unavailable, accepts no arguments,
and uses metadata endpoints without requesting market-data records.
