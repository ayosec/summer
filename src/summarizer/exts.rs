//! Shared functions.

/// Returns the modification time from the file.
pub fn mtime(metadata: &std::fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.mtime() as u64
    }

    #[cfg(not(unix))]
    {
        use std::time::SystemTime;
        metadata
            .modified()
            .ok()
            .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}
