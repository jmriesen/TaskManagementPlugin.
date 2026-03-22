use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::LazyLock;

use serde::Serialize;
use tree_sitter::{Point, StreamingIterator, Tree};
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::LANGUAGE as MD;

use crate::block::BlockTree;
use crate::inline::InLineTree;

mod block;
mod formatting;
mod helper;
mod inline;
use helper::StreamingIteratorExt;

///Represents a tree for the inline parser
struct MyTree<'a, T: TreeType> {
    tree: Tree,
    text: &'a str,
    //The first line that the tree is on.
    start_point: Point,
    _phantom: PhantomData<T>,
}
trait TreeType {}

struct ToDoEntry<'a> {
    heading_tags: Vec<&'a str>,
    task_tags: Vec<&'a str>,
    body: &'a str,
    lnum: usize,
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
pub struct SearchResult {
    pub ordinal: String,
    pub display: String,
    pub tags: Vec<String>,
    pub path: String,
    pub lnum: usize,
}

impl SearchResult {
    fn new(
        ToDoEntry {
            heading_tags,
            task_tags,
            body,
            lnum,
        }: &ToDoEntry,
        path: &PathBuf,
    ) -> Self {
        let tags: Vec<_> = task_tags
            .iter()
            .chain(heading_tags)
            .map(|x| x.to_string())
            .collect();

        let mut display = body
            //Only display the first line up to the tags.
            .split(['\n', '#'])
            .next()
            .unwrap_or_default()
            .to_string();

        let tag_string = tags.clone().join(" ");
        display.push_str(&tag_string);

        SearchResult {
            ordinal: display.clone(),
            display,
            tags,
            path: path
                .to_str()
                .expect("This only fails if you are doing something funky with path encodeings")
                .to_owned(),
            lnum: *lnum,
        }
    }
}

pub fn find_todos(path: &PathBuf) -> Vec<SearchResult> {
    let mut source = String::new();
    let mut file = File::open(path).expect("error opening file");
    file.read_to_string(&mut source).unwrap();

    let tree = BlockTree::parse(&source);

    let mut cursor = QueryCursor::new();

    tree.todos(&source, &mut cursor)
        .map(|x| SearchResult::new(x, path))
        .collect()
}

pub fn find_next_actions(path: &PathBuf) -> Vec<SearchResult> {
    find_todos(path)
        .into_iter()
        .filter(|x| x.tags.contains(&"#NextAction".into()))
        .collect()
}

#[test]
fn get_next_actions() {
    let results = find_next_actions(&"test_files/test.md".into());
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
    let results = find_todos(&"test_files/heading_level_tags.md".into());
    dbg!(&results);
    assert_eq!(
        results[0].tags,
        vec!["#NextAction".to_string(), "#HeadingTag".to_string()]
    );
    assert_eq!(results[1].tags, vec!["#HeadingTag".to_string()]);
    assert_eq!(results[2].tags, vec!["#NextAction".to_string(),]);
    assert_eq!(results[3].tags, Vec::<String>::new());
}

fn extract_headlines<'source, 'cursor, 'tree>(
    source: &'source str,
    tree: &'tree tree_sitter::Tree,
    cursor: &'cursor mut QueryCursor,
) -> impl StreamingIterator<Item = InLineTree<'source>> + use<'source, 'cursor>
where
    'tree: 'cursor,
{
    static SELECT_HEADLINES: LazyLock<Query> = LazyLock::new(|| {
        Query::new(
            &MD.into(),
            "[
  (atx_heading
    (_) 
    (inline) @heading.text)

  (setext_heading
    (paragraph (inline) @heading.text)
    (_))
]",
        )
        .unwrap()
    });

    cursor
        .matches(&SELECT_HEADLINES, tree.root_node(), source.as_bytes())
        .map(move |r#match| InLineTree::parse(source, r#match.captures[0].node))
}
