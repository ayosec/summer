# summer

# 0.2 - ?

* Add `mtime` as an alias of the `modification_time` sort key.
* Add `not` matcher.
* Add `deep_mtime` sort key, to sort directories using the newest file in the hierarchy.
* Add `colors.disk_usage` to set the style for the disk usage column.
* Add `%p` to print the path, but replace `$HOME` with `~`.
* Fix issue with the `glob` matcher when the pattern does not starts with `*`.
* Fix default value for `colors.use_lscolors`. Now, it is `true` if `colors` is omitted.
* Fix error messages when the configuration file contains many multibyte characters.

# 0.1 - 2021-09-23

* Configuration is loaded from YAML files.
* Files can be matched by name, file type, MIME type, modification time and git status.
* Matchers can be used to organize files in columns, to compute stats, and to apply styles.
* Display data in columns.
* Collect stats to generate headers and an extra column.
