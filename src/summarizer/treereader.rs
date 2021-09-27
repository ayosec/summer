//! This module implements a process to read data from directory trees:
//!
//! * Disk usage.
//! * Newest modification time.
//!
//! The computed size is the length of the files, instead of the actual disk
//! usage (in blocks). This is similar to `du --apparent-size`.
//!
//! The computation is done in a thread pool, and results after a timeout are
//! discarded.
//!
//! In Linux, the collector will not descend directories on other filesystems
//! (like `du -x`).

use std::cell::RefCell;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;
use std::{cmp, fs};

use super::exts::mtime;
use crate::config;
use threadpool::ThreadPool;

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

/// Collector to compute disk usage for a path.
#[derive(Default)]
pub(super) struct TreeReader {
    deadline: Option<Instant>,
    threadpool: ThreadPool,
}

/// Results from the [`TreeReader`]
#[derive(Clone, Copy, Default)]
#[cfg_attr(test, derive(Debug))]
pub(super) struct TreeInfo {
    pub disk_usage: u64,
    pub mtime: u64,
}

/// Results of the disk usage computation.
#[cfg_attr(test, derive(Debug))]
pub(super) struct TreeInfoJob(RefCell<TreeInfoInner>);

#[cfg_attr(test, derive(Debug))]
enum TreeInfoInner {
    Working(Option<Instant>, mpsc::Receiver<Option<TreeInfo>>),
    Done(Option<TreeInfo>),
}

impl TreeReader {
    pub fn new(config: &config::Root) -> Option<TreeReader> {
        if !config.collector.disk_usage {
            return None;
        }

        let deadline = config
            .collector
            .timeout
            .as_ref()
            .map(|t| Instant::now() + t.0);

        let threadpool = threadpool::Builder::new().build();

        Some(TreeReader {
            deadline,
            threadpool,
        })
    }

    /// Read data from the path in a background thread.
    pub fn read_info(&self, path: &Path) -> TreeInfoJob {
        TreeInfoJob::new(&self.threadpool, path, self.deadline)
    }
}

impl TreeInfo {
    fn new(disk_usage: u64, mtime: u64) -> TreeInfo {
        TreeInfo { disk_usage, mtime }
    }
}

impl TreeInfoJob {
    fn new(pool: &ThreadPool, path: &Path, deadline: Option<Instant>) -> TreeInfoJob {
        let (tx, rx) = mpsc::channel();
        let path = path.to_owned();

        pool.execute(move || {
            let metadata = if cfg!(target_os = "linux") {
                path.parent().and_then(|p| p.metadata().ok())
            } else {
                None
            };

            let _ = tx.send(read_path(&path, metadata));
        });

        TreeInfoJob(RefCell::new(TreeInfoInner::Working(deadline, rx)))
    }

    /// Returns the disk usage computed by a background thread.
    ///
    /// If the value is still unavailable, it will wait until `deadline`.
    pub fn get(&self) -> Option<TreeInfo> {
        let mut inner = self.0.borrow_mut();

        let (deadline, rx) = match &mut *inner {
            TreeInfoInner::Done(n) => return *n,
            TreeInfoInner::Working(d, r) => (d, r),
        };

        let timeout = deadline.map(|dl| dl.saturating_duration_since(Instant::now()));
        let res = match timeout {
            Some(t) => rx.recv_timeout(t).ok().flatten(),
            None => rx.recv().ok().flatten(),
        };

        *inner = TreeInfoInner::Done(res);
        res
    }
}

fn read_path(path: &Path, parent_metadata: Option<fs::Metadata>) -> Option<TreeInfo> {
    #[cfg(target_os = "linux")]
    if path.metadata().map(|m| m.st_dev()).ok() != parent_metadata.map(|m| m.st_dev()) {
        // Don't descend in directories in they are
        // in another filesystem.
        return None;
    }

    #[cfg(not(target_os = "linux"))]
    let _ = parent_metadata;

    let dir = match fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => return None,
    };

    dir.filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok().map(|m| (e, m)))
        .map(|(entry, metadata)| {
            if metadata.is_dir() {
                read_path(&entry.path(), Some(metadata))
            } else {
                Some(TreeInfo::new(metadata.len(), mtime(&metadata)))
            }
        })
        .flatten()
        .reduce(|a, b| TreeInfo::new(a.disk_usage + b.disk_usage, cmp::max(a.mtime, b.mtime)))
        .or_else(|| Some(TreeInfo::default()))
}
