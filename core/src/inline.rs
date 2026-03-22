use std::marker::PhantomData;

use tree_sitter::{Parser, StreamingIterator};
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::INLINE_LANGUAGE;

use crate::{MyTree, TreeType};

pub struct Inline;
pub type InLineTree<'a> = MyTree<'a, Inline>;
impl TreeType for Inline {}

impl<'a> InLineTree<'a> {
    pub fn parse(source: &'a str, node: tree_sitter::Node<'_>) -> Self {
        let text = node.utf8_text(source.as_bytes()).unwrap();
        let start_point = node.start_position();

        let mut inline_parser = Parser::new();
        inline_parser.set_language(&INLINE_LANGUAGE.into()).unwrap();
        let tree = inline_parser.parse(text, None).unwrap();

        Self {
            tree,
            text,
            start_point,
            _phantom: PhantomData::default(),
        }
    }

    pub fn tags(&self) -> Vec<&'a str> {
        let select_tags = &Query::new(&INLINE_LANGUAGE.into(), "(tag) @text").unwrap();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(select_tags, self.tree.root_node(), self.text.as_bytes());
        let mut results = vec![];

        while let Some(r#match) = matches.next() {
            for cap in r#match.captures {
                //Only one capture in the query so we don't need to check capture name.
                let node = cap.node;
                let text = node.utf8_text(self.text.as_bytes()).unwrap();
                results.push(text);
            }
        }
        results
    }
}
