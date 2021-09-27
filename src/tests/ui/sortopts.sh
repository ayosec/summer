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
