pub mod document;
mod helper;
mod inline;
#[cfg(test)]
mod test_helpers;

pub use document::{combine_requires, filter_todos, Document, HeadingMismatch, SearchResult, TagFilter};

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::{filter_todos, Document, HeadingMismatch, SearchResult, TagFilter};

    #[test]
    fn get_next_actions() {
        let results = Document::from_file(Path::new("test_files/test.md")).next_actions();
        assert_eq!(
            results,
            vec![
                SearchResult {
                    display: "Simple task #NextAction".to_string(),
                    ordinal: "Simple task #NextAction".to_string(),
                    tags: vec!["#NextAction".to_string()],
                    path: "test_files/test.md".to_string(),
                    lnum: 2,
                },
                SearchResult {
                    display: "Second task #NextAction #SecondTag".to_string(),
                    ordinal: "Second task #NextAction #SecondTag".to_string(),
                    tags: vec!["#NextAction".to_string(), "#SecondTag".to_string()],
                    path: "test_files/test.md".to_string(),
                    lnum: 4
                }
            ]
        )
    }

    #[test]
    fn get_heading_level_tags() {
        let results = Document::from_file(Path::new("test_files/heading_level_tags.md")).todos();
        dbg!(&results);
        assert_eq!(
            results[0].tags,
            vec!["#NextAction".to_string(), "#HeadingTag".to_string()]
        );
        assert_eq!(results[1].tags, vec!["#HeadingTag".to_string()]);
        assert_eq!(results[2].tags, vec!["#NextAction".to_string(),]);
        assert_eq!(results[3].tags, Vec::<String>::new());
    }

    #[test]
    fn get_requires() {
        let results = Document::from_file(Path::new("test_files/requires_test.md")).requires();
        let mut alice = results["Alice"].clone();
        let mut bob = results["Bob"].clone();
        alice.sort_by_key(|x| x.lnum);
        bob.sort_by_key(|x| x.lnum);
        assert_eq!(
            alice,
            vec![
                SearchResult {
                    ordinal: "Follow up with Alice #RequiresAlice".to_string(),
                    display: "Follow up with Alice #RequiresAlice".to_string(),
                    tags: vec!["#RequiresAlice".to_string()],
                    path: "test_files/requires_test.md".to_string(),
                    lnum: 2,
                },
                SearchResult {
                    ordinal: "Another Alice item #RequiresAlice #NextAction".to_string(),
                    display: "Another Alice item #RequiresAlice #NextAction".to_string(),
                    tags: vec!["#RequiresAlice".to_string(), "#NextAction".to_string()],
                    path: "test_files/requires_test.md".to_string(),
                    lnum: 4,
                },
            ]
        );
        assert_eq!(
            bob,
            vec![SearchResult {
                ordinal: "Review doc #RequiresBob".to_string(),
                display: "Review doc #RequiresBob".to_string(),
                tags: vec!["#RequiresBob".to_string()],
                path: "test_files/requires_test.md".to_string(),
                lnum: 3,
            }]
        );
    }

    #[test]
    fn heading_mismatch_detected() {
        // filename is "heading_mismatch_test", heading is "Wrong Heading" — should mismatch
        let result =
            Document::from_file(Path::new("test_files/heading_mismatch_test.md")).heading_mismatch();
        assert_eq!(
            result,
            Some(HeadingMismatch {
                path: "test_files/heading_mismatch_test.md".to_string(),
                filename: "heading_mismatch_test".to_string(),
                heading: Some("Wrong Heading".to_string()),
            })
        );
    }

    #[test]
    fn heading_match_returns_none() {
        let result =
            Document::from_file(Path::new("test_files/heading_mismatch_test.md")).heading_mismatch();
        assert!(result.is_some());
    }

    #[test]
    fn tag_filter_include_requires_all() {
        let filter = TagFilter {
            include: vec!["#NextAction".into(), "#SecondTag".into()],
            ..Default::default()
        };
        assert!(!filter.matches(&["#NextAction".to_string()]));
        assert!(filter.matches(&["#NextAction".to_string(), "#SecondTag".to_string()]));
    }

    #[test]
    fn tag_filter_any_requires_at_least_one() {
        let filter = TagFilter {
            any: vec!["#Waiting".into(), "#Blocked".into()],
            ..Default::default()
        };
        assert!(!filter.matches(&["#NextAction".to_string()]));
        assert!(filter.matches(&["#Waiting".to_string()]));
    }

    #[test]
    fn tag_filter_exclude_rejects_any_match() {
        let filter = TagFilter {
            exclude: vec!["#Done".into()],
            ..Default::default()
        };
        assert!(filter.matches(&["#NextAction".to_string()]));
        assert!(!filter.matches(&["#NextAction".to_string(), "#Done".to_string()]));
    }

    #[test]
    fn filter_todos_combines_all_three_rules() {
        let results = Document::from_file(Path::new("test_files/test.md")).todos();
        let filter = TagFilter {
            include: vec!["#NextAction".into()],
            any: vec![],
            exclude: vec!["#SecondTag".into()],
        };
        let filtered = filter_todos(results, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].display, "Simple task #NextAction");
    }
}
