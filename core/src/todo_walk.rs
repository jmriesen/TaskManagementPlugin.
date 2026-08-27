//! Walks a document's tree collecting unchecked task items, tagging each one
//! with its own heading's tags plus every ancestor heading's tags (since
//! `section` nodes nest for sub-headings).

use tree_sitter::Node;

use crate::inline::InLineTree;
use crate::search_result::ToDoEntry;
use crate::tree_util::first_child_of_kind;

/// Walk the tree front to back. Everywhere we find a `section` (a heading
/// plus everything under it), hand it off to `heading_todos`; everywhere
/// else, just keep walking through the children.
pub(crate) fn walk_todos<'a>(node: Node<'a>, source: &'a str, ancestor_tags: &[&'a str]) -> Vec<ToDoEntry<'a>> {
    if node.kind() == "section" {
        return heading_todos(node, source, ancestor_tags);
    }

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .flat_map(|child| walk_todos(child, source, ancestor_tags))
        .collect()
}

/// One heading `section`'s todo entries: its own unchecked task items, tagged
/// with `ancestor_tags` plus this heading's own tags, followed by whatever
/// `walk_todos` finds in the rest of the section (including nested
/// sub-headings, which is why those pick up this heading's tags too).
fn heading_todos<'a>(section: Node<'a>, source: &'a str, ancestor_tags: &[&'a str]) -> Vec<ToDoEntry<'a>> {
    let mut tags = ancestor_tags.to_vec();
    if let Some(heading_inline) =
        first_child_of_kind(section, "atx_heading").and_then(|atx_heading| first_child_of_kind(atx_heading, "inline"))
    {
        tags.extend(InLineTree::parse(source, heading_inline).tags());
    }

    let own_todos = match first_child_of_kind(section, "list") {
        Some(list) => task_list_entries(list, source, &tags),
        None => vec![],
    };

    let mut cursor = section.walk();
    let nested_todos = section
        .children(&mut cursor)
        .flat_map(|child| walk_todos(child, source, &tags));

    own_todos.into_iter().chain(nested_todos).collect()
}

/// Every unchecked task item directly in `list`, tagged with `heading_tags`.
fn task_list_entries<'a>(list: Node, source: &'a str, heading_tags: &[&'a str]) -> Vec<ToDoEntry<'a>> {
    let mut cursor = list.walk();
    list.children(&mut cursor)
        .filter(|item| item.kind() == "list_item")
        .filter(|item| first_child_of_kind(*item, "task_list_marker_unchecked").is_some())
        .filter_map(|item| {
            let paragraph = first_child_of_kind(item, "paragraph")?;
            let task_inline = first_child_of_kind(paragraph, "inline")?;
            let task_tree = InLineTree::parse(source, task_inline);
            Some(ToDoEntry {
                body: task_tree.text,
                heading_tags: heading_tags.to_vec(),
                task_tags: task_tree.tags(),
                lnum: task_tree.start_point.row + 1,
            })
        })
        .collect()
}
