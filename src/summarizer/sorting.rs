//! This module provides the [`sort`] function, which is used to sort the files
//! in a [`FilesGroup`], according to the sort specification defined by the
//! configuration.
//!
//! The implementation to compute the sorting key tries to be as cheap as possible.
//!
//! [`sort`]: self::sort
//! [`FilesGroup`]: super::FilesGroup

use super::exts::mtime;
use crate::config::{SortKey, SortOrder, SortSpec};

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::ops::RangeInclusive;

/// Sort the files in a `FilesGroup`.
pub(super) fn sort(group: &mut super::FilesGroup) {
    let SortSpec(sort_key, sort_order) = group.column.sort.unwrap_or_default();

    let git_changes_first = group.column.git_changes_first;
    let sort_desc = sort_order == SortOrder::Desc;

    macro_rules! git_order {
        ($a:expr, $b:expr) => {
            if git_changes_first {
                let a = u8::from($a.git_changes.is_none());
                let b = u8::from($b.git_changes.is_none());
                match a.cmp(&b) {
                    Ordering::Equal => (),
                    ord if sort_desc => return ord.reverse(),
                    ord => return ord,
                }
            }
        };
    }

    macro_rules! sort {
        ($file:ident => $key:expr) => {
            group.files.sort_unstable_by(|a, b| {
                git_order!(a, b);

                let key_a = {
                    let $file = a;
                    $key
                };

                let key_b = {
                    let $file = b;
                    $key
                };

                key_a.cmp(&key_b)
            })
        };
    }

    match sort_key {
        SortKey::DeepModificationTime => {
            sort!(f => (
                    f.tree_info
                        .as_ref()
                        .and_then(|ti| ti.get())
                        .map(|ti| ti.mtime)
                        .unwrap_or_else(|| mtime(&f.metadata)),
                    &f.file_name
                )
            )
        }

        SortKey::Name => sort!(f => &f.file_name),

        SortKey::Size => {
            sort!(f => (
                f.tree_info
                    .as_ref()
                    .and_then(|ti| ti.get())
                    .map(|ti| ti.disk_usage)
                    .unwrap_or_else(|| f.metadata.len()),
                &f.file_name)
            )
        }

        SortKey::ModificationTime => {
            sort!(f => (mtime(&f.metadata), &f.file_name))
        }

        SortKey::Version => {
            group.files.sort_unstable_by(|a, b| {
                git_order!(a, b);
                compare_versions(&a.file_name, &b.file_name)
            });
        }
    }

    if sort_desc {
        group.files.reverse();
    }
}

/// Compare two version strings.
///
/// Implementation is similar to `strverscmp(3)`.
fn compare_versions(s1: &OsStr, s2: &OsStr) -> Ordering {
    macro_rules! chars {
        ($s:expr) => {{
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;
                $s.as_bytes().into_iter().copied().map(usize::from)
            }

            #[cfg(not(unix))]
            {
                use std::os::windows::ffi::OsStrExt;
                $s.encode_wide().map(usize::from)
            }
        }};
    }

    const ZERO: usize = b'0' as usize;
    const DIGITS: RangeInclusive<usize> = ZERO..=b'9' as usize;

    let mut a = chars!(s1);
    let mut b = chars!(s2);

    let mut number1;
    let mut number2;

    'prefix: loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,

            (None, Some(_)) => return Ordering::Less,

            (Some(_), None) => return Ordering::Greater,

            (Some(a), Some(b)) => {
                if DIGITS.contains(&a) && DIGITS.contains(&b) {
                    number1 = a - ZERO;
                    number2 = b - ZERO;
                    break 'prefix;
                }

                if a != b {
                    return a.cmp(&b);
                }
            }
        }
    }

    for (iter, number) in [(a, &mut number1), (b, &mut number2)] {
        for c in iter {
            if !DIGITS.contains(&c) {
                break;
            }

            // If the multiplication overflows, then ignore the version numbers
            // and return a lexicographical comparison.
            match number.checked_mul(10).and_then(|n| n.checked_add(c - ZERO)) {
                Some(n) => *number = n,
                None => return s1.cmp(s2),
            }
        }
    }

    number1.cmp(&number2)
}

#[test]
fn check_compare_versions() {
    use std::ffi::OsString;

    macro_rules! check {
        ($a:expr, $b:expr, $ord:ident) => {
            assert_eq!(
                compare_versions(&OsString::from($a), &OsString::from($b)),
                Ordering::$ord
            )
        };
    }

    check!("aaa", "bbb", Less);
    check!("aaa100", "aaa2", Greater);
    check!("aaa", "aaa0", Less);
    check!("aaa10", "aaa9", Greater);
    check!("aaa", "aaa", Equal);
    check!("aaa9", "aaaa1", Less);
    check!("aaa100", "aaa10z", Greater);
    check!("aaa10000000000000", "aaa10000000000001", Less);
    check!("aaa90000", "aaa1000000000000000000000", Greater);
}
