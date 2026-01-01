use tree_sitter::{Parser, StreamingIterator, Tree};
use tree_sitter::{Query, QueryCursor};
use tree_sitter_md::{INLINE_LANGUAGE, LANGUAGE as MD};

fn main() {
    let mut parser = Parser::new();
    parser
        .set_language(&MD.into())
        .expect("Error loading Markdown inline grammar");
    let source = std::fs::read_to_string("test.md").unwrap();
    //println!("{source}");
    //println!("-------------");
    let tree = parser.parse(&source, None).unwrap();
    println!("{}", pretty_print_sexpr(&tree.root_node().to_sexp()));
    //println!("-------------");
    let headlines = extract_headlines(&source, &tree);
    println!("headlines");
    for (tree, text) in headlines {
        let tags = extract_tags(text, tree.root_node());
        println!("----------");
        println!("Headlines:{}", text);

        //println!("{}", pretty_print_sexpr(&tree.root_node().to_sexp()));
        println!("---");
        println!("Tags:{}", tags.join(","));
    }
    println!("");
    println!("todos");
    for (tree, text) in extract_todos(&source, &tree) {
        let tags = extract_tags(text, tree.root_node());
        println!("----------");
        println!("todos:{}", text);
        println!("---");
        println!("Tags:{}", tags.join(","));
    }
}

//TODO I am tempted to wrapp the tree type so I can't pass the wrong one.
fn extract_headlines<'a>(source: &'a str, tree: &tree_sitter::Tree) -> Vec<(Tree, &'a str)> {
    let select_headlines = Query::new(
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
    .unwrap();

    let mut cursor = QueryCursor::new();
    let mut headings = cursor.matches(&select_headlines, tree.root_node(), source.as_bytes());
    let mut results = vec![];
    while let Some(m) = headings.next() {
        for cap in m.captures {
            if select_headlines.capture_names()[cap.index as usize] == "heading.text" {
                let node = cap.node;
                let text = node.utf8_text(source.as_bytes()).unwrap();
                let mut inline_parser = Parser::new();
                inline_parser
                    .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
                    .unwrap();
                let inline_tree = inline_parser.parse(text, None).unwrap();
                results.push((inline_tree, text));
            }
        }
    }
    results
}
fn extract_todos<'a>(source: &'a str, tree: &tree_sitter::Tree) -> Vec<(Tree, &'a str)> {
    let select_headlines = Query::new(
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
    .unwrap();
    /*
     */

    let mut cursor = QueryCursor::new();
    let mut headings = cursor.matches(&select_headlines, tree.root_node(), source.as_bytes());
    let mut results = vec![];
    while let Some(m) = headings.next() {
        for cap in m.captures {
            if select_headlines.capture_names()[cap.index as usize] == "inline" {
                let node = cap.node;
                let text = node.utf8_text(source.as_bytes()).unwrap();
                let mut inline_parser = Parser::new();
                inline_parser
                    .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
                    .unwrap();
                let inline_tree = inline_parser.parse(text, None).unwrap();
                results.push((inline_tree, text));
            }
        }
    }
    results
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
