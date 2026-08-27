use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser, Tree};
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::inline::InLineTree;

struct ToDoEntry<'a> {
    heading_tags: Vec<&'a str>,
    task_tags: Vec<&'a str>,
    body: &'a str,
    lnum: usize,
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
    fn new(
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

/// Tag-based filter for search results.
#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
pub struct TagFilter {
    /// Result must have all of these tags.
    #[serde(default)]
    pub include: Vec<String>,
    /// Result must have at least one of these tags (ignored if empty).
    #[serde(default)]
    pub any: Vec<String>,
    /// Result must have none of these tags.
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl TagFilter {
    pub fn matches(&self, tags: &[String]) -> bool {
        self.include.iter().all(|t| tags.contains(t))
            && (self.any.is_empty() || self.any.iter().any(|t| tags.contains(t)))
            && self.exclude.iter().all(|t| !tags.contains(t))
    }
}

/// Apply a [`TagFilter`] to a collection of search results, e.g. the combined
/// todos from every file in a vault.
pub fn filter_todos<I: IntoIterator<Item = SearchResult>>(
    results: I,
    filter: &TagFilter,
) -> Vec<SearchResult> {
    results
        .into_iter()
        .filter(|r| filter.matches(&r.tags))
        .collect()
}

/// Merge the `#Requires<Name>` results from every document in a vault into a
/// single `person -> tasks` map.
///
/// Takes documents by value (rather than `&Document`) so each one — and the
/// tree-sitter `Tree` it holds — can be dropped as soon as its `requires()`
/// have been extracted, instead of every document in the vault staying alive
/// for the whole call. See the note in task-management-bindings/src/lib.rs
/// for why that matters under wasm.
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

/// Collect every descendant node (including `node` itself) whose `kind()` matches, in document order.
fn collect_nodes_of_kind<'a>(node: Node<'a>, kind: &str, out: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nodes_of_kind(child, kind, out);
    }
}

fn first_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

/// Depth-first search for the first `atx_heading` that has both an
/// `atx_h1_marker` and an `inline` child, mirroring the original SELECT_H1 query.
fn find_first_h1<'a>(node: Node, source: &'a str) -> Option<InLineTree<'a>> {
    if node.kind() == "atx_heading" && first_child_of_kind(node, "atx_h1_marker").is_some() {
        if let Some(inline) = first_child_of_kind(node, "inline") {
            return Some(InLineTree::parse(source, inline));
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_first_h1(child, source) {
            return Some(found);
        }
    }
    None
}

/// Walk the tree front to back. Everywhere we find a `section` (a heading
/// plus everything under it), hand it off to `heading_todos`; everywhere
/// else, just keep walking through the children.
fn walk_todos<'a>(
    node: Node<'a>,
    source: &'a str,
    ancestor_tags: &[&'a str],
) -> Vec<ToDoEntry<'a>> {
    if node.kind() == "section" {
        return heading_todos(node, source, ancestor_tags);
    }

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .flat_map(|child| walk_todos(child, source, ancestor_tags))
        .collect()
}

/// One heading `section`'s todo entries: its own unchecked task items, tagged
/// with `ancestor_tags` plus this heading's own tags, followed by whatever
/// `walk_todos` finds in the rest of the section (including nested
/// sub-headings, which is why those pick up this heading's tags too).
fn heading_todos<'a>(
    section: Node<'a>,
    source: &'a str,
    ancestor_tags: &[&'a str],
) -> Vec<ToDoEntry<'a>> {
    let mut tags = ancestor_tags.to_vec();
    if let Some(heading_inline) = first_child_of_kind(section, "atx_heading")
        .and_then(|atx_heading| first_child_of_kind(atx_heading, "inline"))
    {
        tags.extend(InLineTree::parse(source, heading_inline).tags());
    }

    let own_todos = match first_child_of_kind(section, "list") {
        Some(list) => task_list_entries(list, source, &tags),
        None => vec![],
    };

    let mut cursor = section.walk();
    let nested_todos = section
        .children(&mut cursor)
        .flat_map(|child| walk_todos(child, source, &tags));

    own_todos.into_iter().chain(nested_todos).collect()
}

/// Every unchecked task item directly in `list`, tagged with `heading_tags`.
fn task_list_entries<'a>(
    list: Node,
    source: &'a str,
    heading_tags: &[&'a str],
) -> Vec<ToDoEntry<'a>> {
    let mut cursor = list.walk();
    list.children(&mut cursor)
        .filter(|item| item.kind() == "list_item")
        .filter(|item| first_child_of_kind(*item, "task_list_marker_unchecked").is_some())
        .filter_map(|item| {
            let paragraph = first_child_of_kind(item, "paragraph")?;
            let task_inline = first_child_of_kind(paragraph, "inline")?;
            let task_tree = InLineTree::parse(source, task_inline);
            Some(ToDoEntry {
                body: task_tree.text,
                heading_tags: heading_tags.to_vec(),
                task_tags: task_tree.tags(),
                lnum: task_tree.start_point.row + 1,
            })
        })
        .collect()
}
