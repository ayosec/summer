#!/bin/bash
#
# This script generates a RPM package with the binary built from
# cargo build --release.

set -euo pipefail

cd "$(dirname "$0")"

# shellcheck source=/dev/null
source <(../system/metadata)

DEST=$(mktemp -d)

envsubst < summer.spec > "$DEST/summer.spec"

cd "$(git rev-parse --show-toplevel)"
rpmbuild -bb --build-in-place             \
  --define "_rpmdir $PWD/target/packages" \
  "$DEST/summer.spec"

cd target/packages
mv */*.rpm .
