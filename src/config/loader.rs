//! This module provides the [`load`] function, which is used to load
//! configuration settings from a YAML file.
//!
//! If the `colors` section contains more files in the `style_files` key, they
//! will be parsed and loaded into the final configuration object.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::{fmt, mem};

use crate::config;

/// Load configuration from a YAML file.
///
/// Files from `colors.style_files` are added to `colors.styles`.
pub fn load(path: impl AsRef<Path>) -> Result<config::Root, LoaderError> {
    let mut root: config::Root = load_file(path.as_ref())?;

    // Load styles from the colors.style_files list.
    //
    // Paths are relative to the parent of the main
    // configuration file.

    let parent = match path.as_ref().parent() {
        Some(p) => p,
        None => return Ok(root),
    };

    for style_file in mem::take(&mut root.colors.style_files) {
        let path = parent.join(style_file);
        let styles: Vec<config::Style> = load_file(&path)?;

        root.colors.styles.reserve(styles.len());
        for style in styles {
            root.colors.styles.push(style);
        }
    }

    Ok(root)
}

fn load_file<T>(path: &Path) -> Result<T, LoaderError>
where
    T: for<'a> serde::Deserialize<'a>,
{
    let file = File::open(path)
        .map(BufReader::new)
        .map_err(|e| LoaderError::Io(path.to_owned(), e))?;

    serde_yaml::from_reader(file).map_err(|e| LoaderError::Parser(path.to_owned(), e))
}

#[derive(Debug)]
pub enum LoaderError {
    Io(PathBuf, io::Error),
    Parser(PathBuf, serde_yaml::Error),
}

impl std::error::Error for LoaderError {}

impl fmt::Display for LoaderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoaderError::Io(path, e) => write!(fmt, "{}: {}", path.display(), e),
            LoaderError::Parser(path, e) => display_yaml_error(fmt, path, e),
        }
    }
}

/// Try to display an error from the YAML parser in a easy-to-read way.
///
/// The format is inspired by error messages from the Rust compiler.
fn display_yaml_error(
    fmt: &mut fmt::Formatter,
    path: &Path,
    error: &serde_yaml::Error,
) -> fmt::Result {
    if let Some(location) = error.location() {
        writeln!(
            fmt,
            "error: cannot parse configuration file.\n\n   --> {}",
            path.display()
        )?;

        if let Ok(Some(line)) = get_line(path, location.line()) {
            let column = location.column() - 1;
            writeln!(fmt, "    |")?;
            writeln!(fmt, "{:3} | {}", location.line(), line)?;

            write!(fmt, "    | {:1$}", "", column)?;
            for _ in 0..line.chars().count() - column {
                fmt.write_str("^")?;
            }

            fmt.write_str("\n")?;
        }

        write!(fmt, "\n{}", error)
    } else {
        write!(fmt, "{}: {}", path.display(), error)
    }
}

/// Reads one line from the file in `path`.
fn get_line(path: &Path, line: usize) -> io::Result<Option<String>> {
    // Ignore non-file paths.
    if !path.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "not a file"));
    }

    BufReader::new(File::open(path)?)
        .lines()
        .nth(line - 1)
        .transpose()
}

#[test]
fn include_style_files() {
    use std::fs;

    let dir = tempdir::TempDir::new("summer").unwrap();

    // Create configuration files.

    let main = dir.path().join("config.yaml");
    let style1 = dir.path().join("style1.yaml");
    let style2 = dir.path().join("style2.yaml");

    fs::write(
        &main,
        b"\
        colors:
            style_files:
            - style1.yaml
            - style2.yaml

            styles:
            - matchers: [ { glob: red } ]
              color: red
            - matchers: [ { glob: green } ]
              color: green
    ",
    )
    .unwrap();

    fs::write(
        &style1,
        b"
        - matchers: [ { glob: blue } ]
          color: blue
    ",
    )
    .unwrap();

    fs::write(
        &style2,
        b"
        - matchers: [ { glob: yellow } ]
          color: yellow
    ",
    )
    .unwrap();

    // Load the files, and check the data.
    let styles = load(&main).unwrap().colors.styles;

    for (n, color) in "red green blue yellow".split_whitespace().enumerate() {
        assert_eq!(
            styles[n].matchers,
            vec![config::Matcher::Glob(
                config::Glob::new(vec![color.into()]).unwrap()
            )]
        );

        assert_eq!(
            styles[n].color,
            Some(config::Color {
                original: color.into(),
                style: colorparse::parse(color).unwrap()
            })
        );
    }

    assert_eq!(styles.len(), 4);
}
