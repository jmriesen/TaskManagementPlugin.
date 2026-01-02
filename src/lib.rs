use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::LazyLock;

use serde::Serialize;
use tree_sitter::{Parser, Point, StreamingIterator, Tree};
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{INLINE_LANGUAGE, LANGUAGE as MD};

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
pub struct SearchResult {
    lable: String,
    tags: Vec<String>,
    path: String,
    lnum: usize,
}

pub fn find_next_actions(path: &PathBuf) -> Vec<SearchResult> {
    let mut parser = Parser::new();
    parser
        .set_language(&MD.into())
        .expect("Error loading Markdown inline grammar");
    let mut source = String::new();
    let mut file = File::open(path).expect("error opening file");
    file.read_to_string(&mut source).unwrap();
    let tree = parser.parse(&source, None).unwrap();

    let mut cursor = QueryCursor::new();

    let mut search_results = extract_todos(&source, &tree, &mut cursor).filter_map(
        |InLineTree {
             tree,
             text,
             start_point,
         }| {
            let tags = extract_tags(text, tree.root_node());
            if tags.contains(&"#NextAction") {
                Some(SearchResult {
                    lable: text.to_string(),
                    tags: tags.into_iter().map(ToOwned::to_owned).collect(),
                    path: path
                        .to_str()
                        .expect(
                            "This only fails if you are doing something funky with path encodeings",
                        )
                        .to_owned(),
                    //Rows are 0 indexed lines are 1 index
                    lnum: start_point.row + 1,
                })
            } else {
                None
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

#[test]
fn get_next_actions() {
    let results = find_next_actions(&"test_files/test.md".into());
    assert_eq!(
        results,
        vec![
            SearchResult {
                lable: "Simple task #NextAction".to_string(),
                tags: vec!["#NextAction".to_string()],
                path: "test_files/test.md".to_string(),
                lnum: 2
            },
            SearchResult {
                lable: "Second task #NextAction #SecondTag".to_string(),
                tags: vec!["#NextAction".to_string(), "#SecondTag".to_string()],
                path: "test_files/test.md".to_string(),
                lnum: 4
            }
        ]
    )
}

///Represents a tree for the inline parser
struct InLineTree<'a> {
    tree: Tree,
    text: &'a str,
    //The first line that the tree is on.
    start_point: Point,
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

fn extract_todos<'source, 'cursor, 'tree>(
    source: &'source str,
    tree: &'tree tree_sitter::Tree,
    cursor: &'cursor mut QueryCursor,
) -> impl StreamingIterator<Item = InLineTree<'source>> + use<'source, 'cursor>
where
    'tree: 'cursor,
{
    static SELECT_TODO: LazyLock<Query> = LazyLock::new(|| {
        Query::new(
            &MD.into(),
            "
(list_item
        (task_list_marker_unchecked)
            (paragraph
              ((inline) @inline)
            )
)
",
        )
        .unwrap()
    });
    cursor
        .matches(&SELECT_TODO, tree.root_node(), source.as_bytes())
        .map(move |r#match| parse_inline_tree(source, r#match.captures[0].node))
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
    }
}

fn extract_tags<'a>(text: &'a str, node: tree_sitter::Node<'_>) -> Vec<&'a str> {
    let select_tags = &Query::new(&INLINE_LANGUAGE.into(), "(tag) @text").unwrap();
    let mut cursor = QueryCursor::new();

    let mut matches = cursor.matches(select_tags, node, text.as_bytes());
    let mut results = vec![];

    while let Some(r#match) = matches.next() {
        for cap in r#match.captures {
            //Only one capture in the query so we don't need to check capture name.
            let node = cap.node;
            let text = node.utf8_text(text.as_bytes()).unwrap();
            results.push(text);
        }
    }
    results
}

//TODO: Figure out how to simplify this llm generated code.
//It does work.
pub fn pretty_print_sexpr(input: &str) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    let mut token = String::new();

    let push_token = |out: &mut String, token: &mut String, indent: usize| {
        if !token.trim().is_empty() {
            out.push_str(&"  ".repeat(indent));
            out.push_str(token.trim());
            out.push('\n');
            token.clear();
        }
    };

    for c in input.chars() {
        match c {
            '(' => {
                push_token(&mut out, &mut token, indent);
                out.push_str(&"  ".repeat(indent));
                out.push('(');
                out.push('\n');
                indent += 1;
            }
            ')' => {
                push_token(&mut out, &mut token, indent);
                indent = indent.saturating_sub(1);
                out.push_str(&"  ".repeat(indent));
                out.push(')');
                out.push('\n');
            }
            c if c.is_whitespace() => {
                // finalize token on whitespace
                push_token(&mut out, &mut token, indent);
            }
            _ => token.push(c),
        }
    }

    // flush last token
    if !token.trim().is_empty() {
        out.push_str(&"  ".repeat(indent));
        out.push_str(token.trim());
        out.push('\n');
    }

    out
}
