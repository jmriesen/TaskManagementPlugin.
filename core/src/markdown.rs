//! The one place that talks to pulldown-cmark. Wraps its event stream in an
//! [`Outline`] adapter that yields just the headings and list items the rest
//! of the crate works with, and finds Obsidian `#tags`, which pulldown-cmark
//! doesn't know about.

use std::ops::Range;

use pulldown_cmark::{
    DefaultBrokenLinkCallback, Event, HeadingLevel, OffsetIter, Options, Parser, Tag, TagEnd,
    TextMergeWithOffset,
};

/// A heading or list item, as yielded by [`Outline`]. Like pulldown-cmark's
/// own events, nesting is shown by matching start and end events: every
/// `ItemStart` has an `ItemEnd`, with any sub-items' events between them.
#[derive(Debug, PartialEq)]
pub(crate) enum OutlineEvent<'a> {
    Heading {
        level: HeadingLevel,
        text: &'a str,
        tags: Vec<&'a str>,
    },
    /// Yielded once the item's first paragraph is complete, so it comes
    /// before any of its sub-items.
    ItemStart {
        /// `Some(checked)` for a task item (`- [ ]` / `- [x]`), `None` for a plain bullet.
        checked: Option<bool>,
        /// The item's first paragraph, as written in the source.
        text: &'a str,
        tags: Vec<&'a str>,
        /// 1-based line the item starts on.
        lnum: usize,
    },
    ItemEnd,
}

/// The headings and list items in `source`, read lazily, so a caller that
/// stops early (e.g. at the first H1) doesn't parse the rest.
pub(crate) fn outline(source: &str) -> Outline<'_> {
    Outline {
        source,
        events: events(source),
        scanner: TagScanner::default(),
        lines: LineCounter::default(),
        open: vec![],
        pending: None,
    }
}

/// Every tag in `source`, in document order, duplicates included.
pub(crate) fn tags(source: &str) -> Vec<&str> {
    let mut scanner = TagScanner::default();
    events(source)
        .filter_map(|(event, range)| scanner.taggable(&event, range))
        .flat_map(|range| tags_in(source, range))
        .collect()
}

type Events<'a> = TextMergeWithOffset<'a, OffsetIter<'a, DefaultBrokenLinkCallback>>;

fn events(source: &str) -> Events<'_> {
    // Front matter, tables and strikethrough are enabled so their syntax
    // isn't misread as headings or plain text.
    let options = Options::ENABLE_TASKLISTS
        | Options::ENABLE_WIKILINKS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH;
    // Merging keeps text split by pulldown-cmark (e.g. at `_`) in one piece,
    // so a tag like `#Re_Ag` isn't cut in half.
    TextMergeWithOffset::new(Parser::new_ext(source, options).into_offset_iter())
}

/// Iterator adapter from pulldown-cmark events to [`OutlineEvent`]s. See [`outline`].
pub(crate) struct Outline<'a> {
    source: &'a str,
    events: Events<'a>,
    scanner: TagScanner,
    lines: LineCounter,
    /// Headings and items that have started but not ended, innermost last.
    open: Vec<Open<'a>>,
    /// An `ItemEnd` owed straight after an `ItemStart` that was only
    /// yielded when its item ended.
    pending: Option<OutlineEvent<'a>>,
}

/// A heading or item that has started but not ended.
struct Open<'a> {
    heading: Option<HeadingLevel>,
    start: usize,
    checked: Option<bool>,
    /// Source range of the first paragraph read so far.
    span: Option<Range<usize>>,
    tags: Vec<&'a str>,
    /// Whether its `ItemStart` has been yielded (after which its text is complete).
    yielded: bool,
}

impl<'a> Iterator for Outline<'a> {
    type Item = OutlineEvent<'a>;

    fn next(&mut self) -> Option<OutlineEvent<'a>> {
        if let Some(event) = self.pending.take() {
            return Some(event);
        }
        while let Some((event, range)) = self.events.next() {
            let taggable = self.scanner.taggable(&event, range.clone());
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    self.open.push(Open::new(Some(level), range.start))
                }
                Event::Start(Tag::Item) => self.open.push(Open::new(None, range.start)),
                Event::TaskListMarker(checked) => {
                    if let Some(open) = self.open.last_mut() {
                        open.checked = Some(checked);
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    let mut heading = self
                        .open
                        .pop()
                        .expect("pulldown-cmark balances start and end events");
                    return Some(self.yield_open(&mut heading));
                }
                Event::End(TagEnd::Item) => {
                    let mut item = self
                        .open
                        .pop()
                        .expect("pulldown-cmark balances start and end events");
                    if item.yielded {
                        return Some(OutlineEvent::ItemEnd);
                    } else {
                        self.pending = Some(OutlineEvent::ItemEnd);
                        return Some(self.yield_open(&mut item));
                    }
                }
                // The end of an item's first paragraph: its text is complete.
                Event::End(TagEnd::Paragraph)
                | Event::Start(
                    Tag::List(_)
                    | Tag::CodeBlock(_)
                    | Tag::BlockQuote(_)
                    | Tag::HtmlBlock
                    | Tag::Table(_),
                ) => {
                    if let Some(mut item) = self.open.pop_if(|o| o.heading.is_none() && !o.yielded)
                    {
                        let event = self.yield_open(&mut item);
                        self.open.push(item);
                        return Some(event);
                    }
                }
                event if is_inline(&event) => {
                    if let Some(open) = self.open.last_mut().filter(|o| !o.yielded) {
                        let span = open.span.get_or_insert(range.clone());
                        // A link's start event spans the whole link, so the text inside it ends earlier.
                        span.end = span.end.max(range.end);
                        open.tags.extend(taggable.into_iter().flat_map(|r| tags_in(self.source, r)));
                    }
                }
                _ => {}
            }
        }
        None
    }
}

impl<'a> Outline<'a> {
    fn yield_open(&mut self, open: &mut Open<'a>) -> OutlineEvent<'a> {
        open.yielded = true;
        let text = open
            .span
            .clone()
            .map_or("", |span| self.source[span].trim());
        let tags = std::mem::take(&mut open.tags);
        match open.heading {
            Some(level) => OutlineEvent::Heading { level, text, tags },
            None => OutlineEvent::ItemStart {
                checked: open.checked,
                text,
                tags,
                lnum: self.lines.line_at(
                    self.source,
                    open.span.as_ref().map_or(open.start, |s| s.start),
                ),
            },
        }
    }
}

impl Open<'_> {
    fn new(heading: Option<HeadingLevel>, start: usize) -> Self {
        Self {
            heading,
            start,
            checked: None,
            span: None,
            tags: vec![],
            yielded: false,
        }
    }
}

fn is_inline(event: &Event) -> bool {
    matches!(
        event,
        Event::Text(_)
            | Event::Code(_)
            | Event::InlineMath(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Start(
                Tag::Emphasis
                    | Tag::Strong
                    | Tag::Strikethrough
                    | Tag::Superscript
                    | Tag::Subscript
                    | Tag::Link { .. }
                    | Tag::Image { .. }
            )
    )
}

/// Turns byte offsets into 1-based line numbers. Items are yielded in
/// document order, so this only ever counts forward from the last offset.
#[derive(Default)]
struct LineCounter {
    offset: usize,
    newlines: usize,
}

impl LineCounter {
    fn line_at(&mut self, source: &str, offset: usize) -> usize {
        if offset < self.offset {
            *self = Self::default();
        }
        self.newlines += source[self.offset..offset].matches('\n').count();
        self.offset = offset;
        self.newlines + 1
    }
}

/// Finds tags in a stream of events, skipping text where a `#` isn't a tag:
/// link text (e.g. `[[file#heading]]`), code blocks and front matter. Inline
/// code arrives as its own `Event::Code`, so it never looks like text here.
#[derive(Default)]
struct TagScanner {
    link_depth: usize,
    in_literal: bool,
}

impl TagScanner {
    /// Update the scanner's state for `event`, and if it's text whose tags
    /// count, return its source `range`.
    ///
    /// Returns the range rather than the event's own text so tags can borrow
    /// from the source: merged or unescaped text is an owned `CowStr` that
    /// only lives as long as the event.
    fn taggable(&mut self, event: &Event, range: Range<usize>) -> Option<Range<usize>> {
        match event {
            Event::Start(Tag::Link { .. }) => self.link_depth += 1,
            Event::End(TagEnd::Link) => self.link_depth -= 1,
            Event::Start(Tag::CodeBlock(_) | Tag::MetadataBlock(_)) => self.in_literal = true,
            Event::End(TagEnd::CodeBlock | TagEnd::MetadataBlock(_)) => self.in_literal = false,
            Event::Text(_) if self.link_depth == 0 && !self.in_literal => return Some(range),
            _ => {}
        }
        None
    }
}

/// Every `#tag` in `range` of `source`: a `#` followed by a run of letters,
/// digits, `_`, `-` or `/` that isn't all digits (so `#42` isn't a tag).
/// Equivalent to the old tree-sitter grammar's
/// `#[0-9]*[a-zA-Z_\-\/][a-zA-Z_\-\/0-9]*`, which also matched mid-word,
/// e.g. the tag in `done.#Waiting`.
///
/// This reads raw source, so a `#` straight after `\` (an escape) or `&` (an
/// entity like `&#x23;`) is skipped. That check looks at the whole source,
/// since an escaped `#`'s text range starts after its backslash.
fn tags_in(source: &str, range: Range<usize>) -> impl Iterator<Item = &str> {
    let is_tag_char = |c: char| c.is_ascii_alphanumeric() || "_-/".contains(c);
    let mut pos = range.start;
    std::iter::from_fn(move || {
        while let Some(found) = source[pos..range.end].find('#') {
            let hash = pos + found;
            let escaped = source[..hash].ends_with(['\\', '&']);
            let len = source[hash + 1..range.end]
                .find(|c| !is_tag_char(c))
                .unwrap_or(range.end - hash - 1);
            pos = hash + 1 + len;
            let tag = &source[hash..pos];
            if !escaped && tag[1..].chars().any(|c| !c.is_ascii_digit()) {
                return Some(tag);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use OutlineEvent::*;

    fn tags_in_vec(text: &str) -> Vec<&str> {
        tags_in(text, 0..text.len()).collect()
    }

    #[test]
    fn tags_follow_the_grammar_pattern() {
        assert_eq!(
            tags_in_vec("a #One and #Re_Ag #a/b-c"),
            vec!["#One", "#Re_Ag", "#a/b-c"]
        );
        assert_eq!(tags_in_vec("glued.#Waiting"), vec!["#Waiting"]);
        assert_eq!(tags_in_vec("#H1tag."), vec!["#H1tag"]);
        assert_eq!(tags_in_vec("issue #42 and #30m"), vec!["#30m"]);
        assert!(tags_in_vec("# lone ## hashes").is_empty());
    }

    #[test]
    fn escaped_hashes_and_entities_are_not_tags() {
        assert_eq!(tags(r"\#Escaped &#x23;Entity &#35; #Real"), vec!["#Real"]);
    }

    #[test]
    fn tags_in_links_and_code_are_ignored() {
        let source = "- [ ] Task [[filename#heading]] [l](file#anchor) `#code` and #RealTag\n\n```\n#InCode\n```\n";
        assert_eq!(tags(source), vec!["#RealTag"]);
    }

    #[test]
    fn front_matter_is_not_a_heading_or_tags() {
        let source = "---\ntitle: x #NotATag\n---\n# Real\n";
        assert!(tags(source).is_empty());
        assert_eq!(
            outline(source).collect::<Vec<_>>(),
            vec![Heading {
                level: HeadingLevel::H1,
                text: "Real",
                tags: vec![]
            }]
        );
    }

    #[test]
    fn items_nest_between_start_and_end() {
        let source = "# H #T\n- [ ] Parent #P\n  continued\n    - [x] Child\n- Plain\n";
        assert_eq!(
            outline(source).collect::<Vec<_>>(),
            vec![
                Heading {
                    level: HeadingLevel::H1,
                    text: "H #T",
                    tags: vec!["#T"]
                },
                ItemStart {
                    checked: Some(false),
                    text: "Parent #P\n  continued",
                    tags: vec!["#P"],
                    lnum: 2
                },
                ItemStart {
                    checked: Some(true),
                    text: "Child",
                    tags: vec![],
                    lnum: 4
                },
                ItemEnd,
                ItemEnd,
                ItemStart {
                    checked: None,
                    text: "Plain",
                    tags: vec![],
                    lnum: 5
                },
                ItemEnd,
            ]
        );
    }

    #[test]
    fn only_an_items_first_paragraph_is_its_text() {
        let source = "- [ ] First #A\n\n  Second #B\n";
        let items: Vec<_> = outline(source)
            .filter_map(|e| match e {
                ItemStart { text, tags, .. } => Some((text, tags)),
                _ => None,
            })
            .collect();
        assert_eq!(items, vec![("First #A", vec!["#A"])]);
    }
}
