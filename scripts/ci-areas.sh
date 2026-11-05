#!/usr/bin/env bash
# Which CI jobs a change can affect. Reads the changed paths on stdin, one per line, and prints
# `rust=true|false` and `desktop=true|false` for the job conditions in .github/workflows/ci.yml.
# A job is skipped only when every changed path is one it cannot see. No paths at all runs both:
# a list that could not be read must not pass for "nothing changed".
set -euo pipefail

# Documentation builds nothing.
docs='^docs/|\.md$'
# The frontend's own files: no Rust crate compiles or reads them in a check or a test.
frontend='^apps/gui/(src|public)/|^apps/gui/(index\.html|components\.json|eslint\.config\.ts|prettier\.config\.mjs|vite\.config\.ts|tsconfig[^/]*\.json)$'
# What the desktop app may not link (scripts/check-gui-deps.sh), and the terminal app's binary.
terminal='^crates/(maitty-core|maitty-terminal-client|maitty-settings)/|^apps/tui/|^config\.example\.toml$'

paths=$(grep -vE "$docs" || true)
if [ -z "$paths" ]; then
  echo rust=true
  echo desktop=true
  exit 0
fi

rust=false
desktop=false
if grep -qvE "$frontend" <<<"$paths"; then rust=true; fi
if grep -qvE "$terminal" <<<"$paths"; then desktop=true; fi
echo "rust=$rust"
echo "desktop=$desktop"
