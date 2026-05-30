use std::marker::PhantomData;
use std::sync::LazyLock;

use tree_sitter::{Parser, StreamingIterator};
use tree_sitter::{Query, QueryCursor};

use crate::inline::InLineTree;
use crate::{MyTree, ToDoEntry, TreeType};

pub struct Block;
pub type BlockTree<'a> = MyTree<'a, Block>;
impl TreeType for Block {}

impl<'a> BlockTree<'a> {
    pub fn parse(source: &'a str) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_md::LANGUAGE.into())
            .expect("Error loading Markdown inline grammar");
        let tree = parser.parse(&source, None).unwrap();

        Self {
            tree,
            text: source,
            start_point: tree_sitter::Point { row: 0, column: 0 },
            _phantom: PhantomData::default(),
        }
    }
    pub fn first_h1<'source, 'cursor, 'tree>(
        &'tree self,
        source: &'source str,
        cursor: &'cursor mut QueryCursor,
    ) -> Option<InLineTree<'source>>
    where
        'tree: 'cursor,
    {
        static SELECT_H1: LazyLock<Query> = LazyLock::new(|| {
            Query::new(
                &tree_sitter_md::LANGUAGE.into(),
                "(atx_heading (atx_h1_marker) (inline) @heading)",
            )
            .unwrap()
        });

        cursor
            .matches(&SELECT_H1, self.tree.root_node(), source.as_bytes())
            .next()
            .map(|m| InLineTree::parse(source, m.captures[0].node))
    }

    pub fn all_tags<'source, 'cursor, 'tree>(
        &'tree self,
        source: &'source str,
        cursor: &'cursor mut QueryCursor,
    ) -> Vec<String>
    where
        'tree: 'cursor,
    {
        static SELECT_INLINE: LazyLock<Query> = LazyLock::new(|| {
            Query::new(&tree_sitter_md::LANGUAGE.into(), "(inline) @inline").unwrap()
        });

        let mut tags = vec![];
        let mut matches = cursor.matches(&SELECT_INLINE, self.tree.root_node(), source.as_bytes());
        while let Some(m) = matches.next() {
            let inline_tree = InLineTree::parse(source, m.captures[0].node);
            tags.extend(inline_tree.tags().into_iter().map(|s| s.to_string()));
        }
        tags
    }

    pub fn todos<'source, 'cursor, 'tree>(
        &'tree self,
        source: &'source str,
        cursor: &'cursor mut QueryCursor,
    ) -> impl StreamingIterator<Item = ToDoEntry<'source>> + use<'source, 'cursor>
    where
        'tree: 'cursor,
    {
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
            .matches(&SELECT_TODO, self.tree.root_node(), source.as_bytes())
            .map(move |r#match| {
                let extract = |id| {
                    let node = r#match
                        .nodes_for_capture_index(SELECT_TODO.capture_index_for_name(id).unwrap())
                        .next()
                        .expect("expect exacly one node");
                    InLineTree::parse(source, node)
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
