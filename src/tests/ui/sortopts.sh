#!/bin/bash
#
# Check for every columns.*.sort key.

set -euo pipefail

runsort() {
  printf "SORT = %s\n" "$*"

  cat > config.yaml <<-EOF
	grid:
	  max_rows: 30

	collector:
	  disk_usage: false
	  git_diff: false

	columns:
	  - matchers: [ glob: "*0" ]
	    sort: "$*"
	  - matchers: [ regex: '^[AB]' ]
	    sort: "$*"
EOF

  $SUMMER -c config.yaml
}

touch -d -1minutes {A,B}{1..20}

touch -d -5day A5
touch -d -4day B2
touch -d -3day B12
touch -d -2day A4

printf ' ' > A15
printf ' ' > B10
printf '  ' > A10
printf '   ' > A20

runsort name
runsort size
runsort mtime
runsort modification_time
runsort version
runsort version desc


#
# Sort by deep_mtime

mkdir deep_mtime
cd deep_mtime

mkdir -p a a/a b/b/b c/c/c/c d

touch -d -5day c/c/c/c/x d/y
touch -d -4day x a/y
touch -d -3day a/a/x b/y
touch -d -2day b/b/b/x
touch -d -1day d/x

printf "SORT = deep_mtime\n"

cat > config.yaml <<EOF
collector:
  disk_usage: true
  git_diff: false

columns:
  - matchers: [ any ]
    sort: deep_mtime
EOF

$SUMMER -c config.yaml
