#!/usr/bin/env bash
# Installs Ubuntu packages on a CI runner: scripts/ci-apt-install.sh <package>...
#
# The package mirror GitHub's runners use sometimes goes quiet for minutes between packages, and
# a bare `apt-get` waits for it without limit: one install took 30 minutes. Each attempt here is
# cut off and tried again.
set -euo pipefail

readonly ATTEMPTS=3
readonly ATTEMPT_SECONDS=240
# How long a cut-off apt-get gets to stop by itself before it is killed.
readonly KILL_AFTER_SECONDS=10
readonly APT_OPTIONS=(-o Acquire::Retries=3 -o Acquire::http::Timeout=30)

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <package>..." >&2
  exit 2
fi

# Runs one apt-get command, again after a failure, up to ATTEMPTS times.
attempt() {
  local try
  for try in $(seq "$ATTEMPTS"); do
    if sudo timeout --kill-after="$KILL_AFTER_SECONDS" "$ATTEMPT_SECONDS" apt-get "${APT_OPTIONS[@]}" "$@"; then
      return 0
    fi
    echo "apt-get $1 failed or ran past ${ATTEMPT_SECONDS}s (attempt $try of $ATTEMPTS)" >&2
    if [ "$try" -lt "$ATTEMPTS" ]; then
      # A cut-off apt-get can leave dpkg half way through a package. If the repair fails too,
      # the next attempt is what reports it.
      sudo dpkg --configure -a || echo "dpkg --configure -a failed; trying apt-get again" >&2
    fi
  done
  return 1
}

attempt update
attempt install -y "$@"
