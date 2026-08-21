use std::path::PathBuf;

use core::document::Document;
use core::{combine_requires, filter_todos, parse_tag_filters, HeadingMismatch, SearchResult, TagFilter};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ExcerptDirective {
    file_path: String,
    line_start: usize,
    line_end: usize,
}

#[wasm_bindgen]
impl ExcerptDirective {
    #[wasm_bindgen(getter, js_name = filePath)]
    pub fn file_path(&self) -> String {
        self.file_path.clone()
    }

    #[wasm_bindgen(getter, js_name = lineStart)]
    pub fn line_start(&self) -> usize {
        self.line_start
    }

    #[wasm_bindgen(getter, js_name = lineEnd)]
    pub fn line_end(&self) -> usize {
        self.line_end
    }
}

#[wasm_bindgen(js_name = parseExcerptDirective)]
pub fn parse_excerpt_directive(source: &str) -> Result<ExcerptDirective, JsValue> {
    let mut file_path: Option<String> = None;
    let mut lines_param: Option<String> = None;

    for line in source.trim().lines() {
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_lowercase();
            let value = line[colon_pos + 1..].trim().to_string();
            match key.as_str() {
                "file" => file_path = Some(value),
                "lines" => lines_param = Some(value),
                _ => {}
            }
        }
    }

    let file_path = file_path
        .ok_or_else(|| JsValue::from_str("Missing required 'file' field in excerpt directive"))?;
    let lines_param = lines_param
        .ok_or_else(|| JsValue::from_str("Missing required 'lines' field in excerpt directive"))?;

    let parts: Vec<&str> = lines_param.splitn(2, '-').collect();
    if parts.len() != 2 {
        return Err(JsValue::from_str(
            "Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')",
        ));
    }

    let line_start = parts[0].trim().parse::<usize>().map_err(|_| {
        JsValue::from_str("Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')")
    })?;
    let line_end = parts[1].trim().parse::<usize>().map_err(|_| {
        JsValue::from_str("Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')")
    })?;

    if line_start > line_end {
        return Err(JsValue::from_str(
            "Line start must be less than or equal to line end",
        ));
    }

    Ok(ExcerptDirective {
        file_path,
        line_start,
        line_end,
    })
}

#[wasm_bindgen(js_name = extractContent)]
pub fn extract_content(full_content: &str, line_start: usize, line_end: usize) -> String {
    full_content
        .lines()
        .skip(line_start)
        .take(line_end - line_start)
        .collect::<Vec<_>>()[..]
        .join("\n")
}

/// One vault file, read by the TS side (wasm has no filesystem access).
#[wasm_bindgen(getter_with_clone)]
pub struct VaultFile {
    pub path: String,
    pub content: String,
}

#[wasm_bindgen]
impl VaultFile {
    #[wasm_bindgen(constructor)]
    pub fn new(path: String, content: String) -> Self {
        VaultFile { path, content }
    }
}

/// All of one person's `#Requires<Name>` tasks — the wasm-friendly stand-in
/// for a `person -> tasks` map, since wasm-bindgen has no native map/record type.
#[wasm_bindgen(getter_with_clone)]
pub struct PersonResults {
    pub person: String,
    pub tasks: Vec<SearchResult>,
}

fn parse_documents(files: Vec<VaultFile>) -> Vec<Document> {
    files
        .into_iter()
        .map(|f| Document::parse(f.content, PathBuf::from(f.path)))
        .collect()
}

#[wasm_bindgen(js_name = searchTodos)]
pub fn search_todos(
    files: Vec<VaultFile>,
    include: Vec<String>,
    any: Vec<String>,
    exclude: Vec<String>,
) -> Vec<SearchResult> {
    let documents = parse_documents(files);
    let filter = TagFilter {
        include,
        any,
        exclude,
    };
    let todos = documents.iter().flat_map(Document::todos);
    filter_todos(todos, &filter)
}

#[wasm_bindgen(js_name = searchRequires)]
pub fn search_requires(files: Vec<VaultFile>) -> Vec<PersonResults> {
    let documents = parse_documents(files);
    combine_requires(&documents)
        .into_iter()
        .map(|(person, tasks)| PersonResults { person, tasks })
        .collect()
}

#[wasm_bindgen(js_name = headingCheck)]
pub fn heading_check(files: Vec<VaultFile>) -> Vec<HeadingMismatch> {
    let documents = parse_documents(files);
    documents.iter().filter_map(Document::heading_mismatch).collect()
}

#[wasm_bindgen(js_name = collectTags)]
pub fn collect_tags(files: Vec<VaultFile>) -> Vec<String> {
    let documents = parse_documents(files);
    let mut tags: Vec<String> = documents.iter().flat_map(Document::tags).collect();
    tags.sort();
    tags.dedup();
    tags
}

/// One entry of the `name -> TagFilter` map `core::parse_tag_filters` returns —
/// wasm-bindgen has no native map/record type, so the map crosses the
/// boundary as a `Vec` of these, same trick as `PersonResults` above.
#[wasm_bindgen(getter_with_clone)]
pub struct NamedTagFilter {
    pub name: String,
    pub filter: TagFilter,
}

/// Parse a shared TOML config file (see `core::tag_filters` for the format)
/// into named tag filters the plugin can turn into commands.
#[wasm_bindgen(js_name = parseTagFilters)]
pub fn parse_tag_filters_js(source: &str) -> Result<Vec<NamedTagFilter>, JsValue> {
    parse_tag_filters(source)
        .map(|filters| {
            filters
                .into_iter()
                .map(|(name, filter)| NamedTagFilter { name, filter })
                .collect()
        })
        .map_err(|e| JsValue::from_str(&e))
}
