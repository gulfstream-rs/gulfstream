#!/usr/bin/env sh
set -eu

if ! command -v openssl >/dev/null 2>&1; then
  echo 'openssl is required' >&2
  exit 1
fi

secret() {
  openssl rand -hex 32
}

printf '%s\n' \
  "GULFSTREAM_ADMIN_TOKEN='$(secret)'" \
  "GULFSTREAM_PASSWORD_PEPPER='$(secret)'" \
  "GULFSTREAM_SESSION_SIGNING_KEY='$(secret)'" \
  "GULFSTREAM_API_KEY_PEPPER='$(secret)'" \
  "GULFSTREAM_PLAYBACK_SIGNING_KEY='$(secret)'" \
  "GULFSTREAM_VIEWER_HASH_KEY='$(secret)'"
