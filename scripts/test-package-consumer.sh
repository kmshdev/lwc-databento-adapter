#!/bin/sh
set -eu

consumer_root=$(mktemp -d /tmp/lwc-databento-consumer.XXXXXX)
case "$consumer_root" in
  /tmp/lwc-databento-consumer.*) ;;
  *) printf '%s\n' 'unsafe temporary consumer path' >&2; exit 1 ;;
esac
cleanup() {
  case "$consumer_root" in
    /tmp/lwc-databento-consumer.*) rm -rf -- "$consumer_root" ;;
    *) printf '%s\n' 'refusing to clean unsafe temporary consumer path' >&2 ;;
  esac
}
trap cleanup EXIT HUP INT TERM

cp scripts/fixtures/package-consumer/package.json "$consumer_root/package.json"
cp scripts/fixtures/package-consumer/tsconfig.json "$consumer_root/tsconfig.json"
cp scripts/fixtures/package-consumer/main.ts.txt "$consumer_root/main.ts"

pnpm --filter @lwc-databento/adapter pack --pack-destination "$consumer_root" >/dev/null
archive=$(fd -e tgz . "$consumer_root" -d 1 | head -n 1)
if [ -z "$archive" ]; then
  printf '%s\n' 'adapter package archive was not created' >&2
  exit 1
fi

archive_files=$(tar -tf "$archive")
for required_file in package/package.json package/README.md package/dist/index.js package/dist/index.d.ts; do
  if ! printf '%s\n' "$archive_files" | rg -x "$required_file" >/dev/null; then
    printf '%s\n' "adapter archive is missing $required_file" >&2
    exit 1
  fi
done
tar -xOf "$archive" package/package.json | jq -e '
  (has("private") | not) and
  .sideEffects == false and
  .publishConfig.access == "public" and
  .exports["."].types == "./dist/index.d.ts" and
  .exports["."].import == "./dist/index.js" and
  .peerDependencies["lightweight-charts"] == "^5.2.1"
' >/dev/null

(
  cd "$consumer_root"
  pnpm install --offline --ignore-workspace >/dev/null
  pnpm add --offline --ignore-workspace --save-exact "$archive" >/dev/null
  pnpm exec tsc --noEmit
)

printf '%s\n' 'publishable archive and full external Lightweight Charts consumer compiled successfully'
