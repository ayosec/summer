//! This module implements a collector to compute the disk usage by a directory
//! tree.
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
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crate::config;
use threadpool::ThreadPool;

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

/// Collector to compute disk usage for a path.
#[derive(Default)]
pub(super) struct DiskUsageCollector {
    deadline: Option<Instant>,
    threadpool: ThreadPool,
}

/// Results of the disk usage computation.
#[cfg_attr(test, derive(Debug))]
pub struct DiskUsage(RefCell<DiskUsageInner>);

#[cfg_attr(test, derive(Debug))]
enum DiskUsageInner {
    Working(Option<Instant>, mpsc::Receiver<Option<u64>>),
    Done(Option<u64>),
}

impl DiskUsageCollector {
    pub fn new(config: &config::Root) -> Option<DiskUsageCollector> {
        if !config.collector.disk_usage {
            return None;
        }

        let deadline = config
            .collector
            .timeout
            .as_ref()
            .map(|t| Instant::now() + t.0);

        let threadpool = threadpool::Builder::new().build();

        Some(DiskUsageCollector {
            deadline,
            threadpool,
        })
    }

    /// Compute the disk used by a directory in a background thread.
    pub fn disk_usage(&self, path: &Path) -> DiskUsage {
        DiskUsage::new(&self.threadpool, path, self.deadline)
    }
}

impl DiskUsage {
    fn new(pool: &ThreadPool, path: &Path, deadline: Option<Instant>) -> DiskUsage {
        let (tx, rx) = mpsc::channel();
        let path = path.to_owned();

        pool.execute(move || {
            let metadata = if cfg!(target_os = "linux") {
                path.parent().and_then(|p| p.metadata().ok())
            } else {
                None
            };

            let _ = tx.send(compute_disk_usage(&path, metadata));
        });

        DiskUsage(RefCell::new(DiskUsageInner::Working(deadline, rx)))
    }

    /// Returns the disk usage computed by a background thread.
    ///
    /// If the value is still unavailable, it will wait until `deadline`.
    pub fn get(&self) -> Option<u64> {
        let mut inner = self.0.borrow_mut();

        let (deadline, rx) = match &mut *inner {
            DiskUsageInner::Done(n) => return *n,
            DiskUsageInner::Working(d, r) => (d, r),
        };

        let timeout = deadline.map(|dl| dl.saturating_duration_since(Instant::now()));
        let res = match timeout {
            Some(t) => rx.recv_timeout(t).ok().flatten(),
            None => rx.recv().ok().flatten(),
        };

        *inner = DiskUsageInner::Done(res);
        res
    }
}

fn compute_disk_usage(path: &Path, metadata: Option<fs::Metadata>) -> Option<u64> {
    #[cfg(target_os = "linux")]
    if path.metadata().map(|m| m.st_dev()).ok() != metadata.map(|m| m.st_dev()) {
        // Don't descend in directories in they are
        // in another filesystem.
        return None;
    }

    #[cfg(not(target_os = "linux"))]
    let _ = metadata;

    let dir = match fs::read_dir(path) {
        Ok(d) => d,
        Err(_) => return None,
    };

    let bytes = dir
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok().map(|m| (e, m)))
        .map(|(entry, metadata)| {
            if metadata.is_dir() {
                compute_disk_usage(&entry.path(), Some(metadata))
            } else {
                Some(metadata.len())
            }
        })
        .flatten()
        .sum();

    Some(bytes)
}
