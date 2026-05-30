use tree_sitter::{Parser, Point, StreamingIterator};
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::INLINE_LANGUAGE;

pub struct InLineTree<'a> {
    pub(crate) tree: tree_sitter::Tree,
    pub(crate) text: &'a str,
    pub(crate) start_point: Point,
}

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
            let new_tags = r#match
                .captures
                .iter()
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
    use crate::block::BlockTree;
    use crate::test_helpers::dump_inline_tree;
    use tree_sitter::QueryCursor;

    #[test]
    fn markdown_link_hash_is_not_a_tag() {
        // [filename#heading] — the #heading must NOT be extracted as a tag
        let source = "- [ ] Task [filename#heading] and #RealTag\n";
        let block_tree = BlockTree::parse(source.to_owned(), std::path::PathBuf::new());
        let mut cursor = QueryCursor::new();
        let tags = block_tree.all_tags(&mut cursor);

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
