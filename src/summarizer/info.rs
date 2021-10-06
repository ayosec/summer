//! This module provides the [`parse`] function, which takes a formatting string
//! for the info boxes, and returns an iterator over the [tokens] in it. A token
//! can be plain text, or a specifier.
//!
//! [tokens]: Token
//!
//! # Specifiers
//!
//! The formatting string can contain any of the following specifiers:
//!
//! ```notrust
//! %%      literal '%'
//! %P      path
//! %p      path, where '$HOME' is replaced with '~'.
//! %S      disk usage
//! %+      added lines (git)
//! %-      deleted lines (git)
//! %C{…}   color
//! %V{…}   variable
//! ```

use ansi_term::Style;
use std::mem;

/// Tokes extracted from a formatting string.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(super) enum Token<'a> {
    Text(&'a str),
    Variable(&'a str),
    Style(Style),
    StyleReset,
    Path,
    PathHome,
    DiskUsage,
    AddedLines,
    DeletedLines,
}

/// Parse a formatting string, and returns an iterator over the tokens in it.
pub(super) fn parse(format: &str) -> impl Iterator<Item = Token> {
    Parser(format)
}

struct Parser<'a>(&'a str);

impl<'a> Parser<'a> {
    /// Parse the specifier at the beginning of the format string, and updates
    /// the internal state of the parser.
    ///
    /// If the string does not start with a specifier, returns `None`.
    fn parse_specifier(&mut self) -> Option<Token<'a>> {
        let format = self.0.strip_prefix('%')?;

        let (token, skip) = match format.chars().next()? {
            'P' => (Token::Path, 1),
            'p' => (Token::PathHome, 1),
            'S' => (Token::DiskUsage, 1),
            '+' => (Token::AddedLines, 1),
            '-' => (Token::DeletedLines, 1),
            'C' => Self::parse_color(format)?,
            'V' => Self::parse_variable(format)?,
            '%' => (Token::Text("%"), 1),
            _ => return None,
        };

        self.0 = &self.0[skip + 1..];
        Some(token)
    }

    /// Parse `%C{..}` specifiers.
    fn parse_color(format: &str) -> Option<(Token, usize)> {
        let end = memchr::memchr(b'}', format.as_bytes())?;
        let style = match format[..end].strip_prefix("C{")?.trim() {
            "reset" => Token::StyleReset,
            c => Token::Style(colorparse::parse(c).ok()?),
        };

        Some((style, end + 1))
    }

    /// Parse `%V{..}` specifiers.
    fn parse_variable(format: &str) -> Option<(Token, usize)> {
        let end = memchr::memchr(b'}', format.as_bytes())?;
        let var = format[..end].strip_prefix("V{")?;
        Some((Token::Variable(var), end + 1))
    }

    /// Consume all pending input in the formatting string.
    fn consume_pending(&mut self) -> Option<Token<'a>> {
        if self.0.is_empty() {
            None
        } else {
            Some(Token::Text(mem::take(&mut self.0)))
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        if let sp @ Some(_) = self.parse_specifier() {
            return sp;
        }

        // Split the format string at the next '%'.
        let spec_idx = match memchr::memchr(b'%', &self.0.as_bytes()[1..]) {
            Some(idx) => idx + 1,
            None => return self.consume_pending(),
        };

        let (before, after) = self.0.split_at(spec_idx);

        if after == "%" {
            // '%' is at the end of the line.
            return self.consume_pending();
        }

        self.0 = after;
        Some(Token::Text(before))
    }
}

#[test]
fn parse_format_string() {
    use ansi_term::{Colour, Style as AtStyle};
    use Token::*;

    macro_rules! parse {
        ($fmt:expr, $($token:expr),+) => {{
            let mut tokens = parse($fmt);
            $(assert_eq!(tokens.next(), Some($token));)+
            assert_eq!(tokens.next(), None);
        }}
    }

    // A string with all specifiers.
    parse!(
        "%C{blue bold} %P %p : %S%+%-%C{reset}%C{red}%V{dirs} %%dirs",
        Style(AtStyle::new().fg(Colour::Blue).bold()),
        Text(" "),
        Path,
        Text(" "),
        PathHome,
        Text(" : "),
        DiskUsage,
        AddedLines,
        DeletedLines,
        StyleReset,
        Style(AtStyle::new().fg(Colour::Red)),
        Variable("dirs"),
        Text(" "),
        Text("%"),
        Text("dirs")
    );

    // % at the end.
    parse!("aaa%", Text("aaa%"));
    parse!("%P%", Token::Path, Token::Text("%"));

    // No specifiers
    parse!("aaa", Text("aaa"));

    // Multi-byte specifiers.
    parse!("x%α", Token::Text("x"), Token::Text("%α"));
}
