use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::Serialize;
use tree_sitter::{Parser, StreamingIterator};
use tree_sitter::{Query, QueryCursor, Tree};

use crate::helper::StreamingIteratorExt;
use crate::inline::InLineTree;

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
        static SELECT_INLINE: LazyLock<Query> = LazyLock::new(|| {
            Query::new(&tree_sitter_md::LANGUAGE.into(), "(inline) @inline").unwrap()
        });

        let mut cursor = QueryCursor::new();
        let mut tags = vec![];
        let mut matches = cursor.matches(&SELECT_INLINE, self.tree.root_node(), self.source.as_bytes());
        while let Some(m) = matches.next() {
            let inline_tree = InLineTree::parse(&self.source, m.captures[0].node);
            tags.extend(inline_tree.tags().into_iter().map(|s| s.to_string()));
        }
        tags
    }

    pub fn heading_mismatch(&self) -> Option<HeadingMismatch> {
        let filename = self.path.file_stem()?.to_str()?.to_owned();

        let mut cursor = QueryCursor::new();
        let heading = self.first_h1(&mut cursor).map(|h| h.text().to_owned());

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
        let mut cursor = QueryCursor::new();
        self.todo_entries(&mut cursor)
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

    fn first_h1<'a>(&'a self, cursor: &'a mut QueryCursor) -> Option<InLineTree<'a>> {
        static SELECT_H1: LazyLock<Query> = LazyLock::new(|| {
            Query::new(
                &tree_sitter_md::LANGUAGE.into(),
                "(atx_heading (atx_h1_marker) (inline) @heading)",
            )
            .unwrap()
        });

        cursor
            .matches(&SELECT_H1, self.tree.root_node(), self.source.as_bytes())
            .next()
            .map(|m| InLineTree::parse(&self.source, m.captures[0].node))
    }

    fn todo_entries<'a>(&'a self, cursor: &'a mut QueryCursor) -> impl StreamingIterator<Item = ToDoEntry<'a>> + use<'a> {
        static SELECT_TODO: LazyLock<Query> = LazyLock::new(|| {
            Query::new(
                &tree_sitter_md::LANGUAGE.into(),
                "
(section
(atx_heading
      (_)
        (inline) @heading.inline)
    (list
        (list_item
                (task_list_marker_unchecked)
                    (paragraph
                      ((inline) @task.inline)
                    )
        )
    )
)
",
            )
            .unwrap()
        });

        cursor
            .matches(&SELECT_TODO, self.tree.root_node(), self.source.as_bytes())
            .map(move |r#match| {
                let extract = |id| {
                    let node = r#match
                        .nodes_for_capture_index(SELECT_TODO.capture_index_for_name(id).unwrap())
                        .next()
                        .expect("expect exactly one node");
                    InLineTree::parse(&self.source, node)
                };
                let headline = extract("heading.inline");
                let task_tree = extract("task.inline");

                ToDoEntry {
                    body: task_tree.text,
                    heading_tags: headline.tags(),
                    task_tags: task_tree.tags(),
                    lnum: task_tree.start_point.row + 1,
                }
            })
    }
}
