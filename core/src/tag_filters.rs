use std::collections::BTreeMap;

use crate::document::TagFilter;

/// Parse a shared config file into a `name -> TagFilter` map, so the same
/// named filters can be reused across every frontend (CLI, Obsidian) instead
/// of each one hardcoding its own tag combinations. Mirrors the shape of the
/// Lua table the nvim plugin previously kept locally:
///
/// ```toml
/// [nextActions]
/// include = ["NextAction"]
/// exclude = ["RequiresPersonName", "BusinessHours"]
///
/// [waiting]
/// include = ["Waiting"]
///
/// [jobHuntNextActions]
/// include = ["NextAction"]
/// any = ["JobHunt" ]
/// ```
///
/// Tag names in the file are bare (no leading `#`); this adds the `#` prefix
/// to match how tags are stored on [`crate::SearchResult`].
///
/// Returns a plain `String` error message (rather than leaking the `toml`
/// crate's error type through this crate's public API) so callers across the
/// wasm boundary can surface it directly, e.g. in a parse-failure `Notice`.
/// A `BTreeMap` is used (rather than a `HashMap`) purely so callers that list
/// the entries — e.g. to register one command per name — get a stable,
/// alphabetical order.
pub fn parse_tag_filters(source: &str) -> Result<BTreeMap<String, TagFilter>, String> {
    let mut filters: BTreeMap<String, TagFilter> =
        toml::from_str(source).map_err(|e| e.to_string())?;

    for filter in filters.values_mut() {
        filter.include = prefix_tags(std::mem::take(&mut filter.include));
        filter.any = prefix_tags(std::mem::take(&mut filter.any));
        filter.exclude = prefix_tags(std::mem::take(&mut filter.exclude));
    }

    Ok(filters)
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
        let source = r#"
[nextActions]
include = ["NextAction"]
exclude = ["RequiresIan", "BusinessHours"]

[waiting]
include = ["Waiting"]

[jobHuntNextActions]
include = ["NextAction"]
any = ["JobHunt", "ReAg"]
"#;
        let filters = parse_tag_filters(source).unwrap();
        assert_eq!(filters.len(), 3);

        let next_actions = &filters["nextActions"];
        assert_eq!(next_actions.include, vec!["#NextAction"]);
        assert_eq!(next_actions.exclude, vec!["#RequiresIan", "#BusinessHours"]);
        assert!(next_actions.any.is_empty());

        let job_hunt = &filters["jobHuntNextActions"];
        assert_eq!(job_hunt.any, vec!["#JobHunt", "#ReAg"]);
    }

    #[test]
    fn allows_comments() {
        let source = r#"
[nextActions]
include = ["NextAction"]
exclude = [
  "RequiresIan",
  # "RequiresHouse", # temporarily ignored
  "BusinessHours",
]
"#;
        let filters = parse_tag_filters(source).unwrap();
        assert_eq!(
            filters["nextActions"].exclude,
            vec!["#RequiresIan", "#BusinessHours"]
        );
    }

    #[test]
    fn already_prefixed_tags_are_left_alone() {
        let source = r##"
[waiting]
include = ["#Waiting"]
"##;
        let filters = parse_tag_filters(source).unwrap();
        assert_eq!(filters["waiting"].include, vec!["#Waiting"]);
    }

    #[test]
    fn invalid_toml_is_a_readable_error() {
        let err = parse_tag_filters("not valid toml [[[").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn entries_are_returned_in_alphabetical_order() {
        let source = r#"
[zebra]
include = ["Z"]

[apple]
include = ["A"]
"#;
        let filters = parse_tag_filters(source).unwrap();
        let names: Vec<_> = filters.keys().collect();
        assert_eq!(names, vec!["apple", "zebra"]);
    }
}
