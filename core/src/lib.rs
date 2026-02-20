use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::LazyLock;

use serde::Serialize;
use tree_sitter::{Parser, Point, StreamingIterator, Tree};
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{INLINE_LANGUAGE, LANGUAGE as MD};

mod formatting;

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
pub struct SearchResult {
    pub ordinal: String,
    pub display: String,
    pub tags: Vec<String>,
    pub path: String,
    pub lnum: usize,
}

pub fn find_todos(path: &PathBuf) -> Vec<SearchResult> {
    let mut parser = Parser::new();
    parser
        .set_language(&MD.into())
        .expect("Error loading Markdown inline grammar");
    let mut source = String::new();
    let mut file = File::open(path).expect("error opening file");
    file.read_to_string(&mut source).unwrap();
    let tree = parser.parse(&source, None).unwrap();

    let mut cursor = QueryCursor::new();

    let mut search_results = extract_todos(&source, &tree, &mut cursor).map(
        |ToDoEntry {
             heading_tags,
             task_tags,
             body,
             lnum,
         }| {
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
        },
    );
    let mut result = vec![];
    while let Some(x) = search_results.next() {
        //TODO it would be kinda neat if I could make this never alloc.
        result.push(x.clone());
    }
    result
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

///Represents a tree for the inline parser
struct MyTree<'a, T: TreeType> {
    tree: Tree,
    text: &'a str,
    //The first line that the tree is on.
    start_point: Point,
    _phantom: PhantomData<T>,
}
trait TreeType {}
struct Inline;
struct Block;
impl TreeType for Inline {}
impl TreeType for Block {}

type InLineTree<'a> = MyTree<'a, Inline>;

impl<'a> InLineTree<'a> {
    pub fn extract_tags(&self) -> Vec<&'a str> {
        let select_tags = &Query::new(&INLINE_LANGUAGE.into(), "(tag) @text").unwrap();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(select_tags, self.tree.root_node(), self.text.as_bytes());
        let mut results = vec![];

        while let Some(r#match) = matches.next() {
            for cap in r#match.captures {
                //Only one capture in the query so we don't need to check capture name.
                let node = cap.node;
                let text = node.utf8_text(self.text.as_bytes()).unwrap();
                results.push(text);
            }
        }
        results
    }
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
        .map(move |r#match| parse_inline_tree(source, r#match.captures[0].node))
}

struct ToDoEntry<'a> {
    heading_tags: Vec<&'a str>,
    task_tags: Vec<&'a str>,
    body: &'a str,
    lnum: usize,
}

fn extract_todos<'source, 'cursor, 'tree>(
    source: &'source str,
    tree: &'tree tree_sitter::Tree,
    cursor: &'cursor mut QueryCursor,
) -> impl StreamingIterator<Item = ToDoEntry<'source>> + use<'source, 'cursor>
where
    'tree: 'cursor,
{
    static SELECT_TODO: LazyLock<Query> = LazyLock::new(|| {
        Query::new(
            &MD.into(),
            "
(section
(atx_heading 
      (_) 
        (inline) @heading.inline)
    (list 
        (list_item
                (task_list_marker_unchecked)
                    (paragraph
                      ((inline) @task.inline)
                    )
        )
    )
)
",
        )
        .unwrap()
    });

    cursor
        .matches(&SELECT_TODO, tree.root_node(), source.as_bytes())
        .map(move |r#match| {
            //Come back and extract this.
            let headline = r#match
                .nodes_for_capture_index(
                    SELECT_TODO
                        .capture_index_for_name("heading.inline")
                        .unwrap(),
                )
                .next()
                .expect("expect exacly one node");
            let task_nodes = r#match
                .nodes_for_capture_index(SELECT_TODO.capture_index_for_name("task.inline").unwrap())
                .next()
                .expect("expect exacly one node");

            let headline_tree = parse_inline_tree(source, headline);
            let task_tree = parse_inline_tree(source, task_nodes);
            SELECT_TODO.capture_names()[r#match.captures[0].index as usize];

            ToDoEntry {
                body: task_tree.text,
                heading_tags: headline_tree.extract_tags(),
                task_tags: task_tree.extract_tags(),
                lnum: task_tree.start_point.row + 1,
            }
        })
}

fn parse_inline_tree<'a>(source: &'a str, node: tree_sitter::Node<'_>) -> InLineTree<'a> {
    let text = node.utf8_text(source.as_bytes()).unwrap();
    let start_point = node.start_position();

    let mut inline_parser = Parser::new();
    inline_parser
        .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
        .unwrap();
    let tree = inline_parser.parse(text, None).unwrap();

    InLineTree {
        tree,
        text,
        start_point,
        _phantom: PhantomData::default(),
    }
}
