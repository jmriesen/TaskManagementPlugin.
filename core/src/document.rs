use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pulldown_cmark::HeadingLevel;
use serde::Serialize;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::markdown::{self, OutlineEvent, outline};
use crate::search_result::SearchResult;

/// Merge the `#Requires<Name>` results from every document in a vault into a
/// single `person -> tasks` map.
///
/// Takes documents by value (rather than `&Document`) so each one can be
/// dropped as soon as its `requires()` have been extracted, instead of every
/// document in the vault staying alive for the whole call. See the note in
/// task-management-bindings/src/lib.rs for why that matters under wasm.
pub fn combine_requires<I: IntoIterator<Item = Document>>(
    documents: I,
) -> HashMap<String, Vec<SearchResult>> {
    let mut combined: HashMap<String, Vec<SearchResult>> = HashMap::new();
    for document in documents {
        for (person, results) in document.requires() {
            combined.entry(person).or_default().extend(results);
        }
    }
    combined
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
pub struct HeadingMismatch {
    pub path: String,
    pub filename: String,
    pub heading: Option<String>,
}

/// One markdown file. Each method streams through `source` with
/// pulldown-cmark as it needs to; nothing is parsed up front.
pub struct Document {
    source: String,
    pub path: PathBuf,
}

impl Document {
    pub fn from_file(path: &Path) -> Self {
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("error reading {}: {e}", path.display()));
        Self::parse(source, path.to_path_buf())
    }

    pub fn parse(source: String, path: PathBuf) -> Self {
        Self { source, path }
    }

    pub fn tags(&self) -> Vec<String> {
        markdown::tags(&self.source)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    pub fn heading_mismatch(&self) -> Option<HeadingMismatch> {
        let filename = self.path.file_stem()?.to_str()?.to_owned();

        let heading = self.first_h1();

        if heading.as_deref() == Some(filename.as_str()) {
            None
        } else {
            Some(HeadingMismatch {
                path: self.path.to_str().expect("non-UTF-8 path").to_owned(),
                filename,
                heading,
            })
        }
    }

    pub fn todos(&self) -> Vec<SearchResult> {
        crate::todos::todos(&self.source, &self.path)
    }

    pub fn requires(&self) -> HashMap<String, Vec<SearchResult>> {
        let mut map: HashMap<String, Vec<SearchResult>> = HashMap::new();
        for result in self.todos() {
            if let Some(person) = result.tags.iter().find_map(|t| t.strip_prefix("#Requires")) {
                map.entry(person.to_string()).or_default().push(result);
            }
        }
        map
    }

    /// Stops reading at the first H1.
    fn first_h1(&self) -> Option<String> {
        outline(&self.source).find_map(|event| match event {
            OutlineEvent::Heading {
                level: HeadingLevel::H1,
                text,
                ..
            } => Some(text.to_owned()),
            _ => None,
        })
    }
}
