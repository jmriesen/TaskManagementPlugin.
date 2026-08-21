use tree_sitter::{Parser, Point};
use tree_sitter_md::INLINE_LANGUAGE;

pub struct InLineTree<'a> {
    tree: tree_sitter::Tree,
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

    // TEMPORARY WORKAROUND (added 2026-08-20, expected to be reverted within a
    // year): this used to be a tree-sitter `Query`:
    //
    //     static SELECT_TAGS: LazyLock<Query> = LazyLock::new(|| {
    //         Query::new(&INLINE_LANGUAGE.into(), "(tag) @text").unwrap()
    //     });
    //     let mut cursor = QueryCursor::new();
    //     let mut matches = cursor.matches(&SELECT_TAGS, self.tree.root_node(), self.text.as_bytes());
    //     ...(iterate matches, collect captures, filter out link_text ancestors)...
    //
    // `Query::new` against INLINE_LANGUAGE causes memory corruption when compiled
    // to wasm32-unknown-unknown (confirmed: repeated calls fail unpredictably —
    // clean success, then silently corrupted output, then a hard trap — while
    // `Parser::parse` against the same grammar is solid under heavy repeated use).
    // The bug lives in tree-sitter's own C query compiler (`ts_query__analyze_patterns`
    // / `state_predecessor_map_add`), not in this crate or the tree-sitter-markdown
    // grammar, so it isn't something we can fix here. This manually walks the tree
    // instead, since `Parser::parse` is unaffected. Revert to the `Query`-based
    // version above once the upstream tree-sitter/wasm bug is found and fixed.
    pub fn tags(&self) -> Vec<&'a str> {
        let mut results = vec![];
        collect_tags(self.tree.root_node(), self.text, &mut results);
        results
    }
}

fn collect_tags<'a>(node: tree_sitter::Node, text: &'a str, out: &mut Vec<&'a str>) {
    if node.kind() == "tag" && !has_link_ancestor(node) {
        out.push(node.utf8_text(text.as_bytes()).unwrap());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tags(child, text, out);
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::test_helpers::dump_inline_tree;

    #[test]
    fn markdown_link_hash_is_not_a_tag() {
        // [filename#heading] — the #heading must NOT be extracted as a tag
        let source = "- [ ] Task [filename#heading] and #RealTag\n";
        let doc = Document::parse(source.to_owned(), std::path::PathBuf::new());
        let tags = doc.tags();

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
