use std::collections::BTreeMap;

use pulldown_cmark::HeadingLevel;

use crate::markdown::{OutlineEvent, outline};
use crate::tag_filter::TagFilter;

/// Vault-root file holding the named filters. Frontends read it, and vault
/// scans skip it, since its unchecked items would otherwise show up as todos.
pub const TAG_FILTERS_FILE: &str = "task-filters.md";

/// Parse a shared config file into a `name -> TagFilter` map, so the same
/// named filters can be reused across every frontend (CLI, Obsidian) instead
/// of each one hardcoding its own tag combinations:
///
/// ```markdown
/// # Task filters
///
/// ## Next actions
/// ### Include
/// - [x] #NextAction
/// ### Exclude
/// - [x] #BusinessHours
/// - [ ] #RequiresHouse
///
/// ## Job hunt next actions
/// ### Include
/// - [x] #NextAction
/// ### Any
/// - [x] #JobHunt
/// ```
///
/// Each `##` heading is a filter, and each `###` heading under it (`Include`,
/// `Any` or `Exclude`, case-insensitive) holds a checklist of tags. Only
/// checked items are used, so a tag can be switched on and off by ticking it
/// in Obsidian. Only the first word of an item is used as the tag, and the
/// leading `#` is optional. Anything else (prose, other headings) is ignored.
///
/// Returns a plain `String` error message so callers across the wasm boundary
/// can surface it directly, e.g. in a parse-failure `Notice`.
/// A `BTreeMap` is used (rather than a `HashMap`) purely so callers that list
/// the entries — e.g. to register one command per name — get a stable,
/// alphabetical order.
pub fn parse_tag_filters(source: &str) -> Result<BTreeMap<String, TagFilter>, String> {
    let mut filters: BTreeMap<String, TagFilter> = BTreeMap::new();
    // The `##` filter and `###` list we're currently inside, if any.
    let mut name: Option<&str> = None;
    let mut list: Option<List> = None;

    for event in outline(source) {
        match event {
            OutlineEvent::Heading {
                level: HeadingLevel::H2,
                text,
                ..
            } => {
                list = None;
                name = (!text.is_empty()).then_some(text);
                if let Some(name) = name {
                    if filters.insert(name.to_owned(), TagFilter::default()).is_some() {
                        return Err(format!("Duplicate filter \"{name}\""));
                    }
                }
            }
            OutlineEvent::Heading {
                level: HeadingLevel::H3,
                text,
                ..
            } if name.is_some() => {
                list = Some(List::from_heading(text).ok_or_else(|| {
                    format!(
                        "Filter \"{}\": unknown heading \"{text}\" (expected Include, Any or Exclude)",
                        name.unwrap_or_default()
                    )
                })?);
            }
            OutlineEvent::Heading { level, .. } => {
                if level < HeadingLevel::H2 {
                    name = None;
                }
                list = None;
            }
            OutlineEvent::ItemStart {
                checked: Some(true),
                text,
                ..
            } => {
                if let (Some(name), Some(list), Some(tag)) = (name, list, text.split_whitespace().next()) {
                    let filter = filters.get_mut(name).expect("inserted when its heading was seen");
                    list.of(filter).extend(prefix_tags(vec![tag.to_owned()]));
                }
            }
            OutlineEvent::ItemStart { .. } | OutlineEvent::ItemEnd => {}
        }
    }
    Ok(filters)
}

/// Which of a [`TagFilter`]'s lists a `###` heading fills.
#[derive(Clone, Copy)]
enum List {
    Include,
    Any,
    Exclude,
}

impl List {
    fn from_heading(heading: &str) -> Option<Self> {
        match heading.to_lowercase().as_str() {
            "include" => Some(Self::Include),
            "any" => Some(Self::Any),
            "exclude" => Some(Self::Exclude),
            _ => None,
        }
    }

    fn of(self, filter: &mut TagFilter) -> &mut Vec<String> {
        match self {
            Self::Include => &mut filter.include,
            Self::Any => &mut filter.any,
            Self::Exclude => &mut filter.exclude,
        }
    }
}

/// Add the leading `#` tags are stored with (see [`crate::SearchResult`]) to
/// each of `tags`, unless it's already there.
pub fn prefix_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|t| {
            if t.starts_with('#') {
                t
            } else {
                format!("#{t}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_include_any_exclude() {
        let source = r#"# Task filters

Prose is ignored.

## Next actions
### Include
- [x] #NextAction
### Exclude
- [x] #RequiresIan
- [x] #BusinessHours

## Waiting
### Include
- [x] #Waiting

## Job hunt next actions
### Include
- [x] #NextAction
### Any
- [x] #JobHunt
- [x] #ReAg
"#;
        let filters = parse_tag_filters(source).unwrap();
        assert_eq!(filters.len(), 3);

        let next_actions = &filters["Next actions"];
        assert_eq!(next_actions.include, vec!["#NextAction"]);
        assert_eq!(next_actions.exclude, vec!["#RequiresIan", "#BusinessHours"]);
        assert!(next_actions.any.is_empty());

        let job_hunt = &filters["Job hunt next actions"];
        assert_eq!(job_hunt.any, vec!["#JobHunt", "#ReAg"]);
    }

    #[test]
    fn unchecked_items_are_ignored() {
        let source = r#"
## Next actions
### Include
- [x] #NextAction
- [ ] #Waiting
### Exclude
- [x] #RequiresIan
- [ ] #RequiresHouse
- [X] #BusinessHours
"#;
        let filters = parse_tag_filters(source).unwrap();
        assert_eq!(filters["Next actions"].include, vec!["#NextAction"]);
        assert_eq!(
            filters["Next actions"].exclude,
            vec!["#RequiresIan", "#BusinessHours"]
        );
    }

    #[test]
    fn bare_tags_are_prefixed_and_notes_after_the_tag_are_ignored() {
        let source = r#"
## Waiting
### Include
- [x] Waiting until Friday
"#;
        let filters = parse_tag_filters(source).unwrap();
        assert_eq!(filters["Waiting"].include, vec!["#Waiting"]);
    }

    #[test]
    fn headings_are_case_insensitive() {
        let source = r#"
## Waiting
### include
- [x] #Waiting
### EXCLUDE
- [x] #Done
"#;
        let filters = parse_tag_filters(source).unwrap();
        assert_eq!(filters["Waiting"].include, vec!["#Waiting"]);
        assert_eq!(filters["Waiting"].exclude, vec!["#Done"]);
    }

    #[test]
    fn unknown_sub_heading_is_a_readable_error() {
        let source = r#"
## Waiting
### Inclde
- [x] #Waiting
"#;
        let err = parse_tag_filters(source).unwrap_err();
        assert!(err.contains("Waiting") && err.contains("Inclde"), "{err}");
    }

    #[test]
    fn duplicate_filter_names_are_an_error() {
        let source = r#"
## Waiting
### Include
- [x] #Waiting

## Waiting
### Include
- [x] #Blocked
"#;
        assert!(parse_tag_filters(source).is_err());
    }

    #[test]
    fn entries_are_returned_in_alphabetical_order() {
        let source = r#"
## Zebra
### Include
- [x] Z

## Apple
### Include
- [x] A
"#;
        let filters = parse_tag_filters(source).unwrap();
        let names: Vec<_> = filters.keys().collect();
        assert_eq!(names, vec!["Apple", "Zebra"]);
    }
}
