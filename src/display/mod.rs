//! This module provides the [`print()`] function, which takes an instance of
//! [`Screen`], and writes the required text (including ANSI escape codes) to
//! display it.

mod grid;
mod strings;
pub mod styles;

use crate::config::{self, ColorsWhen};
use std::env;
use std::io::{self, Write};
use std::num::NonZeroUsize;

pub use grid::{Column, Row, Screen, Span};
pub use strings::QuotedString;

/// Terminal width when the value can't be read from the TTY.
const DEFAULT_WIDTH: usize = 80;

/// Format the generated columns and write them to `output`.
pub fn print(mut output: impl Write, screen: Screen, config: &config::Root) -> io::Result<()> {
    let width = terminal_width();

    // Header columns.
    if let Some(header) = compute_header(width, screen.info_left, screen.info_right) {
        print_columns(&mut output, width, &header, config)?;
    }

    // Main columns.
    let mut columns = screen.columns;

    if let Some(info_column) = screen.info_column {
        let width = width.map(|w| w.get()).unwrap_or(DEFAULT_WIDTH);
        let columns_width: usize = columns.iter().map(|c| c.width).sum();
        if let Some(padding) = width.checked_sub(columns_width + info_column.width) {
            columns.push(Column::padding(padding, 0, None));
            columns.push(info_column);
        }
    }

    print_columns(&mut output, width, &columns, config)
}

fn compute_header(
    terminal_width: Option<NonZeroUsize>,
    info_left: Option<Column>,
    info_right: Option<Column>,
) -> Option<Vec<Column>> {
    if info_left.is_none() && info_right.is_none() {
        return None;
    }

    let width = terminal_width.map(|w| w.get()).unwrap_or(DEFAULT_WIDTH);

    let padding = match (&info_left, &info_right) {
        (Some(l), Some(r)) => width.checked_sub(l.width + r.width),
        (_, Some(r)) => width.checked_sub(r.width),
        _ => None,
    };

    let padding_column = padding.map(|p| Column::padding(p, 0, None));
    let mut columns = [info_left, padding_column, info_right];
    Some(columns.iter_mut().flat_map(Option::take).collect())
}

fn print_columns(
    mut output: impl Write,
    terminal_width: Option<NonZeroUsize>,
    columns: &[Column],
    config: &config::Root,
) -> io::Result<()> {
    // Discard columns if the total width exceeds the terminal width.
    let max_columns = terminal_width.map(|max_width| {
        columns
            .iter()
            .scan(0, |width, col| {
                *width += col.width;
                Some(*width)
            })
            .take_while(|width| *width <= max_width.get())
            .count()
    });

    // Prepare columns to be printed.

    let mut columns_iter: Vec<_> = columns.iter().map(|col| (col, col.rows.iter())).collect();

    if let Some(max_columns) = max_columns {
        columns_iter.truncate(max_columns);
    }

    let num_rows = columns_iter
        .iter()
        .filter(|c| c.0.has_files)
        .map(|c| c.0.rows.len())
        .max()
        .unwrap_or(0);

    let use_colors = match config.colors.as_ref().and_then(|c| c.when.as_ref()) {
        None | Some(ColorsWhen::Auto) => atty::is(atty::Stream::Stdout),
        Some(ColorsWhen::Never) => false,
        Some(ColorsWhen::Always) => true,
    };

    for num_row in 0..num_rows {
        for (column, rows) in &mut columns_iter {
            macro_rules! padding {
                ($width:expr) => {
                    match $width {
                        0 => (),

                        width => {
                            if use_colors && column.height > num_row {
                                if let Some(style) = column.style {
                                    write!(&mut output, "{}", style.prefix())?;
                                }
                            }

                            write!(&mut output, "{:1$}", " ", width)?;

                            if use_colors && column.height > num_row {
                                if let Some(style) = column.style {
                                    write!(&mut output, "{}", style.suffix())?;
                                }
                            }
                        }
                    }
                };
            }

            match rows.next() {
                None => {
                    padding!(column.width);
                }

                Some(row) => {
                    if column.align == grid::Align::Right {
                        padding!(column.width.saturating_sub(row.width));
                    }

                    for Span { text, style } in &row.spans {
                        let style = styles::combine_opt(column.style, *style);

                        if use_colors {
                            if let Some(style) = style {
                                write!(&mut output, "{}", style.prefix())?;
                            }
                        }

                        write!(&mut output, "{}", text)?;

                        if use_colors {
                            if let Some(style) = style {
                                write!(&mut output, "{}", style.suffix())?;
                            }
                        }
                    }

                    if column.align == grid::Align::Left {
                        padding!(column.width.saturating_sub(row.width));
                    }
                }
            }
        }

        writeln!(&mut output)?;
    }

    let more_columns = columns[columns_iter.len()..]
        .iter()
        .filter(|c| c.has_files)
        .count();

    if more_columns > 0 {
        writeln!(
            &mut output,
            "\n[{} more column{}]",
            more_columns,
            if more_columns == 1 { "" } else { "s" }
        )?;
    }

    Ok(())
}

/// Returns the terminal width.
///
/// 1. It checks the `COLUMNS` environment variable.
/// 2. Then, it queries the TTY for the window size.
/// 3. If none of the above works, returns `None`.
fn terminal_width() -> Option<NonZeroUsize> {
    if let Ok(Ok(w)) = env::var("COLUMNS").map(|c| c.parse()) {
        return NonZeroUsize::new(w);
    }

    terminal_size::terminal_size().and_then(|(w, _)| NonZeroUsize::new(w.0 as usize))
}
