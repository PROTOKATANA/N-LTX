#!/bin/sh
# Releases every package whose version in the code is not published yet: creates its tag and pushes
# all of them at once, and each tag starts its workflow (crates.yml, npm.yml, pypi.yml).
#
#   scripts/release.sh            create and push the missing tags
#   scripts/release.sh --dry-run  only show what would be released
#
# Each release needs its section in CHANGELOG.md (`## <version> - <date>`): the script stops without it,
# and makes the GitHub release of tag v<version> from it once the tags are pushed.
#
# Versions come from the files the workflows check: Cargo.toml (crates voltik and voltik-cli, tag v*),
# npm/package.json (npm-v*) and crates/python/Cargo.toml (py-v*). A version already on its registry,
# or a tag that already exists, is skipped with a note.
set -e
cd "$(dirname "$0")/.."

dry_run=false
case "${1:-}" in
  --dry-run) dry_run=true ;;
  "") ;;
  *) echo "usage: scripts/release.sh [--dry-run]" >&2; exit 2 ;;
esac

crates=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
npm=$(sed -n 's/^  "version": "\(.*\)",/\1/p' npm/package.json | head -1)
python=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/python/Cargo.toml | head -1)

# the tags must point at what is on main, so the published code is the code that passed the pipeline
git fetch --quiet --tags origin main
branch=$(git symbolic-ref --short HEAD)
problems=""
[ "$branch" = main ] || problems="$problems\n  - on branch $branch, not main"
[ -z "$(git status --porcelain --untracked-files=no)" ] || problems="$problems\n  - uncommitted changes"
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || problems="$problems\n  - not in sync with origin/main (git pull --ff-only)"
# what changed, for the GitHub release: written before the release, not after
python3 -I scripts/changelog.py notes "$crates" >/dev/null 2>&1 ||
  problems="$problems\n  - CHANGELOG.md has no section for $crates (## $crates - $(date +%F))"
if [ -n "$problems" ] && ! $dry_run; then
  printf "Cannot release:%b\n" "$problems" >&2
  exit 1
fi

agent="voltik-release (github.com/matheusmota-dev-br/voltik)"
published() {
  case "$1" in
    # both crates must be out: the CLI is published after the core and can lag behind
    crates) curl -sf -A "$agent" -o /dev/null "https://crates.io/api/v1/crates/voltik/$2" &&
      curl -sf -A "$agent" -o /dev/null "https://crates.io/api/v1/crates/voltik-cli/$2" ;;
    npm) curl -sf -o /dev/null "https://registry.npmjs.org/@mathlabs%2Fvoltik/$2" ;;
    python) curl -sf -o /dev/null "https://pypi.org/pypi/voltik/$2/json" ;;
  esac
}

tags=""
check() { # registry name, version, tag
  if git rev-parse --quiet --verify "refs/tags/$3" >/dev/null; then
    if published "$1" "$2"; then
      echo "  skip  $1 $2: already published (tag $3)"
    else
      # a tag that never published: its workflow failed, or it was made before the workflow existed
      echo "  WARN  $1 $2: tag $3 exists at $(git rev-parse --short "$3^{commit}") but $2 is not on the registry;"
      echo "        rerun its workflow, or delete the tag (git push origin :refs/tags/$3 && git tag -d $3) and run again"
    fi
  elif published "$1" "$2"; then
    echo "  skip  $1 $2: already published (bump the version to release again)"
  else
    echo "  new   $1 $2 → tag $3"
    tags="$tags $3"
  fi
}

echo "Packages:"
check crates "$crates" "v$crates"
check npm "$npm" "npm-v$npm"
check python "$python" "py-v$python"

if [ -z "$tags" ]; then
  echo "Nothing to release."
  exit 0
fi
if $dry_run; then
  [ -z "$problems" ] || printf "A real run would stop first:%b\n" "$problems"
  echo "Dry run: would tag $(git rev-parse --short HEAD) and push:$tags"
  exit 0
fi

for t in $tags; do git tag "$t"; done
# shellcheck disable=SC2086 # one word per tag
git push origin $tags
echo "Pushed:$tags. Follow the workflows with: gh run list --limit 5"

# the GitHub release of the crates' tag, with its section of CHANGELOG.md as the notes
case " $tags " in
  *" v$crates "*)
    if python3 -I scripts/changelog.py notes "$crates" | gh release create "v$crates" --title "voltik $crates" --notes-file - --verify-tag; then
      echo "GitHub release v$crates created from CHANGELOG.md."
    else
      echo "Could not create the GitHub release; make it by hand with:"
      echo "  python3 -I scripts/changelog.py notes $crates | gh release create v$crates --title \"voltik $crates\" --notes-file -"
    fi
    ;;
esac
