# summer

# 0.2 - ?

* Add `mtime` as an alias of the `modification_time` sorting key.
* Add `not` matcher.
* Fix issue with the `glob` matcher when the pattern does not starts with `*`.

# 0.1 - 2021-09-23

* Configuration is loaded from YAML files.
* Files can be matched by name, file type, MIME type, modification time and git status.
* Matchers can be used to organize files in columns, to compute stats, and to apply styles.
* Display data in columns.
* Collect stats to generate headers and an extra column.
