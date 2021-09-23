#!/bin/bash
#
# Truncate long names, and escape invalid characters.

set -euo pipefail

cat > config.yaml <<'EOF'
collector:
  disk_usage: false
  git_diff: false

grid:
  max_name_width: 25

columns:
  - matchers: [ { changes: 12 hours } ]
    max_name_width: 3

  - matchers: [ glob: "*Z" ]

  - matchers: [ any ]
EOF

touch a aa aaa aaaa aaaaa
touch β ββ βββ ββββ βββββ
touch βb bβ βbβ ββbβ bββbβ
touch Ｃ ＣＣ ＣＣＣ ＣＣＣＣ

touch -d -1day ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ

touch -d -1day $'Xd\xFA\xFA\xFAd' $'Xd\e\xFAd' $'Xdd\n\xFAdd' $'Xddδδ\xFAd'

$SUMMER -c config.yaml
