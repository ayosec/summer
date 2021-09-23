//! This module implements the logic for the file matchers.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use super::gitdiff::Change;
use crate::config::{Changes, FileType, Matcher, MimeType};

/// Returns `true` if the file matches any of the items in `matchers`.
///
/// If `include_hidden` is `false`, the file is ignored if it starts with a `.`.
pub(super) fn is_match<'a>(
    path: &Path,
    metadata: &fs::Metadata,
    change: Option<&Change>,
    include_hidden: bool,
    matchers: impl IntoIterator<Item = &'a Matcher>,
) -> bool {
    let mut cached_mime_type = None;

    if !include_hidden {
        if let Some(name) = path.file_name() {
            if is_hidden_file(name, metadata) {
                return false;
            }
        }
    }

    for matcher in matchers {
        match matcher {
            Matcher::Any => {
                return true;
            }

            Matcher::All(matchers) => {
                if matchers
                    .iter()
                    .all(|m| is_match(path, metadata, change, include_hidden, [m]))
                {
                    return true;
                }
            }

            Matcher::Changes(changes) => match changes {
                Changes::Git => {
                    if change.is_some() {
                        return true;
                    }
                }

                Changes::Duration(limit) => {
                    let newer = metadata
                        .modified()
                        .ok()
                        .and_then(|m| SystemTime::now().duration_since(m).ok())
                        .map(|d| d < *limit)
                        .unwrap_or(false);

                    if newer {
                        return true;
                    }
                }
            },

            Matcher::Glob(glob) => {
                if glob.globs.is_match(path) {
                    return true;
                }
            }

            Matcher::Mime(mime_type) => {
                let mt = cached_mime_type
                    .get_or_insert_with(|| path.extension().and_then(MimeType::from_extension));

                if let Some(mt) = mt {
                    if mt == mime_type {
                        return true;
                    }
                }
            }

            Matcher::Regex(regex) => {
                if let Some(n) = path.file_name().and_then(|n| n.to_str()) {
                    if regex.0.is_match(n) {
                        return true;
                    }
                }
            }

            Matcher::Type(file_type) => {
                #[cfg(unix)]
                use std::os::unix::fs::{FileTypeExt, MetadataExt};

                let matched = match file_type {
                    #[cfg(unix)]
                    FileType::BlockDev => metadata.file_type().is_block_device(),

                    #[cfg(unix)]
                    FileType::CharDev => metadata.file_type().is_char_device(),

                    FileType::Directory => metadata.is_dir(),

                    #[cfg(unix)]
                    FileType::Executable => metadata.is_file() && metadata.mode() & 0o111 != 0,

                    FileType::File => metadata.is_file(),

                    #[cfg(unix)]
                    FileType::Fifo => metadata.file_type().is_fifo(),

                    #[cfg(unix)]
                    FileType::Socket => metadata.file_type().is_socket(),

                    FileType::SymLink => metadata.file_type().is_symlink(),
                };

                if matched {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(unix)]
fn is_hidden_file(name: &OsStr, _: &fs::Metadata) -> bool {
    use std::os::unix::ffi::OsStrExt;
    matches!(name.as_bytes(), [b'.', ..])
}

#[cfg(not(unix))]
fn is_hidden_file(name: &OsStr, metadata: &fs::Metadata) -> bool {
    use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};

    const FILE_ATTRIBUTE_HIDDEN: u32 = 2;

    name.encode_wide().next() == Some(b'.' as u16)
        || metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}
