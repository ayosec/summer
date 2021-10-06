#!/bin/bash
#
# Content for the info boxes.

set -euo pipefail

cat > config.yaml <<'EOF'
colors:
  when: always

collector:
  disk_usage: false
  git_diff: false

info:
  left: "%C{bold ul}%p - %S"

  right:
    color: normal blue
    text: |-
      %C{yellow}%V{dirs} dirs%C{reset}
      %C{red}%V{all} files%C{reset}

  column:
    text: |-
      %C{bold}aaa%C{reset}.
      %C{italic}bbb%C{reset}.
    color: normal green

  variables:
    dirs: [ type: directory ]
    all: [ type: file ]

columns:
  - matchers: [ any ]
EOF

export HOME=$PWD
mkdir data
cd data

touch a b
mkdir e f
fallocate -l 123 c

COLUMNS=80 $SUMMER -c ../config.yaml
