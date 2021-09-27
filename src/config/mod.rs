//! This module contains the type definitions to load configuration from YAML
//! files.

mod loader;
mod mimetypes;
mod serde_impls;

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use loader::{load, LoaderError};
pub use mimetypes::MimeType;

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Root {
    pub colors: Option<Colors>,

    #[serde(default)]
    pub grid: Grid,

    #[serde(default)]
    pub columns: Vec<Column>,

    pub info: Option<Info>,

    #[serde(default)]
    pub collector: Collector,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Colors {
    pub when: Option<ColorsWhen>,

    #[serde(default)]
    pub use_lscolors: LsColors,

    pub column_label: Option<Color>,

    pub name_ellipsis: Option<Color>,

    pub more_entries: Option<Color>,

    pub diff_added: Option<Color>,

    pub diff_deleted: Option<Color>,

    #[serde(default)]
    pub styles: Vec<Style>,

    #[serde(default)]
    pub style_files: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ColorsWhen {
    Auto,
    Always,
    Never,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(untagged)]
pub enum LsColors {
    Bool(bool),
    VarName(String),
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub matchers: Vec<Matcher>,
    pub color: Option<Color>,
    pub indicator: Option<Indicator>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields, untagged)]
pub enum Indicator {
    Plain(String),
    Attrs { text: String, color: Option<Color> },
}

impl Indicator {
    pub fn get(&self) -> (&str, Option<ansi_term::Style>) {
        match self {
            Indicator::Plain(s) => (s, None),
            Indicator::Attrs { text, color } => (text, color.as_ref().map(|c| c.style)),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Grid {
    pub max_rows: Option<NonZeroUsize>,

    pub max_name_width: Option<NonZeroUsize>,

    pub column_padding: Option<usize>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Column {
    #[serde(default)]
    pub include_hidden: bool,

    pub label: Option<String>,

    pub max_name_width: Option<NonZeroUsize>,

    pub matchers: Vec<Matcher>,

    #[serde(default)]
    pub exclude: Vec<Matcher>,

    #[serde(default = "default_true")]
    pub git_changes_first: bool,

    pub color: Option<Color>,

    pub sort: Option<SortSpec>,
}

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct SortSpec(pub SortKey, pub SortOrder);

#[derive(Serialize, Deserialize, Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    #[serde(alias = "deep_mtime")]
    DeepModificationTime,

    Name,

    Size,

    #[serde(alias = "mtime")]
    ModificationTime,

    Version,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
#[cfg_attr(test, derive(Debug))]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(rename_all = "snake_case")]
pub enum Matcher {
    Any,
    All(Vec<Matcher>),
    Changes(Changes),
    Glob(Glob),
    Mime(MimeType),
    Not(Box<Matcher>),
    Regex(Regex),
    Type(FileType),
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Changes {
    Git,
    Duration(Duration),
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Color {
    pub original: String,
    pub style: ansi_term::Style,
}

#[cfg_attr(test, derive(Debug))]
pub struct Glob {
    pub original: Vec<String>,
    pub globs: globset::GlobSet,
}

#[cfg(test)]
impl PartialEq for Glob {
    fn eq(&self, other: &Self) -> bool {
        self.original.eq(&other.original)
    }
}

impl Glob {
    pub fn new(globs: Vec<String>) -> Result<Self, globset::Error> {
        let mut set = globset::GlobSetBuilder::new();
        for glob in &globs {
            set.add(globset::Glob::new(glob)?);
        }

        Ok(Glob {
            original: globs,
            globs: set.build()?,
        })
    }
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    BlockDev,
    CharDev,
    Directory,
    Executable,
    File,
    Fifo,
    Socket,
    SymLink,
}

#[cfg(not(unix))]
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Directory,
    File,
    SymLink,
}

#[cfg_attr(test, derive(Debug))]
pub struct Regex(pub regex::Regex);

#[cfg(test)]
impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str().eq(other.0.as_str())
    }
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Info {
    pub left: Option<InfoContent>,

    pub right: Option<InfoContent>,

    pub column: Option<InfoContent>,

    #[serde(default)]
    pub variables: HashMap<String, Vec<Matcher>>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields, untagged)]
pub enum InfoContent {
    Plain(String),
    Attrs { text: String, color: Option<Color> },
}

impl InfoContent {
    pub fn get(&self) -> (&str, Option<ansi_term::Style>) {
        match self {
            InfoContent::Plain(s) => (s, None),
            InfoContent::Attrs { text, color } => (text, color.as_ref().map(|c| c.style)),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Collector {
    #[serde(default = "default_true")]
    pub disk_usage: bool,

    #[serde(default = "default_true")]
    pub git_diff: bool,

    pub timeout: Option<Timeout>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Timeout(pub Duration);

fn default_true() -> bool {
    true
}

impl Default for Root {
    fn default() -> Self {
        Root {
            colors: None,
            grid: Grid::default(),
            collector: Collector::default(),
            info: None,
            columns: vec![
                Column {
                    include_hidden: false,
                    label: None,
                    max_name_width: None,
                    matchers: vec![Matcher::Type(FileType::Directory)],
                    exclude: vec![],
                    git_changes_first: true,
                    color: None,
                    sort: None,
                },
                Column {
                    include_hidden: true,
                    label: None,
                    max_name_width: None,
                    matchers: vec![Matcher::Any],
                    exclude: vec![],
                    git_changes_first: true,
                    color: None,
                    sort: None,
                },
            ],
        }
    }
}

impl Default for Collector {
    fn default() -> Self {
        Collector {
            disk_usage: true,
            git_diff: true,
            timeout: Some(Timeout(Duration::from_millis(100))),
        }
    }
}

impl Default for LsColors {
    fn default() -> Self {
        LsColors::Bool(true)
    }
}

impl Default for SortSpec {
    fn default() -> SortSpec {
        SortSpec(SortKey::Name, SortOrder::Asc)
    }
}
