#!/usr/bin/env bash
# The desktop app's dependency rule (ADR 0042, 0048): the Tauri host must not link the daemon
# crate, the terminal client or the settings crate, on any platform or in any build kind.
# Run from the repo root; fails with the offending lines.
set -euo pipefail

forbidden='^(maitty-core|maitty-terminal-client|maitty-settings) '
# A failing `cargo tree` stops the script here (set -e) and an empty tree is not a pass: a check that
# read nothing would find nothing.
tree=$(cargo tree -p maitty-gui --target all --prefix none)
if ! grep -q '^maitty-gui ' <<<"$tree"; then
  echo "cargo tree did not list maitty-gui, so nothing was checked" >&2
  exit 1
fi
found=$(sort -u <<<"$tree" | grep -E "$forbidden" || true)
if [ -n "$found" ]; then
  echo "maitty-gui must not depend on these crates (ADR 0042, 0048):" >&2
  echo "$found" >&2
  exit 1
fi
echo "maitty-gui depends on none of maitty-core, maitty-terminal-client, maitty-settings"
