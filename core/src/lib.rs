use std::path::Path;
use serde::Serialize;
use tree_sitter::{QueryCursor, StreamingIterator};

use crate::block::BlockTree;

pub mod block;
#[cfg(test)]
mod test_helpers;
mod helper;
mod inline;
use helper::StreamingIteratorExt;

struct ToDoEntry<'a> {
    heading_tags: Vec<&'a str>,
    task_tags: Vec<&'a str>,
    body: &'a str,
    lnum: usize,
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
pub struct SearchResult {
    pub ordinal: String,
    pub display: String,
    pub tags: Vec<String>,
    pub path: String,
    pub lnum: usize,
}

impl SearchResult {
    fn new(
        ToDoEntry {
            heading_tags,
            task_tags,
            body,
            lnum,
        }: &ToDoEntry,
        path: &Path,
    ) -> Self {
        let tags: Vec<_> = task_tags
            .iter()
            .chain(heading_tags)
            .map(|x| x.to_string())
            .collect();

        let mut display = body
            //Only display the first line up to the tags.
            .split(['\n', '#'])
            .next()
            .unwrap_or_default()
            .to_string();

        let tag_string = tags.clone().join(" ");
        display.push_str(&tag_string);

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

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct HeadingMismatch {
    pub path: String,
    pub filename: String,
    pub heading: Option<String>,
}

pub fn find_tags(tree: &BlockTree) -> Vec<String> {
    let mut cursor = QueryCursor::new();
    tree.all_tags(&mut cursor)
}

pub fn find_heading_mismatch(tree: &BlockTree) -> Option<HeadingMismatch> {
    let path = &tree.path;
    let filename = path.file_stem()?.to_str()?.to_owned();

    let mut cursor = QueryCursor::new();
    let heading = tree
        .first_h1(&mut cursor)
        .map(|h| h.text().to_owned());

    if heading.as_deref() == Some(filename.as_str()) {
        None
    } else {
        Some(HeadingMismatch {
            path: path.to_str().expect("non-UTF-8 path").to_owned(),
            filename,
            heading,
        })
    }
}

pub fn find_todos(tree: &BlockTree) -> Vec<SearchResult> {
    let mut cursor = QueryCursor::new();
    tree.todos(&mut cursor)
        .map(|x| SearchResult::new(x, &tree.path))
        .collect()
}

pub fn find_next_actions(tree: &BlockTree) -> Vec<SearchResult> {
    find_todos(tree)
        .into_iter()
        .filter(|x| x.tags.contains(&"#NextAction".into()))
        .collect()
}

pub fn find_requires(tree: &BlockTree) -> std::collections::HashMap<String, Vec<SearchResult>> {
    let mut map: std::collections::HashMap<String, Vec<SearchResult>> =
        std::collections::HashMap::new();
    for result in find_todos(tree) {
        for tag in &result.tags {
            if let Some(person) = tag.strip_prefix("#Requires") {
                map.entry(person.to_string())
                    .or_default()
                    .push(result.clone());
                break;
            }
        }
    }
    map
}

#[test]
fn get_next_actions() {
    let results = find_next_actions(&BlockTree::from_file(Path::new("test_files/test.md")));
    assert_eq!(
        results,
        vec![
            SearchResult {
                display: "Simple task #NextAction".to_string(),
                ordinal: "Simple task #NextAction".to_string(),
                tags: vec!["#NextAction".to_string()],
                path: "test_files/test.md".to_string(),
                lnum: 2,
            },
            SearchResult {
                display: "Second task #NextAction #SecondTag".to_string(),
                ordinal: "Second task #NextAction #SecondTag".to_string(),
                tags: vec!["#NextAction".to_string(), "#SecondTag".to_string()],
                path: "test_files/test.md".to_string(),
                lnum: 4
            }
        ]
    )
}

#[test]
fn get_heading_level_tags() {
    let results = find_todos(&BlockTree::from_file(Path::new("test_files/heading_level_tags.md")));
    dbg!(&results);
    assert_eq!(
        results[0].tags,
        vec!["#NextAction".to_string(), "#HeadingTag".to_string()]
    );
    assert_eq!(results[1].tags, vec!["#HeadingTag".to_string()]);
    assert_eq!(results[2].tags, vec!["#NextAction".to_string(),]);
    assert_eq!(results[3].tags, Vec::<String>::new());
}

#[test]
fn get_requires() {
    let results = find_requires(&BlockTree::from_file(Path::new("test_files/requires_test.md")));
    let mut alice = results["Alice"].clone();
    let mut bob = results["Bob"].clone();
    alice.sort_by_key(|x| x.lnum);
    bob.sort_by_key(|x| x.lnum);
    assert_eq!(
        alice,
        vec![
            SearchResult {
                ordinal: "Follow up with Alice #RequiresAlice".to_string(),
                display: "Follow up with Alice #RequiresAlice".to_string(),
                tags: vec!["#RequiresAlice".to_string()],
                path: "test_files/requires_test.md".to_string(),
                lnum: 2,
            },
            SearchResult {
                ordinal: "Another Alice item #RequiresAlice #NextAction".to_string(),
                display: "Another Alice item #RequiresAlice #NextAction".to_string(),
                tags: vec!["#RequiresAlice".to_string(), "#NextAction".to_string()],
                path: "test_files/requires_test.md".to_string(),
                lnum: 4,
            },
        ]
    );
    assert_eq!(
        bob,
        vec![SearchResult {
            ordinal: "Review doc #RequiresBob".to_string(),
            display: "Review doc #RequiresBob".to_string(),
            tags: vec!["#RequiresBob".to_string()],
            path: "test_files/requires_test.md".to_string(),
            lnum: 3,
        }]
    );
}

#[test]
fn heading_mismatch_detected() {
    // filename is "heading_mismatch_test", heading is "Wrong Heading" — should mismatch
    let result = find_heading_mismatch(&BlockTree::from_file(Path::new("test_files/heading_mismatch_test.md")));
    assert_eq!(
        result,
        Some(HeadingMismatch {
            path: "test_files/heading_mismatch_test.md".to_string(),
            filename: "heading_mismatch_test".to_string(),
            heading: Some("Wrong Heading".to_string()),
        })
    );
}

#[test]
fn heading_match_returns_none() {
    let result = find_heading_mismatch(&BlockTree::from_file(Path::new("test_files/heading_mismatch_test.md")));
    assert!(result.is_some());
}
