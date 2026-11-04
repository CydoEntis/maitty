#!/usr/bin/env bash
# The desktop app's dependency rule (ADR 0042, 0048): the Tauri host must not link the daemon
# crate, the terminal client or the settings crate, on any platform or in any build kind.
# Run from the repo root; fails with the offending lines.
set -euo pipefail

forbidden='^(maitty-core|maitty-terminal-client|maitty-settings) '
found=$(cargo tree -p maitty-gui --target all --prefix none | sort -u | grep -E "$forbidden" || true)
if [ -n "$found" ]; then
  echo "maitty-gui must not depend on these crates (ADR 0042, 0048):" >&2
  echo "$found" >&2
  exit 1
fi
echo "maitty-gui depends on none of maitty-core, maitty-terminal-client, maitty-settings"
