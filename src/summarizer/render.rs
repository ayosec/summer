//! This module provides the [`render_groups`] function. It takes the output of
//! [`analyze_path`] and prepares the columns to be printed by the [`display`]
//! module.
//!
//! [`analyze_path`]: super::analyzer::analyze_path
//! [`display`]: crate::display

// XXX `manual_flatten` is broken in the current version of Clippy.
//      Remove this attribute when the fix from [7566] is released.
//
// [7566]: https://github.com/rust-lang/rust-clippy/pull/7566
#![allow(clippy::manual_flatten)]

use std::path::Path;
use std::{env, mem};

use super::{Analysis, FilesGroup};
use crate::config;
use crate::display::{styles, Column, QuotedString, Row, Screen};

/// Default padding between columns.
const DEFAULT_PADDING: usize = 4;

pub(super) fn render_groups<'a>(analysis: &Analysis<'a>, config: &'a config::Root) -> Screen {
    let padding = config.grid.column_padding.unwrap_or(DEFAULT_PADDING);

    let has_labels = analysis.groups.iter().any(|g| g.column.label.is_some());

    let mut columns = Vec::with_capacity(analysis.groups.len() * 2);
    for group in &analysis.groups {
        if group.files.is_empty() {
            continue;
        }

        if !columns.is_empty() {
            columns.push(Column::padding(padding, 0, None));
        }

        render_group(group, config, has_labels, &mut columns);
    }

    macro_rules! info {
        ($field:ident) => {
            config
                .info
                .as_ref()
                .and_then(|i| i.$field.as_ref())
                .map(|i| render_info(analysis, i))
        };
    }

    Screen {
        columns,
        info_left: info!(left),
        info_right: info!(right),
        info_column: info!(column),
    }
}

fn render_group(
    group: &FilesGroup,
    config: &config::Root,
    has_labels: bool,
    columns: &mut Vec<Column>,
) {
    // Truncate the column height if `max_rows` is set.
    let (files, more_entries) = match (config.grid.max_rows, group.files.len()) {
        (Some(max_rows), len) if max_rows.get() < len && len > 2 => (
            &group.files[..max_rows.get() - 1],
            Some(len - max_rows.get() + 1),
        ),

        _ => (&group.files[..], None),
    };

    // For git changes and disk, create columns only if there are
    // data for them.

    macro_rules! extra_column {
        ($($filter:tt)+) => {
            if files.iter().any($($filter)+) {
                let mut column = Column::new(false);

                if has_labels {
                    column.push(Row::default());
                }

                Some(column)
            } else {
                None
            }
        }
    }

    let mut git_added_column = extra_column!(|file| match &file.git_changes {
        Some(gc) => gc.insertions > 0,
        None => false,
    });

    let mut git_deleted_column = extra_column!(|file| match &file.git_changes {
        Some(gc) => gc.deletions > 0,
        None => false,
    });

    let mut disk_usage_column = extra_column!(|file| file.tree_info.is_some());

    let lscolors = {
        let var_name = match &config.colors.use_lscolors {
            config::LsColors::Bool(false) => None,
            config::LsColors::Bool(true) => Some("LS_COLORS"),
            config::LsColors::VarName(var) => Some(&var[..]),
        };

        var_name
            .and_then(|name| env::var(name).ok())
            .map(|l| lscolors::LsColors::from_string(&l))
    };

    macro_rules! color {
        ($key:ident) => {
            config.colors.$key.as_ref().map(|color| color.style)
        };
    }

    let mut names_column = Column::new(true);
    let mut indicators_column = Column::new(false);
    let mut has_indicators = false;

    if has_labels {
        indicators_column.push(Row::default());

        let mut row = Row::default();
        if let Some(label) = &group.column.label {
            row.add_text(label.to_string(), color!(column_label));
        }

        names_column.push(row);
    }

    for file in files {
        if let Some(column) = git_added_column.as_mut() {
            let mut row = Row::new();
            if let Some(changes) = file.git_changes {
                if changes.insertions > 0 {
                    let style = color!(diff_added)
                        .or_else(|| Some(styles::Style::new().fg(ansi_term::Color::Green)));

                    row.add_text(format!("+{}", changes.insertions), style);
                }
            }
            column.push(row);
        }

        if let Some(column) = git_deleted_column.as_mut() {
            let mut row = Row::new();
            if let Some(changes) = file.git_changes {
                if changes.deletions > 0 {
                    let style = color!(diff_deleted)
                        .or_else(|| Some(styles::Style::new().fg(ansi_term::Color::Red)));

                    row.add_text(format!("-{}", changes.deletions), style);
                }
            }

            column.push(row);
        }

        if let Some(column) = disk_usage_column.as_mut() {
            let mut row = Row::new();
            if let Some(ti) = file.tree_info.as_ref().and_then(|ti| ti.get()) {
                row.add_text(format_size(ti.disk_usage), color!(disk_usage));
            }

            column.push(row);
        }

        let path = Path::new(&file.file_name);
        let mut indicator = Row::new();

        // Apply styles for this file.
        let mut name_style = styles::Style::new();

        if let Some(lscolors) = lscolors.as_ref() {
            if let Some(style) = lscolors.style_for_path_with_metadata(&path, Some(&file.metadata))
            {
                name_style = styles::combine(name_style, style.to_ansi_term_style());
            }
        }

        for style in &config.colors.styles {
            if super::matchers::is_match(
                path,
                &file.metadata,
                file.tree_info.as_ref(),
                file.git_changes.as_ref(),
                true,
                &style.matchers,
            ) {
                if let Some(color) = &style.color {
                    name_style = styles::combine(name_style, color.style);
                }

                if let Some(i) = &style.indicator {
                    has_indicators = true;
                    let (text, style) = i.get();
                    indicator.add_text(text, style);
                }
            }
        }

        let mut row = Row::new();
        let max_name_width = group.column.max_name_width.or(config.grid.max_name_width);
        let name_style = Some(name_style).filter(|s| !s.is_plain());

        let quoted_name = QuotedString::new(path.as_ref(), max_name_width);
        row.add_text(quoted_name.to_string(), name_style);

        if quoted_name.is_truncated() {
            row.add_text("…", color!(name_ellipsis));
        }

        names_column.push(row);
        indicators_column.push(indicator);
    }

    if let Some(more_entries) = more_entries {
        let mut row = Row::new();
        row.add_text(format!("+{} entries", more_entries), color!(more_entries));
        names_column.push(row)
    }

    // Append columns only if they have content.

    let column_style = group.column.color.as_ref().map(|c| c.style);

    for column in [git_added_column, git_deleted_column, disk_usage_column] {
        if let Some(mut column) = column {
            column.align_right();
            column.set_style(column_style);
            column.set_height(names_column.height());
            columns.push(column);
            columns.push(Column::padding(1, names_column.height(), column_style));
        }
    }

    if has_indicators {
        indicators_column.align_right();
        indicators_column.set_style(column_style);
        columns.push(indicators_column);
    }

    names_column.set_style(column_style);
    columns.push(names_column);
}

fn format_size(mut size: u64) -> String {
    if size < 1024 {
        return size.to_string();
    }

    let mut units = "KMGTPEZY".chars();
    while size > 1 << 20 {
        let _ = units.next();
        size >>= 10;
    }

    let size = size as f32 / 1024.0;
    let unit = units.next().unwrap_or('?');
    format!("{:.0}{}", size, unit)
}

fn render_info(analysis: &Analysis, info: &config::InfoContent) -> Column {
    use super::info::{self, Token};

    let (text, base_style) = info.get();
    let mut style = base_style;

    let mut column = Column::new(true);
    let mut row = Row::default();

    column.set_style(base_style);

    for token in info::parse(text) {
        match token {
            Token::Text(mut text) => {
                while let Some(nl) = memchr::memchr(b'\n', text.as_bytes()) {
                    row.add_text(&text[..nl], style);
                    text = &text[nl + 1..];
                    column.push(mem::take(&mut row));
                }

                row.add_text(text, style);
            }

            Token::Variable(var) => {
                row.add_text(
                    format!("{}", analysis.variables.get(&var).unwrap_or(&0)),
                    style,
                );
            }

            Token::Style(next_style) => {
                style = styles::combine_opt(style, Some(next_style));
            }

            Token::StyleReset => {
                style = base_style;
            }

            Token::Path => {
                row.add_text(format!("{}", analysis.path.display()), style);
            }

            Token::PathHome => {
                let mut path = format!("{}", analysis.path.display());
                if let Ok(home_value) = env::var("HOME") {
                    if path.starts_with(&home_value) {
                        path.replace_range(..home_value.len(), "~");
                    }
                }
                row.add_text(path, style);
            }

            Token::DiskUsage => {
                row.add_text(format_size(analysis.disk_usage_files), style);
            }

            Token::AddedLines => {
                if let Some(changes) = &analysis.changes {
                    row.add_text(format!("{}", changes.insertions), style);
                }
            }

            Token::DeletedLines => {
                if let Some(changes) = &analysis.changes {
                    row.add_text(format!("{}", changes.deletions), style);
                }
            }
        }
    }

    if !row.is_empty() {
        column.push(row);
    }

    column
}

#[test]
fn check_size_formats() {
    assert_eq!(format_size(900), "900");
    assert_eq!(format_size(1024), "1K");
    assert_eq!(format_size(1100), "1K");
    assert_eq!(format_size(11111), "11K");
    assert_eq!(format_size((1 << 21) + 100), "2M");
}
