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
    pub fn extract_todos<'source, 'cursor, 'tree>(
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
                //Come back and extract this.
                let headline = r#match
                    .nodes_for_capture_index(
                        SELECT_TODO
                            .capture_index_for_name("heading.inline")
                            .unwrap(),
                    )
                    .next()
                    .expect("expect exacly one node");
                let task_nodes = r#match
                    .nodes_for_capture_index(
                        SELECT_TODO.capture_index_for_name("task.inline").unwrap(),
                    )
                    .next()
                    .expect("expect exacly one node");

                let headline_tree = InLineTree::parse(source, headline);
                let task_tree = InLineTree::parse(source, task_nodes);
                SELECT_TODO.capture_names()[r#match.captures[0].index as usize];

                ToDoEntry {
                    body: task_tree.text,
                    heading_tags: headline_tree.extract_tags(),
                    task_tags: task_tree.extract_tags(),
                    lnum: task_tree.start_point.row + 1,
                }
            })
    }
}
