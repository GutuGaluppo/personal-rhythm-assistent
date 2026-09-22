#!/usr/bin/env bash
# Fails if the build for this platform links any HTTP, TLS or socket-client crate.
# The app is local-first and needs no internet (docs/PRIVACY_REVIEW.md, section 4).
set -euo pipefail

manifest="$(dirname "$0")/../src-tauri/Cargo.toml"
banned='^(reqwest|hyper|hyper-util|hyper-tls|hyper-rustls|ureq|isahc|curl|curl-sys|attohttpc|surf|awc|native-tls|rustls|openssl|openssl-sys|h2|tungstenite|tokio-tungstenite|tokio-native-tls|tokio-rustls)$'

found=$(cargo tree --manifest-path "$manifest" -e normal --prefix none 2>/dev/null | sed 's/ v.*//' | sort -u | grep -E "$banned" || true)
if [ -n "$found" ]; then
  echo "network crates in the runtime build:" >&2
  echo "$found" >&2
  exit 1
fi
echo "no network crates in the runtime build"
