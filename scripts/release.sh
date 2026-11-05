#!/usr/bin/env bash
# One release: scripts/release.sh <X.Y.Z> [--dry-run] [--skip-ci-check]
#
# Bumps the version on dev, brings main up to dev, tags vX.Y.Z on main and pushes the three
# together. The tag starts .github/workflows/release.yml, which builds and publishes.
#
# Running this is the explicit, one-time authorization docs/COMMIT-STANDARDS.md asks for before
# a direct commit to a protected branch, for this one release.
#
# It works in a temporary detached worktree and reads the remote without moving any local ref,
# so the caller's checkout, index and branches are untouched: dev and main are often checked out
# in other worktrees, where `git switch` would refuse.
#
# --dry-run        runs every check, then prints the commands instead of running them.
# --skip-ci-check  releases without a green ci run on dev's head. GitHub cancelling a job for
#                  lack of a runner marks the whole run failed with nothing wrong.
set -euo pipefail

readonly REMOTE=origin
readonly TUI_MANIFEST=apps/tui/Cargo.toml
readonly GUI_CONFIG=apps/gui/src-tauri/tauri.conf.json
# What ci.yml's `paths-ignore` leaves out: a push of nothing else starts no ci run.
readonly NO_CI_PATHS='^docs/|\.md$'
readonly RELEASE_RUN_POLLS=10
readonly RELEASE_RUN_POLL_SECONDS=3

die() {
  echo "release: $*" >&2
  exit 1
}

remote_sha() {
  git ls-remote "$REMOTE" "$1" | cut -f1
}

# True when version $1 is greater than version $2, both X.Y.Z.
is_greater() {
  local IFS=.
  local -a new=($1) old=($2)
  local i
  for i in 0 1 2; do
    if ((10#${new[i]} > 10#${old[i]})); then return 0; fi
    if ((10#${new[i]} < 10#${old[i]})); then return 1; fi
  done
  return 1
}

# True when every file that differs between commit $1 and dev's head is one ci never runs for.
only_ci_ignored_paths_since() {
  local changed
  git merge-base --is-ancestor "$1" "$dev" || return 1
  # Without renames a source file moved into docs/ is listed under its old path as well.
  changed=$(git diff --name-only --no-renames "$1" "$dev") || return 1
  [ -z "$changed" ] || ! grep -qvE "$NO_CI_PATHS" <<<"$changed"
}

check_ci() {
  local run sha status conclusion url
  run=$(gh run list --workflow ci --branch dev --event push --limit 1 \
    --json headSha,status,conclusion,url \
    --jq '.[0] // empty | [.headSha, .status, .conclusion, .url] | join("|")')
  [ -n "$run" ] || die "no ci run was found for a push to dev"
  IFS='|' read -r sha status conclusion url <<<"$run"
  [ "$status" = completed ] || die "the newest ci run on dev is not finished ($status): $url"
  [ "$conclusion" = success ] || die "the newest ci run on dev ended in $conclusion: $url"
  [ "$sha" = "$dev" ] || only_ci_ignored_paths_since "$sha" \
    || die "dev's head ${dev:0:8} has no ci run; the newest is for ${sha:0:8}: $url"
}

# Puts the new version in the first line of a file that matches a pattern, and changes nothing
# else in it.
set_version() {
  local file=$1 pattern=$2 edited
  edited=$(mktemp)
  awk -v pattern="$pattern" -v version="$version" '
    !done && $0 ~ pattern && sub(/"[0-9]+\.[0-9]+\.[0-9]+"/, "\"" version "\"") { done = 1 }
    { print }
    END { exit !done }
  ' "$file" >"$edited" || die "found no version to set in $file"
  cat "$edited" >"$file"
  rm "$edited"
}

# Prints a command, and runs it unless this is a dry run.
run() {
  local arg line='+'
  for arg in "$@"; do
    if [[ "$arg" =~ ^[A-Za-z0-9_./:=@-]+$ ]]; then line+=" $arg"; else line+=" '$arg'"; fi
  done
  echo "$line"
  if ! $dry_run; then "$@"; fi
}

# The commit the temporary worktree is on. A dry run has made none, so it gets a name to print.
worktree_head() {
  if $dry_run; then
    echo "$1"
  else
    git -C "$worktree" rev-parse HEAD
  fi
}

# Runs however the script ends, an interrupt included. A tag made here that did not reach the
# remote would make the next run refuse.
clean_up() {
  if [ -n "$tag_to_drop" ]; then git tag -d "$tag_to_drop"; fi
  git worktree remove --force "$worktree" 2>/dev/null || rm -rf "$worktree"
}

print_release_run() {
  local url='' try
  echo "release: $tag is pushed"
  for try in $(seq "$RELEASE_RUN_POLLS"); do
    # The release is out by now. A lookup that fails must not end the script as a failure.
    url=$(gh run list --workflow release --event push --limit 5 --json headBranch,url \
      --jq "map(select(.headBranch == \"$tag\"))[0].url // empty") || url=''
    if [ -n "$url" ]; then break; fi
    sleep "$RELEASE_RUN_POLL_SECONDS"
  done
  echo "release: its release run: ${url:-not listed yet, see the release workflow on GitHub}"
}

version=''
dry_run=false
skip_ci_check=false
for arg in "$@"; do
  case "$arg" in
    --dry-run) dry_run=true ;;
    --skip-ci-check) skip_ci_check=true ;;
    -*) die "unknown option $arg" ;;
    *)
      [ -z "$version" ] || die "give one version, got '$version' and '$arg'"
      version=$arg
      ;;
  esac
done
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  || die "usage: scripts/release.sh <X.Y.Z> [--dry-run] [--skip-ci-check]; the version must be X.Y.Z, got '$version'"
readonly tag="v$version"

cd "$(git rev-parse --show-toplevel)"

dev=$(remote_sha refs/heads/dev)
main=$(remote_sha refs/heads/main)
[ -n "$dev" ] && [ -n "$main" ] || die "could not read dev and main from $REMOTE"
# An empty refmap fetches the commits without moving this checkout's origin/dev or origin/main.
git fetch --quiet --no-tags --refmap= "$REMOTE" refs/heads/dev refs/heads/main
git cat-file -e "$dev^{commit}" && git cat-file -e "$main^{commit}" \
  || die "dev or main moved on $REMOTE while they were being read; run it again"

current=$(git show "$dev:$TUI_MANIFEST" | sed -n '/^version = "/{s/^version = "\(.*\)".*/\1/p;q;}')
is_greater "$version" "$current" || die "$version is not greater than $current, the version on dev"

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  die "the tag $tag already exists in this repository"
fi
remote_tag=$(remote_sha "refs/tags/$tag") || die "could not read the tags on $REMOTE"
[ -z "$remote_tag" ] || die "the tag $tag already exists on $REMOTE"

if $skip_ci_check; then
  echo "release: --skip-ci-check: not checking that ci passed on dev's head ${dev:0:8}"
else
  check_ci
fi

echo "release: $current -> $version, dev ${dev:0:8}, main ${main:0:8}"
if $dry_run; then
  worktree='<temporary worktree>'
else
  worktree=$(mktemp -d)
  tag_to_drop=''
  trap clean_up EXIT
fi

run git worktree add --quiet --detach "$worktree" "$dev"
run set_version "$worktree/$TUI_MANIFEST" '^version = "'
run set_version "$worktree/$GUI_CONFIG" '^  "version": "'
run cargo update --quiet --workspace --offline --manifest-path "$worktree/Cargo.toml"
run git -C "$worktree" add "$TUI_MANIFEST" "$GUI_CONFIG" Cargo.lock
run git -C "$worktree" commit --quiet -m "build: bump to $tag"
bump=$(worktree_head '<bump commit>')

run git -C "$worktree" checkout --quiet --detach "$main"
run git -C "$worktree" merge --quiet --no-ff -m "Merge dev into main for $tag" "$bump"
merge=$(worktree_head '<merge commit>')
run git tag "$tag" "$merge"
if ! $dry_run; then tag_to_drop=$tag; fi

if ! run git push --atomic "$REMOTE" "$bump:refs/heads/dev" "$merge:refs/heads/main" "refs/tags/$tag"; then
  # A push also fails when the connection drops after the remote has applied it.
  if ! remote_tag=$(remote_sha "refs/tags/$tag"); then
    tag_to_drop=''
    die "the push failed and $REMOTE cannot be read to see whether $tag arrived. The local tag is kept: look before running this again"
  fi
  [ "$remote_tag" = "$merge" ] || die "the push was refused, so nothing was released"
  echo "release: git push reported a failure, but $tag is on $REMOTE" >&2
fi
tag_to_drop=''

if $dry_run; then
  echo "release: dry run: nothing was committed, tagged or pushed"
else
  print_release_run
fi
