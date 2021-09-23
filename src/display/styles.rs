//! Extra functions to manage styles.

pub use ansi_term::Style;

/// Combine two styles.
pub fn combine(old: Style, new: Style) -> Style {
    Style {
        foreground: new.foreground.or(old.foreground),
        background: new.background.or(old.background),
        is_bold: new.is_bold || old.is_bold,
        is_dimmed: new.is_dimmed || old.is_dimmed,
        is_italic: new.is_italic || old.is_italic,
        is_underline: new.is_underline || old.is_underline,
        is_blink: new.is_blink || old.is_blink,
        is_reverse: new.is_reverse || old.is_reverse,
        is_hidden: new.is_hidden || old.is_hidden,
        is_strikethrough: new.is_strikethrough || old.is_strikethrough,
    }
}

/// Like [`combine`], but for `Option` values.
pub fn combine_opt(old: Option<Style>, new: Option<Style>) -> Option<Style> {
    match (old, new) {
        (Some(a), Some(b)) => Some(combine(a, b)),
        (a, None) => a,
        (None, b) => b,
    }
}
