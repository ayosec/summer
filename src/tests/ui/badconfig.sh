#!/bin/bash
#
# Truncate long names, and escape invalid characters.

set -euo pipefail

cat > config.yaml <<'EOF'
collector:
  disk_usage: false
  git_diff: false

columns:
  - matchers: [ { type: dir } ]
  - matchers: [ any ]
EOF

exec 2>&1
if $SUMMER -c config.yaml
then
  echo "Command must not succeed"
  exit 1
fi
