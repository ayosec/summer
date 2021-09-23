#!/bin/bash
#
# This script is part of `pkg/build-release.sh`. It detects the underlying
# operating system, and installs the required dependencies to build the package
# for it.

set -xeuo pipefail

if command -v apt-get
then
  export DEBIAN_FRONTEND=noninteractive

  apt-get update
  apt-get install -y \
    build-essential  \
    curl             \
    debhelper        \
    git

elif command -v yum
then
  yum install -y \
    gcc          \
    gettext      \
    git          \
    rpm-build

elif command -v zypper
then
  zypper install -y \
    curl            \
    gcc             \
    gettext-runtime \
    git             \
    rpm-build

elif command -v choco
then
  choco install zip

elif [ "$(uname -s)" = Darwin ]
then
  :

else
  echo "Unsupported system"
  exit 1
fi
