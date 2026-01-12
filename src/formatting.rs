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
