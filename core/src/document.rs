use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
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

/// Tag-based filter for search results.
#[derive(Debug, Default, Clone)]
pub struct TagFilter {
    /// Result must have all of these tags.
    pub include: Vec<String>,
    /// Result must have at least one of these tags (ignored if empty).
    pub any: Vec<String>,
    /// Result must have none of these tags.
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
pub fn combine_requires<'a, I: IntoIterator<Item = &'a Document>>(
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

    // TEMPORARY WORKAROUND (added 2026-08-20, expected to be reverted within a
    // year): this used to be a tree-sitter `Query`:
    //
    //     static SELECT_INLINE: LazyLock<Query> = LazyLock::new(|| {
    //         Query::new(&tree_sitter_md::LANGUAGE.into(), "(inline) @inline").unwrap()
    //     });
    //     let mut cursor = QueryCursor::new();
    //     let mut matches = cursor.matches(&SELECT_INLINE, self.tree.root_node(), self.source.as_bytes());
    //     ...(iterate matches, InLineTree::parse each captured node, collect tags)...
    //
    // See the identical note on `InLineTree::tags` in inline.rs for why: `Query::new`
    // causes memory corruption under wasm32-unknown-unknown, in tree-sitter's own C
    // query compiler, not in this crate or the tree-sitter-markdown grammar. This
    // walks the tree manually instead, since `Parser::parse` is unaffected. Revert
    // to the `Query`-based version above once the upstream bug is fixed.
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

    pub fn next_actions(&self) -> Vec<SearchResult> {
        self.todos()
            .into_iter()
            .filter(|x| x.tags.contains(&"#NextAction".into()))
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

    // TEMPORARY WORKAROUND (see the note on `tags` above for why): this used
    // to be a tree-sitter `Query`:
    //
    //     static SELECT_H1: LazyLock<Query> = LazyLock::new(|| {
    //         Query::new(&tree_sitter_md::LANGUAGE.into(), "(atx_heading (atx_h1_marker) (inline) @heading)").unwrap()
    //     });
    //     cursor.matches(&SELECT_H1, self.tree.root_node(), self.source.as_bytes())
    //         .next()
    //         .map(|m| InLineTree::parse(&self.source, m.captures[0].node))
    fn first_h1(&self) -> Option<InLineTree<'_>> {
        find_first_h1(self.tree.root_node(), &self.source)
    }

    // TEMPORARY WORKAROUND (see the note on `tags` above for why): this used
    // to be a tree-sitter `Query`:
    //
    //     static SELECT_TODO: LazyLock<Query> = LazyLock::new(|| {
    //         Query::new(&tree_sitter_md::LANGUAGE.into(), "
    //             (section
    //               (atx_heading (_) (inline) @heading.inline)
    //               (list (list_item (task_list_marker_unchecked) (paragraph ((inline) @task.inline)))))
    //         ").unwrap()
    //     });
    //     cursor.matches(&SELECT_TODO, self.tree.root_node(), self.source.as_bytes())
    //         .map(|match| { ...extract heading.inline and task.inline captures, build a ToDoEntry per match... })
    fn todo_entries(&self) -> Vec<ToDoEntry<'_>> {
        let mut sections = vec![];
        collect_nodes_of_kind(self.tree.root_node(), "section", &mut sections);

        sections
            .into_iter()
            .flat_map(|section| section_todo_entries(section, &self.source))
            .collect()
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

/// One `section`'s todo entries, mirroring what SELECT_TODO matched per section:
/// the section's heading text paired with each unchecked task item under it.
fn section_todo_entries<'a>(section: Node, source: &'a str) -> Vec<ToDoEntry<'a>> {
    let Some(atx_heading) = first_child_of_kind(section, "atx_heading") else {
        return vec![];
    };
    let Some(heading_inline) = first_child_of_kind(atx_heading, "inline") else {
        return vec![];
    };
    let headline = InLineTree::parse(source, heading_inline);

    let Some(list) = first_child_of_kind(section, "list") else {
        return vec![];
    };

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
                heading_tags: headline.tags(),
                task_tags: task_tree.tags(),
                lnum: task_tree.start_point.row + 1,
            })
        })
        .collect()
}
