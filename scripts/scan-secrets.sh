#!/bin/sh
set -eu

paths_file=$(mktemp "${TMPDIR:-/tmp}/lwc-databento-paths.XXXXXX")
matches_file=$(mktemp "${TMPDIR:-/tmp}/lwc-databento-secret-paths.XXXXXX")
trap 'rm -f "$paths_file" "$matches_file"' EXIT HUP INT TERM

git ls-files -co --exclude-standard -z -- ':!mise.local.toml' ':!.env' ':!.env.*' > "$paths_file"

if [ ! -s "$paths_file" ]; then
  exit 0
fi

if xargs -0 rg -l --no-messages \
  -e "DATABENTO_API_KEY[[:space:]]*=[[:space:]]*[^[:space:]#\"\\$][^[:space:]#\"]*" \
  -e '-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----' \
  -e '(^|[^A-Za-z0-9])db-[A-Za-z0-9]{20,}' \
  < "$paths_file" > "$matches_file"; then
  printf '%s\n' 'credential-looking value found in:' >&2
  sed 's/^/  /' "$matches_file" >&2
  exit 1
fi
