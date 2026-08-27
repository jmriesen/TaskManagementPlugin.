//! Generic tree-sitter node helpers shared across the document/todo-walking code.

use tree_sitter::Node;

use crate::inline::InLineTree;

/// Collect every descendant node (including `node` itself) whose `kind()` matches, in document order.
pub(crate) fn collect_nodes_of_kind<'a>(node: Node<'a>, kind: &str, out: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nodes_of_kind(child, kind, out);
    }
}

pub(crate) fn first_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

/// Depth-first search for the first `atx_heading` that has both an
/// `atx_h1_marker` and an `inline` child, mirroring the original SELECT_H1 query.
pub(crate) fn find_first_h1<'a>(node: Node, source: &'a str) -> Option<InLineTree<'a>> {
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
