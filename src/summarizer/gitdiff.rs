//! This module implements a collector to get stats about the changes in the
//! current repository.
//!
//! To get the stats, the collector executes the following command:
//!
//! ```notrust
//! $ git diff --numstat --relative -z HEAD .
//! ```
//!
//! The output, described in [`git-diff(1)`] manual page, is parsed in the
//! `Change::parse` function.
//!
//! A background thread is used to support the timeout set in the configuration.
//!
//! [`git-diff(1)`]: https://git-scm.com/docs/git-diff#_other_diff_formats

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::config;

/// Map to associate file names with their stats.
pub type Changes = HashMap<OsString, Change>;

/// Stats about insertions and deletions for a single path in the repository.
#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Change {
    pub insertions: u32,
    pub deletions: u32,
}

/// Read changes in a Git repository using `git diff`.
pub fn collect(path: &Path, config: &config::Root) -> Option<Changes> {
    if !config.collector.git_diff {
        return None;
    }

    let (tx, rx) = mpsc::channel();

    let child = Command::new("git")
        .args(["diff", "--numstat", "--relative", "-z", "HEAD", "."])
        .current_dir(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    thread::spawn(move || {
        let stdin = match child.wait_with_output() {
            Ok(s) if s.status.success() => s.stdout,

            _ => {
                let _ = tx.send(Changes::new());
                return;
            }
        };

        let changes = Changes::parse(&stdin).unwrap_or_default();
        let _ = tx.send(changes);
    });

    // We have to wait for `git diff` because some matchers may need
    // info about changes in the repository.
    match &config.collector.timeout {
        Some(t) => rx.recv_timeout(t.0).ok(),
        None => rx.recv().ok(),
    }
}

impl Change {
    pub fn new(insertions: u32, deletions: u32) -> Change {
        Change {
            insertions,
            deletions,
        }
    }
}

impl<'a> std::iter::Sum<&'a Change> for Change {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(Change::new(0, 0), |a, b| {
            Change::new(a.insertions + b.insertions, a.deletions + b.deletions)
        })
    }
}

pub trait ChangesParser: Sized {
    /// Parse the output from `git diff --numstat -z`, and returns a map with
    /// insertions and deletions.
    ///
    /// Changes in subdirectories are aggregated in the common parent.
    ///
    /// Returns `None` if the input can't be parsed.
    fn parse(input: &[u8]) -> Option<Self>;
}

impl ChangesParser for Changes {
    /// Parse the output of the `git diff` command.
    fn parse(mut input: &[u8]) -> Option<Changes> {
        let mut changes = HashMap::new();

        macro_rules! until {
            ($delim:expr) => {
                match memchr::memchr($delim, input)? {
                    l => {
                        let (a, b) = input.split_at(l);
                        input = &b[1..];
                        a
                    }
                }
            };
        }

        macro_rules! path {
            () => {{
                let path = until!(b'\0');
                let path = match memchr::memchr(b'/', path) {
                    Some(i) => &path[0..i],
                    None => path,
                };

                #[cfg(unix)]
                {
                    use std::ffi::OsStr;
                    use std::os::unix::ffi::OsStrExt;
                    OsStr::from_bytes(path).to_os_string()
                }

                #[cfg(not(unix))]
                {
                    use std::os::windows::ffi::OsStringExt;
                    let wide: Vec<_> = path.iter().map(|b| *b as u16).collect();
                    OsString::from_wide(&wide)
                }
            }};
        }

        macro_rules! parse_num {
            ($delim:expr) => {
                match std::str::from_utf8(until!($delim)).ok()? {
                    "-" => 0,
                    n => n.parse().ok()?,
                }
            };
        }

        macro_rules! add_change {
            ($path:expr, $insertions:expr, $deletions:expr) => {
                changes
                    .entry($path)
                    .and_modify(|c: &mut Change| {
                        c.insertions += $insertions;
                        c.deletions += $deletions;
                    })
                    .or_insert(Change::new($insertions, $deletions));
            };
        }

        while !input.is_empty() {
            let insertions = parse_num!(b'\t');
            let deletions = parse_num!(b'\t');

            match input {
                [0, tail @ ..] => {
                    // For a rename (`NUL pre NUL post NUL`), increment `deletions`
                    // in the old path, and `insertions` in the new path.
                    input = tail;

                    add_change!(path!(), 0, deletions);
                    add_change!(path!(), insertions, 0);
                }

                _ => {
                    add_change!(path!(), insertions, deletions);
                }
            }
        }

        Some(changes)
    }
}

#[test]
fn parse_git_diff() {
    let input = b"10\t0\tCHANGELOG.md\0\
                  14\t3\tREADME.md\0\
                  10\t1\t\0src/foo.rs\0src/bar.rs\0\
                  5\t7\t\0abc/x\0def/x\0\
                  -\t-\t\0imgs/foo.png\0images/foo.png\0\
                  1\t3\tsrc/main.rs\0";

    let changes = Changes::parse(input).unwrap();

    assert_eq!(changes[&OsString::from("CHANGELOG.md")], Change::new(10, 0));
    assert_eq!(changes[&OsString::from("README.md")], Change::new(14, 3));
    assert_eq!(changes[&OsString::from("src")], Change::new(11, 4));
    assert_eq!(changes[&OsString::from("abc")], Change::new(0, 7));
    assert_eq!(changes[&OsString::from("def")], Change::new(5, 0));
    assert_eq!(changes[&OsString::from("images")], Change::new(0, 0));
    assert_eq!(changes[&OsString::from("imgs")], Change::new(0, 0));
}
