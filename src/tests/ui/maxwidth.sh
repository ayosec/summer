#!/bin/bash
#
# Dicard columns if they exceeds the maximum width.

set -euo pipefail

cat > config.yaml <<'EOF'
collector:
  disk_usage: false
  git_diff: false

columns:
  - matchers: [ regex: "1" ]
  - matchers: [ regex: "2" ]
  - matchers: [ regex: "3" ]
  - matchers: [ regex: "4" ]
  - matchers: [ regex: "5" ]
EOF

for N in {1..5}
do
  : > "$(printf "%-10s" $N)"
done

export COLUMNS=45
$SUMMER -c config.yaml
