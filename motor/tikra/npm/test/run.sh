#!/bin/sh
# Packs npm/pkg the way npm publish would, installs the tarball in a scratch project and runs the
# tests against it, so exports, files and both module systems are checked as users get them.
set -e
cd "$(dirname "$0")/.."
tgz=$(cd pkg && npm pack --silent --pack-destination ..)
project=$(mktemp -d)
cp test/*.test.* "$project/"
(cd "$project" && npm init -y >/dev/null && npm install --silent "$OLDPWD/$tgz" && node --test)
rm -rf "$project" "$tgz"
