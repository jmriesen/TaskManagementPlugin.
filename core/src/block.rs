use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use tree_sitter::{Parser, StreamingIterator};
use tree_sitter::{Query, QueryCursor, Tree};

use crate::inline::InLineTree;
use crate::ToDoEntry;

pub struct BlockTree {
    tree: Tree,
    source: String,
    pub path: PathBuf,
}

impl BlockTree {
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

    pub fn first_h1<'a>(&'a self, cursor: &'a mut QueryCursor) -> Option<InLineTree<'a>> {
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

    pub fn all_tags(&self, cursor: &mut QueryCursor) -> Vec<String> {
        static SELECT_INLINE: LazyLock<Query> = LazyLock::new(|| {
            Query::new(&tree_sitter_md::LANGUAGE.into(), "(inline) @inline").unwrap()
        });

        let mut tags = vec![];
        let mut matches = cursor.matches(&SELECT_INLINE, self.tree.root_node(), self.source.as_bytes());
        while let Some(m) = matches.next() {
            let inline_tree = InLineTree::parse(&self.source, m.captures[0].node);
            tags.extend(inline_tree.tags().into_iter().map(|s| s.to_string()));
        }
        tags
    }

    pub fn todos<'a>(&'a self, cursor: &'a mut QueryCursor) -> impl StreamingIterator<Item = ToDoEntry<'a>> + use<'a> {
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
