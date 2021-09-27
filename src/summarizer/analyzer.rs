//! This module provides the [`analyze_path`] function.
//!
//! It reads the contents of a directory, and processes it according to the
//! configuration settings. Its output is used by [`render_groups`] to generate
//! the final columns.
//!
//! [`render_groups`]: super::render::render_groups

use crate::config;
use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};

use super::treereader::TreeReader;
use super::{gitdiff, matchers, sorting};
use super::{Analysis, File, FilesGroup};

/// Read a path and analyze it.
pub(super) fn analyze_path<'a>(
    path: &Path,
    config: &'a config::Root,
) -> Result<Analysis<'a>, io::Error> {
    let mut variables = HashMap::new();

    // Run the collectors to get git and disk usage data.
    let tree_reader = TreeReader::new(config);
    let diff_stats = gitdiff::collect(path, config);

    let mut disk_usage_files = 0;

    // A group contains the column definition and the files for it.
    let mut groups = config
        .columns
        .iter()
        .map(|c| FilesGroup {
            column: c,
            files: Vec::new(),
        })
        .collect::<Vec<_>>();

    for entry in fs::read_dir(path)? {
        let (path, file_name, metadata) = match entry.map(|e| (e.metadata(), e)) {
            Ok((Ok(m), e)) => (e.path(), e.file_name(), m),
            _ => continue,
        };

        if metadata.is_file() {
            disk_usage_files += metadata.len();
        }

        let git_changes = diff_stats.as_ref().and_then(|c| c.get(&file_name));
        let file_name_path = Path::new(&file_name);

        // Find variables to track this entry.
        if let Some(info) = &config.info {
            for (var_name, matchers) in &info.variables {
                if matchers::is_match(file_name_path, &metadata, git_changes, true, matchers) {
                    *variables.entry(&**var_name).or_default() += 1;
                }
            }
        }

        // Find a group for this directory entry.
        for group in &mut groups {
            if matchers::is_match(
                file_name_path,
                &metadata,
                git_changes,
                true,
                &group.column.exclude,
            ) {
                continue;
            }

            if matchers::is_match(
                file_name_path,
                &metadata,
                git_changes,
                group.column.include_hidden,
                &group.column.matchers,
            ) {
                let tree_info = tree_reader.as_ref().and_then(|duc| {
                    if metadata.is_dir() {
                        Some(duc.read_info(&path))
                    } else {
                        None
                    }
                });

                group.files.push(File {
                    file_name,
                    metadata,
                    tree_info,
                    git_changes: git_changes.copied(),
                });

                break;
            }
        }
    }

    // Sort the contents of every column.
    for group in &mut groups {
        sorting::sort(group);
    }

    Ok(Analysis {
        path: path.canonicalize().unwrap_or_else(|_| path.to_owned()),
        groups,
        variables,
        changes: diff_stats.map(|ds| ds.values().sum()),
        disk_usage_files,
    })
}
