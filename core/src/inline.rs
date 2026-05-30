use std::marker::PhantomData;

use tree_sitter::{Parser, StreamingIterator};
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::INLINE_LANGUAGE;

use crate::{MyTree, TreeType};

pub struct Inline;
pub type InLineTree<'a> = MyTree<'a, Inline>;
impl TreeType for Inline {}

fn has_link_ancestor(node: tree_sitter::Node) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        if n.kind() == "link_text" {
            return true;
        }
        current = n.parent();
    }
    false
}

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

    pub fn text(&self) -> &'a str {
        self.text
    }

    pub fn tags(&self) -> Vec<&'a str> {
        let select_tags = &Query::new(&INLINE_LANGUAGE.into(), "(tag) @text").unwrap();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(select_tags, self.tree.root_node(), self.text.as_bytes());
        let mut results = vec![];

        while let Some(r#match) = matches.next() {
            let new_tags = r#match.captures.iter()
                .map(|cap| cap.node)
                .filter(|node| !has_link_ancestor(*node))
                .map(|node| node.utf8_text(self.text.as_bytes()).unwrap());
            results.extend(new_tags);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockTree;
    use tree_sitter::QueryCursor;

    fn dump_inline_tree(text: &str) -> String {
        let mut inline_parser = tree_sitter::Parser::new();
        inline_parser.set_language(&INLINE_LANGUAGE.into()).unwrap();
        let tree = inline_parser.parse(text, None).unwrap();

        fn walk(node: tree_sitter::Node, src: &[u8], depth: usize, out: &mut String) {
            let indent = "  ".repeat(depth);
            let snippet: String = node.utf8_text(src).unwrap_or("").chars().take(40).collect();
            out.push_str(&format!("{}{} {:?}\n", indent, node.kind(), snippet));
            for i in 0..node.child_count() {
                walk(node.child(i as u32).unwrap(), src, depth + 1, out);
            }
        }

        let mut out = String::new();
        walk(tree.root_node(), text.as_bytes(), 0, &mut out);
        out
    }

    #[test]
    fn markdown_link_hash_is_not_a_tag() {
        // [filename#heading] — the #heading must NOT be extracted as a tag
        let source = "- [ ] Task [filename#heading] and #RealTag\n";
        let block_tree = BlockTree::parse(source);
        let mut cursor = QueryCursor::new();
        let tags = block_tree.all_tags(source, &mut cursor);
        assert!(
            !tags.iter().any(|t| t == "#heading"),
            "link anchor #heading was incorrectly extracted as a tag.\nTree:\n{}",
            dump_inline_tree("Task [filename#heading] and #RealTag")
        );
        assert!(
            tags.contains(&"#RealTag".to_string()),
            "real tag #RealTag was not found"
        );
    }
}
