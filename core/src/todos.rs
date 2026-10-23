//! Collects unchecked task items, tagging each one with its own tags plus
//! those of every enclosing heading and parent list item.

use std::path::Path;

use pulldown_cmark::HeadingLevel;

use crate::markdown::{OutlineEvent, outline};
use crate::search_result::SearchResult;

pub(crate) fn todos(source: &str, path: &Path) -> Vec<SearchResult> {
    // Tags of the headings and items enclosing the current position, outermost first.
    let mut headings: Vec<(HeadingLevel, Vec<&str>)> = vec![];
    let mut parents: Vec<Vec<&str>> = vec![];
    let mut todos = vec![];

    for event in outline(source) {
        match event {
            OutlineEvent::Heading { level, tags, .. } => {
                headings.retain(|(lev, _)| *lev < level);
                headings.push((level, tags));
            }
            OutlineEvent::ItemStart {
                checked,
                text,
                tags,
                lnum,
            } => {
                if checked == Some(false) && !text.is_empty() {
                    let context = headings.iter().map(|(_, t)| t).chain(&parents).flatten();
                    let mut all_tags: Vec<&str> = tags.iter().chain(context).copied().collect();
                    all_tags.sort();
                    all_tags.dedup();
                    todos.push(SearchResult::new(text, &all_tags, lnum, path));
                }
                parents.push(tags);
            }
            OutlineEvent::ItemEnd => {
                parents.pop();
            }
        }
    }
    todos
}
