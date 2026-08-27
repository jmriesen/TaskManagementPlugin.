use std::path::Path;

use serde::Serialize;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

/// One unchecked task item found while walking a document, before it's been
/// turned into a [`SearchResult`]. See `todo_walk.rs`.
pub(crate) struct ToDoEntry<'a> {
    pub(crate) heading_tags: Vec<&'a str>,
    pub(crate) task_tags: Vec<&'a str>,
    pub(crate) body: &'a str,
    pub(crate) lnum: usize,
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
#[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
pub struct SearchResult {
    pub ordinal: String,
    pub display: String,
    pub tags: Vec<String>,
    pub path: String,
    pub lnum: usize,
}

impl SearchResult {
    pub(crate) fn new(
        ToDoEntry {
            heading_tags,
            task_tags,
            body,
            lnum,
        }: &ToDoEntry,
        path: &Path,
    ) -> Self {
        let tags: Vec<String> = task_tags
            .iter()
            .chain(heading_tags)
            .map(ToString::to_string)
            .collect();

        //Only display the first line up to the tags.
        let first_line = body.split(['\n', '#']).next().unwrap_or_default();
        let display = format!("{first_line}{}", tags.join(" "));

        SearchResult {
            ordinal: display.clone(),
            display,
            tags,
            path: path
                .to_str()
                .expect("This only fails if you are doing something funky with path encodings")
                .to_owned(),
            lnum: *lnum,
        }
    }
}
