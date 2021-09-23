#!/bin/bash
#
# Test for all valid matchers.

set -euo pipefail

cat > config.yaml <<'EOF'
collector:
  disk_usage: false
  git_diff: false

columns:
  - matchers: [ { type: directory } ]
  - matchers: [ { type: fifo }, { type: socket }, { type: symlink } ]
  - matchers: [ all: [ { type: file }, { type: executable } ] ]
  - matchers: [ { glob: [ "*.a", "y*z" ] }, { regex: '\A[A-Z0-9]+\z' } ]
  - matchers:
    - all:
      - glob: "*.9"
      - regex: "^0"
  - matchers: [ { mime: audio }, { mime: video } ]
  - matchers: [ { mime: image } ]
  - matchers: [ { changes: 12 hours } ]
  - matchers: [ { changes: 30 days } ]
  - matchers: [ { any } ]
EOF

# Directories.
mkdir dir0 dir1

# Executables.
touch a b
chmod +x a b

# Recent changes.
touch -d -10hours c0 c1
touch -d -20days d0 d1
touch -d -40days e0 e1

# FIFO, socket.
mkfifo fifo
perl -mIO::Socket::UNIX -e 'IO::Socket::UNIX->new(Local => "socket")'

# Match file names.
touch 0.a 1.a y000z ABC0Z 00.9

# Mime types.
touch {x,y,z}.{png,ogg,ogv}

$SUMMER -c config.yaml
