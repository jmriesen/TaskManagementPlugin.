pub fn dump_node(node: tree_sitter::Node, src: &[u8]) -> String {
    fn dump_node_inner(node: tree_sitter::Node, src: &[u8], depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        let snippet: String = node.utf8_text(src).unwrap_or("").chars().take(40).collect();
        out.push_str(&format!("{}{} {:?}\n", indent, node.kind(), snippet));
        for i in 0..node.child_count() {
            dump_node_inner(node.child(i as u32).unwrap(), src, depth + 1, out);
        }
    }
    let mut out = String::new();
    dump_node_inner(node, src, 0, &mut out);
    out
}

pub fn dump_inline_tree(text: &str) -> String {
    use tree_sitter_md::INLINE_LANGUAGE;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&INLINE_LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    dump_node(tree.root_node(), text.as_bytes())
}
