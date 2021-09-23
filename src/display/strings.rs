//! This module implements the [`QuotedString`] type, which can be used to print
//! paths that may contain non-printable characters.
//!
//! The final string can be limited to a maximum width.

use std::cell::Cell;
use std::ffi::OsStr;
use std::fmt;
use std::num::NonZeroUsize;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use unicode_width::UnicodeWidthChar;

/// Contains a [`OsStr`] that can be formatted as a Unicode string.
///
/// Optionally, the generated string can have a maximum width. If the actual
/// width exceeds this limit, `is_truncated()` returns `true` *after* invoking
/// the `Display::fmt` function.
pub struct QuotedString<'a> {
    string: &'a OsStr,
    max_width: Option<NonZeroUsize>,
    truncated: Cell<bool>,
}

impl QuotedString<'_> {
    pub fn new(string: &OsStr, max_width: Option<NonZeroUsize>) -> QuotedString {
        QuotedString {
            string,
            max_width,
            truncated: Cell::new(false),
        }
    }

    /// Returns `true` is the string was truncated after calling the
    /// `Display::fmt` function.
    ///
    /// # Example
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    /// let qs = QuotedString::new(OsStr::new("abcd"), NonZeroUsize::new(3));
    ///
    /// let mut s = qs.to_string();
    /// if qs.is_truncated() {
    ///     s.push('…');
    /// }
    /// ```
    pub fn is_truncated(&self) -> bool {
        self.truncated.get()
    }

    /// Implementation for ASCII-only strings with no control characters.
    #[cfg(unix)]
    #[inline(always)]
    fn try_write_unquoted(&self, fmt: &mut fmt::Formatter) -> Result<bool, fmt::Error> {
        let bytes = self.string.as_bytes();

        if !bytes.iter().all(|b| (20..128).contains(b)) {
            return Ok(false);
        }

        // SAFETY: it is safe to use `from_utf8_unchecked` because we know that
        //         `bytes` only contains ASCII characters.

        match self.max_width {
            Some(max_width) if max_width.get() < bytes.len() => {
                let s = unsafe { std::str::from_utf8_unchecked(&bytes[..max_width.get() - 1]) };
                fmt.write_str(s)?;
                self.truncated.set(true);
            }

            _ => {
                fmt.write_str(unsafe { std::str::from_utf8_unchecked(bytes) })?;
            }
        }

        Ok(true)
    }
}

impl fmt::Display for QuotedString<'_> {
    #[cfg(unix)]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if self.try_write_unquoted(fmt)? {
            return Ok(());
        }

        let mut bytes = self.string.as_bytes();
        let mut width = self.max_width;

        macro_rules! next_char {
            ($width:expr) => {{
                if let Some(w) = width.as_mut() {
                    let char_width = $width;
                    if w.get() <= char_width {
                        self.truncated.set(true);
                        return Ok(());
                    }

                    width = NonZeroUsize::new(w.get() - char_width);
                }
            }};
        }

        macro_rules! write_str {
            ($str:expr) => {
                for chr in $str.chars() {
                    if chr < ' ' {
                        next_char!(4);
                        write!(fmt, "\\x{:02X}", chr as u32)?;
                    } else {
                        next_char!(chr.width().unwrap_or(0));
                        write!(fmt, "{}", chr)?;
                    }
                }
            };
        }

        loop {
            match std::str::from_utf8(bytes) {
                Ok(s) => {
                    write_str!(s);
                    return Ok(());
                }

                Err(e) => {
                    let (valid, after_valid) = bytes.split_at(e.valid_up_to());

                    unsafe {
                        write_str!(std::str::from_utf8_unchecked(valid));
                    }

                    let invalid = match e.error_len() {
                        Some(len) => &after_valid[..len],
                        None => after_valid,
                    };

                    for byte in invalid {
                        next_char!(4);
                        write!(fmt, "\\x{:02X}", *byte)?;
                    }

                    bytes = &after_valid[invalid.len()..];
                }
            }
        }
    }

    #[cfg(not(unix))]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use std::path::Path;

        let display = Path::new(self.string).display();

        match self.max_width {
            Some(max_width) => {
                let mut width = 0;
                for chr in display.to_string().chars() {
                    width += chr.width().unwrap_or(1);

                    if width >= max_width.get() {
                        self.truncated.set(true);
                        return Ok(());
                    }

                    write!(fmt, "{}", chr)?;
                }

                Ok(())
            }

            None => write!(fmt, "{}", display),
        }
    }
}

#[cfg(unix)]
#[test]
#[allow(clippy::bool_assert_comparison)]
fn quote_strings() {
    use std::os::unix::ffi::OsStrExt;

    macro_rules! check {
        ($string:expr, $width:expr, $expected:expr) => {
            check!($string, $width, $expected, false)
        };

        ($string:expr, $width:expr, $expected:expr, $truncated:expr) => {
            let qs = QuotedString::new(OsStr::from_bytes(&$string[..]), NonZeroUsize::new($width));
            assert_eq!(format!("{}", qs), $expected);
            assert_eq!(qs.is_truncated(), $truncated);
        };
    }

    check!(b"\xEF\xBC\xA1 \xEF\xBC\xA2", 0, "Ａ Ｂ");
    check!(b"\xCE\xB1 \xEF\xBC", 0, "α \\xEF\\xBC");
    check!(b"\xCE\xB1 \xEF\xBC \xCE\xB1", 0, "α \\xEF\\xBC α");
    check!(b"a\n\0\x11b", 0, "a\\x0A\\x00\\x11b");

    check!(b"aaa", 3, "aaa");
    check!(b"bbbbb", 3, "bb", true);
    check!(b"\xCE\xB1 \xEF\xBC", 3, "α ", true);
}
