//! This module implements the required functionality to read the contents of a
//! directory, and generate the elements required to display the columns defined
//! by a configuration file.
//!
//! The function [`process`] is the only public item of the module.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{fs, io};

mod analyzer;
mod diskusage;
mod gitdiff;
mod info;
mod matchers;
mod render;
mod sorting;

#[cfg(test)]
mod tests;

use crate::config;
use crate::display::Screen;

/// Reads the contents of the directory at `path`, and build the elements
/// required to display the columns defined in `config`.
pub fn process(path: &Path, config: &config::Root) -> Result<Screen, io::Error> {
    let analysis = analyzer::analyze_path(path, config)?;
    Ok(render::render_groups(&analysis, config))
}

/// Data collected by the analyzer.
#[cfg_attr(test, derive(Debug))]
struct Analysis<'a> {
    path: PathBuf,
    groups: Vec<FilesGroup<'a>>,
    variables: HashMap<&'a str, usize>,
    changes: Option<gitdiff::Change>,
    disk_usage_files: u64,
}

#[cfg_attr(test, derive(Debug))]
struct FilesGroup<'a> {
    column: &'a config::Column,
    files: Vec<File>,
}

#[cfg_attr(test, derive(Debug))]
struct File {
    file_name: OsString,
    metadata: fs::Metadata,
    git_changes: Option<gitdiff::Change>,
    disk_usage: Option<diskusage::DiskUsage>,
}
