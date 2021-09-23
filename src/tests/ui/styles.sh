#!/bin/bash
#
# Truncate long names, and escape invalid characters.

set -euo pipefail

export SUMMER_COLORS='*.yaml=0;35;48;2;255;255;200:*.aa=31:'

cat > config.yaml <<'EOF'
colors:
  when: always
  use_lscolors: SUMMER_COLORS

  column_label: red white
  more_entries: italic blue
  diff_added: white green
  diff_deleted: white red
  name_ellipsis: bold red

  styles:
    - indicator: "X "
      matchers: [ glob: dir0 ]

    - indicator:
        color: blue
        text: 📁
      matchers: [ glob: dir1 ]

    - indicator: ∅
      matchers: [ { glob: '1*' } ]

    - indicator: ✓
      matchers: [ { glob: '*aa' } ]

    - color: "normal #00ffff"
      matchers:
      - glob: '*.aa'
      - glob: '*.bb'
      - regex: '\A[A-Z]+\z'

grid:
  max_rows: 10

columns:
  - matchers: [ { type: directory } ]
    color: white cyan
    label: a bb
  - matchers: [ { regex: '^1' } ]
    color: white green
    label: a aa
  - matchers: [ { regex: '^N' } ]
    sort: version
  - matchers: [ any ]
    max_name_width: 5
EOF

touch 0.aa 1.aa 0.bb 1.xyz ABC abc XYZ ZZZZZZ
touch N{1..20}

mkdir dir0 dir1
fallocate -l100 dir0/x

git init 1>&2
seq 10 > X
git add X
git -c user.email=x -c user.name=x commit -m X 1>&2
seq 5 15 > X

$SUMMER -c config.yaml
