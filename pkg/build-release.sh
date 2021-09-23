#!/bin/bash
#
# This script builds packages for a release of the program. It is expected to
# be invoked in a GitHub Actions runner.

set -xeuo pipefail

# Install system dependencies.
bash pkg/system/install-deps.sh

if [ "$(uname -s)" = Linux ]
then
  # Install the Rust compiler.
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y
  source ~/.cargo/env
else
  MAKE_TARBALL=1
fi

# Test the program in this system.
cargo test

ASSETS_PATH="$(git rev-parse --show-toplevel)/ASSETS"
export ASSETS_PATH

mkdir -p "$ASSETS_PATH"

# If MAKE_TARBALL is 1, create two packages (with and without debug info).
#
# Otherwise, build and test a package for the current OS.
if [ "${MAKE_TARBALL:-0}" -eq 1 ]
then

  exec bash pkg/system/make-tarball.sh

elif command -v apt-get
then

  ./pkg/debian/build.sh

  rm -f target/packages/*-dbgsym*deb
  dpkg -i target/packages/*.deb

else

  ./pkg/rpm/build.sh
  rpm -i target/packages/*.rpm

fi

# Test the installed package.
summer --version
summer

source /etc/os-release
for PKG in target/packages/*
do
  mv -vn \
    "$PKG" \
    "ASSETS/$ID-$VERSION_ID--$(basename "$PKG")"
done
