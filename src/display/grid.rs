//! This module contains elements to build the output of the program.
//!
//! The main type is [`Screen`], which can contain any number of columns, and
//! the definitions for the info boxes.

use ansi_term::Style;
use unicode_width::UnicodeWidthStr;

/// Data to be written to the screen.
pub struct Screen {
    pub columns: Vec<Column>,
    pub info_left: Option<Column>,
    pub info_right: Option<Column>,
    pub info_column: Option<Column>,
}

#[derive(Debug)]
pub struct Column {
    pub(super) align: Align,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) has_files: bool,
    pub(super) style: Option<Style>,
    pub(super) rows: Vec<Row>,
}

#[derive(Default, Debug)]
pub struct Row {
    pub(super) spans: Vec<Span>,
    pub(super) width: usize,
}

#[derive(Debug)]
pub struct Span {
    pub text: Box<str>,
    pub style: Option<Style>,
}

#[derive(Debug, PartialEq)]
pub enum Align {
    Left,
    Right,
}

impl Column {
    pub fn new(has_files: bool) -> Column {
        Column {
            align: Align::Left,
            width: 0,
            height: 0,
            has_files,
            style: None,
            rows: Vec::new(),
        }
    }

    /// Creates a column to contains only spaces.
    pub fn padding(width: usize, height: usize, style: Option<Style>) -> Column {
        Column {
            align: Align::Left,
            width,
            height,
            has_files: false,
            style,
            rows: Vec::new(),
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn align_right(&mut self) {
        self.align = Align::Right;
    }

    pub fn set_style(&mut self, style: Option<Style>) {
        self.style = style;
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
        self.rows.truncate(height);
    }

    pub fn push(&mut self, row: Row) {
        if row.width > self.width {
            self.width = row.width;
        }

        self.height += 1;
        self.rows.push(row);
    }
}

impl Row {
    pub fn new() -> Row {
        Row::default()
    }

    pub fn add_text<S>(&mut self, text: S, style: Option<Style>)
    where
        S: Into<Box<str>>,
    {
        let text = text.into();
        self.width += text.width();
        self.spans.push(Span { text, style })
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

#[test]
fn compute_row_width() {
    let mut row = Row::new();
    row.add_text("α = 0", None);
    row.add_text("(＠)", Some(Style::default().bold()));

    assert_eq!(row.width, 9);
}
