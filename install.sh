#!/bin/sh
# Install the app on macOS or Linux (any distro, Omarchy / Arch included).
#
#   Public repo:   curl -fsSL https://raw.githubusercontent.com/<REPO>/main/install.sh | sh
#   Private repo:  gh api -H "Accept: application/vnd.github.raw" repos/<REPO>/contents/install.sh | sh
#
# Settings (environment variables, <APP> being the app's command in capitals):
#   <APP>_VERSION      a tag such as v0.1.0 (default: the latest release)
#   <APP>_INSTALL_DIR  where the binary goes (default: ~/.local/bin)
#   <APP>_NO_INTEGRATE set to 1 to leave your agents' config alone (no `<app> integrate`)
set -eu

# The app's command and where it is released: the only place this script names it (ADR 0019).
APP="maitty"
REPO="CydoEntis/maitty"

ENV_PREFIX="$(printf '%s' "$APP" | tr 'a-z-' 'A-Z_')"
eval "VERSION=\${${ENV_PREFIX}_VERSION:-}"
eval "DIR=\${${ENV_PREFIX}_INSTALL_DIR:-\$HOME/.local/bin}"
eval "NO_INTEGRATE=\${${ENV_PREFIX}_NO_INTEGRATE:-}"

say() { printf '%s\n' "$*"; }
fail() { say "$APP install: $*" >&2; exit 1; }

case "$(uname -s)" in
  Linux) os=unknown-linux-musl ;;
  Darwin) os=apple-darwin ;;
  *) fail "this script is for macOS and Linux; on Windows use install.ps1" ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  aarch64 | arm64) arch=aarch64 ;;
  *) fail "no build for $(uname -m) yet; build from source with: cargo install --git https://github.com/$REPO" ;;
esac
target="$arch-$os"
asset="$APP-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# A private repo needs the GitHub CLI (signed in); a public one downloads directly.
fetch() {
  if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    if [ -n "$VERSION" ]; then
      gh release download "$VERSION" -R "$REPO" -p "$1" -D "$tmp" --clobber
    else
      gh release download -R "$REPO" -p "$1" -D "$tmp" --clobber
    fi
  else
    if [ -n "$VERSION" ]; then
      url="https://github.com/$REPO/releases/download/$VERSION/$1"
    else
      url="https://github.com/$REPO/releases/latest/download/$1"
    fi
    curl -fsSL "$url" -o "$tmp/$1" || fail "couldn't download $1 (a private repo needs the GitHub CLI: gh auth login)"
  fi
}

say "Downloading $APP for ${target}..."
fetch "$asset"
fetch sha256sums.txt

expected="$(grep " $asset\$" "$tmp/sha256sums.txt" | cut -d' ' -f1)"
[ -n "$expected" ] || fail "no checksum for $asset"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$asset" | cut -d' ' -f1)"
else
  actual="$(shasum -a 256 "$tmp/$asset" | cut -d' ' -f1)"
fi
[ "$expected" = "$actual" ] || fail "checksum mismatch for $asset"

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$DIR"
# A running copy keeps working: the new file replaces the old one in one step.
cp "$tmp/$APP-$target/$APP" "$DIR/.$APP.new"
chmod 755 "$DIR/.$APP.new"
mv -f "$DIR/.$APP.new" "$DIR/$APP"
if [ "$(uname -s)" = Darwin ]; then
  xattr -d com.apple.quarantine "$DIR/$APP" 2>/dev/null || true
fi
say "Installed $("$DIR/$APP" --version) to $DIR/$APP"

# Status hooks and MCP for every agent installed here; a failure must not fail the install.
say ""
if [ "$NO_INTEGRATE" = 1 ]; then
  say "Left your agents alone (${ENV_PREFIX}_NO_INTEGRATE=1); run '$APP integrate' to connect them."
else
  say "Connecting your agents (set ${ENV_PREFIX}_NO_INTEGRATE=1 to skip)..."
  # Its own stdin: under `curl | sh` the shell's is the rest of this script.
  "$DIR/$APP" integrate </dev/null || say "Warning: some agents weren't connected; run '$APP integrate' to retry."
fi

# Another program of the same name found first on PATH?
found="$(command -v "$APP" 2>/dev/null || true)"
if [ -n "$found" ] && [ "$found" != "$DIR/$APP" ]; then
  say ""
  say "Note: '$found' comes first on your PATH, a different program."
  say "Give this one its own name, e.g. add to your shell's rc file:"
  say "  alias ${APP}2='$DIR/$APP'"
fi

case ":$PATH:" in
  *":$DIR:"*) ;;
  *)
    say ""
    say "$DIR isn't on your PATH yet. Add this to ~/.bashrc, ~/.zshrc or your shell's rc:"
    say "  export PATH=\"$DIR:\$PATH\""
    ;;
esac

say ""
say "Next: run '$APP'. Restart agents that were already running so they load the hooks; '$APP doctor' checks your setup."
say "If $APP was already running, just run $APP: it restarts itself onto the new version (agents resume)."
