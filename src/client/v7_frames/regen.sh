#!/bin/sh
# Regenerate every v7 reference frame from the design source, with the app's name from
# Cargo.toml (ADR 0019). Run from the repo root after changing the design or renaming the app:
#   sh src/client/v7_frames/regen.sh
set -eu
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
cp docs/design/v7/*.js "$T"
echo '{"type":"module"}' > "$T/package.json"
node docs/design/v7/render-frames.mjs "$T" "$T/frames.txt"
node src/client/v7_frames/split-frames.mjs "$T/frames.txt"
node src/client/v7_frames/cut-refs.mjs "$T/frames.txt" src/client/screen/v7_ref.txt src/client/screen/check_modals_frames.txt "$T/v7_ref.txt" "$T/modals.txt"
cp "$T/v7_ref.txt" src/client/screen/v7_ref.txt
cp "$T/modals.txt" src/client/screen/check_modals_frames.txt
echo "frames regenerated"
