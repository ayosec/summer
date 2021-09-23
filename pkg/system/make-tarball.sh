#!/bin/bash
#
# This script is part of `pkg/build-release.sh`. It compiles the program with
# and without debug data, and creates the expected packages (zip or tar.gz).

set -xeuo pipefail

source <(bash pkg/system/metadata)

OS="${RUNNER_OS:-linux}"
BINARY_PATH="$PACKAGE_NAME"

if [ "$OS" = Windows ]
then
  BINARY_PATH="$BINARY_PATH.exe"
fi

makepkg() {
  local suffix="$1"
  local file

  # Test the binary.
  ./"$BINARY_PATH" --version
  ./"$BINARY_PATH"

  if [ "$OS" = Windows ]
  then
    file="$ASSETS_PATH/windows--$PACKAGE_NAME-$PACKAGE_VERSION$suffix.zip"
    zip "$file" "$BINARY_PATH"
  else
    file="$ASSETS_PATH/$OS--$PACKAGE_NAME-$PACKAGE_VERSION$suffix.tar.gz"
    tar -czf "$file" "$BINARY_PATH"
  fi
}

RUSTFLAGS="-g" cargo build --release
cd target/release

# With debug.
makepkg "-debug"

# Remove debugging symbols and build another package.
strip "$BINARY_PATH"
makepkg ""
