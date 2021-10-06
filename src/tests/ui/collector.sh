#!/bin/bash
#
# Collect disk usage from subdirectories, and changes in a git repository.

set -euo pipefail

cat > config.yaml <<'EOF'
columns:
  - matchers: [ type: directory ]

  - matchers: [ any ]
    exclude: [ glob: .git ]
EOF

mkdir -p aaaa/aa bbb ccccc ddd/dd

fallocate -l 500K aaaa/aa/file0
fallocate -l 70K aaaa/file1
fallocate -l 140K ddd/dd/file2

seq 20 > ccccc/file3
seq 30 > file4

git init 1>&2
git add .
git -c user.email=x -c user.name=x commit -m X 1>&2

seq 10 30 > ccccc/file3
seq 20 > file4
git add .

$SUMMER -c config.yaml
