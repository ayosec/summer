//! Build script to generate a processor for MIME data.
//!
//! Data is read from the copy of the `shared-mime-info` specification, in the
//! `vendor` directory.
//!
//! The process can be summarized in the following steps:
//!
//! 1. The parser reads the XML file, and then extracts the MIME types, and a
//!    list of extensions for each one.
//!
//!    Complex patterns (like `[0-9][0-9][0-9].vdr` for `video/mpeg`) are
//!    ignored.
//!
//!    Subtypes (i.e. `png` in `image/png`) are ignored.
//!
//! 2. Then, it generates a tree for every byte in the collected extensions.
//!
//!    For example, if the XML contains the extensions `*.abx` and `*.aby`, the
//!    tree will be like this:
//!
//!    ```notrust
//!    a
//!     \
//!      b
//!     / \
//!    x   y
//!    ```
//!
//! 3. Finally, the tree is converted to nested `match` sentences.
//!
//!    For the previous tree, the code could look to something like this:
//!
//!
//!    ```ignore
//!    match bytes.next() {
//!       Some(b'a') => match bytes.next() {
//!           Some(b'b') => match bytes.next() {
//!               Some(b'x') => match bytes.next() {
//!                   None => Some(MimeType::FOO),
//!                   _ => None,
//!               }
//!
//!               Some(b'y') => match bytes.next() {
//!                   None => Some(MimeType::FOO),
//!                   _ => None,
//!               }
//!
//!               _ => None,
//!           }
//!
//!           _ => None,
//!       }
//!
//!       _ => None
//!    }
//!    ```
//!
//! [`shared-mime-info`]: https://www.freedesktop.org/wiki/Specifications/shared-mime-info-spec/

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use quote::__private::TokenStream;
use quote::quote;

const MIME_DATA_SOURCE: &str = "vendor/shared-mime-info/MIME.xml.gz";

const MIME_DATA_OUTPUT: &str = "mime_data.rs";

/// Map to associate extensions for every MIME type.
type MimeTypes = HashMap<String, Vec<String>>;

/// Tree to traverse all strings for MIME extensions.
///
/// `BTreeMap` is used to keep the nodes sorted by their keys.
type ValuesTree<'a> = BTreeMap<u8, TreeNode<'a>>;

/// Nodes in the values tree.
#[derive(Default)]
struct TreeNode<'a> {
    subtree: ValuesTree<'a>,
    mime_type: Option<&'a str>,
}

/// Read the XML file and collect extensions for every MIME type.
///
/// The XML file is expected to be compressed with Gzip.
fn parse_mime_data(source: impl AsRef<Path>) -> Result<MimeTypes, Box<dyn Error>> {
    // Uncompress XML file.
    let xml_source = {
        let mut data = String::new();
        let mut input = GzDecoder::new(BufReader::new(File::open(source)?));
        input.read_to_string(&mut data)?;
        data
    };

    // Parse XML document.
    let doc = roxmltree::Document::parse_with_options(
        &xml_source,
        roxmltree::ParsingOptions { allow_dtd: true },
    )?;

    // Extract MIME types and their extensions through <glob> nodes under
    // <mime-type>.
    //
    // We have to track what extension have been added because some extensions
    // may appear in multiple <mime-type> elements.

    let root = doc.root_element();
    if !root.has_tag_name("mime-info") {
        return Err("Expected <mime-info> as the root element".into());
    }

    let mut found_exts = HashSet::new();
    let mut mime_types: MimeTypes = MimeTypes::new();

    for elem in root.children() {
        if elem.has_tag_name("mime-type") {
            let mime_type = elem
                .attribute("type")
                .and_then(|t| t.split('/').next())
                .filter(|name| name.chars().all(char::is_alphanumeric));

            if let Some(mime_type) = mime_type {
                for elem in elem.children() {
                    if let Some(ext) = parse_glob_elem(&elem) {
                        if found_exts.insert(ext.to_owned()) {
                            mime_types
                                .entry(mime_type.to_owned())
                                .or_insert_with(Vec::new)
                                .push(ext.to_owned())
                        }
                    }
                }
            }
        }
    }

    Ok(mime_types)
}

/// Extract the extension from a <glob> node.
///
/// It discards the pattern if it is not just `*.[a-zA-Z0-9]`.
fn parse_glob_elem<'a>(elem: &'a roxmltree::Node) -> Option<&'a str> {
    if !elem.has_tag_name("glob") {
        return None;
    }

    elem.attribute("pattern")?
        .strip_prefix("*.")
        .filter(|pat| pat.chars().all(char::is_alphanumeric))
}

/// Variant name for a MIME type string.
macro_rules! mime_ident {
    ($mime:expr) => {
        quote::format_ident!("{}", $mime.to_uppercase())
    };
}

/// Write the Rust code to use the MIME types.
fn write_mime_types(
    mime_types: &MimeTypes,
    output: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    let values_tree = build_values_tree(mime_types);
    let parser = parser_tree(&values_tree);

    // Variants for every MIME.
    let variants = mime_types.keys().map(|mime| mime_ident!(mime));

    // Iterator to get bytes from the extension.

    let bytes_iter = match std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() {
        Ok("unix") => {
            quote! {
                use std::os::unix::ffi::OsStrExt;
                let mut bytes = ext.as_bytes().iter();
            }
        }

        Ok("windows") => {
            quote! {
                use std::os::windows::ffi::OsStrExt;
                let mut bytes = ext.encode_wide();
            }
        }

        _ => panic!("Unsupported target."),
    };

    // Final module.
    let tokens = quote! {
        #[repr(u8)]
        #[allow(clippy::upper_case_acronyms)]
        #[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "lowercase")]
        pub enum MimeType {
            #(#variants,)*
        }

        impl MimeType {
            pub fn from_extension(ext: &std::ffi::OsStr) -> Option<Self> {
                use MimeType::*;

                #bytes_iter
                #parser
            }
        }
    };

    // Write code to the final target.
    let mut output = BufWriter::new(File::create(output).expect("Create MIME_DATA_OUTPUT"));
    write!(output, "{}", tokens)?;

    Ok(())
}

fn build_values_tree(mime_types: &MimeTypes) -> ValuesTree {
    let mut tree = ValuesTree::new();

    for (mime_type, extensions) in mime_types {
        for ext in extensions {
            let mut node = &mut tree;
            for byte in ext.as_bytes() {
                node = &mut node.entry(*byte).or_default().subtree;
            }

            let leaf = &mut node.entry(0).or_default();

            if let Some(prev) = leaf.mime_type {
                panic!("Duplicated mime_type: {}", prev);
            }

            leaf.mime_type = Some(mime_type.as_ref());
        }
    }

    tree
}

struct RawLiteral<T>(T);

impl<T: std::fmt::Display> quote::ToTokens for RawLiteral<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(format!("{}", self.0).parse::<TokenStream>().unwrap())
    }
}

fn parser_tree(tree: &ValuesTree) -> TokenStream {
    let branches = tree.iter().map(|(&byte, node)| {
        if byte == 0 {
            let mime = mime_ident!(node.mime_type.unwrap());
            quote! {
                None => { Some(#mime) }
            }
        } else {
            let byte = RawLiteral(byte);
            let subtree = parser_tree(&node.subtree);
            quote! {
                Some(#byte) => { #subtree }
            }
        }
    });

    quote! {
        match bytes.next() {
            #(#branches),*

            _ => None,
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed={}", MIME_DATA_SOURCE);

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let mime_types = parse_mime_data(MIME_DATA_SOURCE).expect("Parse MIME_DATA_SOURCE");
    write_mime_types(&mime_types, out_dir.join(MIME_DATA_OUTPUT)).expect("Create MIME_DATA_OUTPUT");
}
