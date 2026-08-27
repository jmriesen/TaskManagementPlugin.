use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tree_sitter::{Parser, Tree};
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::inline::InLineTree;
use crate::search_result::{SearchResult, ToDoEntry};
use crate::todo_walk::walk_todos;
use crate::tree_util::{collect_nodes_of_kind, find_first_h1};

/// Merge the `#Requires<Name>` results from every document in a vault into a
/// single `person -> tasks` map.
///
/// Takes documents by value (rather than `&Document`) so each one — and the
/// tree-sitter `Tree` it holds — can be dropped as soon as its `requires()`
/// have been extracted, instead of every document in the vault staying alive
/// for the whole call. See the note in task-management-bindings/src/lib.rs
/// for why that matters under wasm.
pub fn combine_requires<I: IntoIterator<Item = Document>>(documents: I) -> HashMap<String, Vec<SearchResult>> {
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

pub struct Document {
    tree: Tree,
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
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_md::LANGUAGE.into())
            .expect("Error loading Markdown grammar");
        let tree = parser.parse(&source, None).unwrap();
        Self { tree, source, path }
    }

    pub fn tags(&self) -> Vec<String> {
        let mut inline_nodes = vec![];
        collect_nodes_of_kind(self.tree.root_node(), "inline", &mut inline_nodes);

        let mut tags = vec![];
        for node in inline_nodes {
            let inline_tree = InLineTree::parse(&self.source, node);
            tags.extend(inline_tree.tags().iter().map(|s| s.to_string()));
        }
        tags
    }

    pub fn heading_mismatch(&self) -> Option<HeadingMismatch> {
        let filename = self.path.file_stem()?.to_str()?.to_owned();

        let heading = self.first_h1().map(|h| h.text().to_owned());

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
        self.todo_entries()
            .iter()
            .map(|x| SearchResult::new(x, &self.path))
            .collect()
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

    fn first_h1(&self) -> Option<InLineTree<'_>> {
        find_first_h1(self.tree.root_node(), &self.source)
    }

    fn todo_entries(&self) -> Vec<ToDoEntry<'_>> {
        walk_todos(self.tree.root_node(), &self.source, &[])
    }
}
