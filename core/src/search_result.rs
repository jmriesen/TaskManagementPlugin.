use std::path::Path;

use serde::Serialize;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

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
    /// A todo whose text is `body`, with `tags` being its own tags plus those
    /// it inherits. See `todos.rs`.
    pub(crate) fn new(body: &str, tags: &[&str], lnum: usize, path: &Path) -> Self {
        //Only display the first line up to the tags.
        let first_line = body.split(['\n', '#']).next().unwrap_or_default();
        let display = format!("{first_line}{}", tags.join(" "));

        SearchResult {
            ordinal: display.clone(),
            display,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            path: path
                .to_str()
                .expect("This only fails if you are doing something funky with path encodings")
                .to_owned(),
            lnum,
        }
    }
}
