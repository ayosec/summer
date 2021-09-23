#!/bin/bash

set -euo pipefail

: "${ROOT_URL:=https://cgit.freedesktop.org/xdg/shared-mime-info/plain}"

cd "$(dirname "$0")"
wget -q -O COPYING "$ROOT_URL/COPYING" &
wget -q -O MIME.xml "$ROOT_URL/data/freedesktop.org.xml.in" &
wait

gzip -f -n MIME.xml
