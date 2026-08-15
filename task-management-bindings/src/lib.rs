use std::path::PathBuf;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ExcerptDirective {
    file_path: String,
    line_start: usize,
    line_end: usize,
}

#[wasm_bindgen]
impl ExcerptDirective {
    #[wasm_bindgen(getter, js_name = filePath)]
    pub fn file_path(&self) -> String {
        self.file_path.clone()
    }

    #[wasm_bindgen(getter, js_name = lineStart)]
    pub fn line_start(&self) -> usize {
        self.line_start
    }

    #[wasm_bindgen(getter, js_name = lineEnd)]
    pub fn line_end(&self) -> usize {
        self.line_end
    }
}

#[wasm_bindgen(js_name = parseExcerptDirective)]
pub fn parse_excerpt_directive(source: &str) -> Result<ExcerptDirective, JsValue> {
    let mut file_path: Option<String> = None;
    let mut lines_param: Option<String> = None;

    for line in source.trim().lines() {
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_lowercase();
            let value = line[colon_pos + 1..].trim().to_string();
            match key.as_str() {
                "file" => file_path = Some(value),
                "lines" => lines_param = Some(value),
                _ => {}
            }
        }
    }

    let file_path = file_path
        .ok_or_else(|| JsValue::from_str("Missing required 'file' field in excerpt directive"))?;
    let lines_param = lines_param
        .ok_or_else(|| JsValue::from_str("Missing required 'lines' field in excerpt directive"))?;

    let parts: Vec<&str> = lines_param.splitn(2, '-').collect();
    if parts.len() != 2 {
        return Err(JsValue::from_str(
            "Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')",
        ));
    }

    let line_start = parts[0].trim().parse::<usize>().map_err(|_| {
        JsValue::from_str("Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')")
    })?;
    let line_end = parts[1].trim().parse::<usize>().map_err(|_| {
        JsValue::from_str("Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')")
    })?;

    if line_start > line_end {
        return Err(JsValue::from_str(
            "Line start must be less than or equal to line end",
        ));
    }

    Ok(ExcerptDirective {
        file_path,
        line_start,
        line_end,
    })
}

#[wasm_bindgen(js_name = extractContent)]
pub fn extract_content(full_content: &str, line_start: usize, line_end: usize) -> String {
    full_content
        .lines()
        .skip(line_start)
        .take(line_end - line_start)
        .collect::<Vec<_>>()[..]
        .join("\n")
}

#[wasm_bindgen(js_name = temp)]
pub fn test() {
    core::Document::parse("document_contnet".into(), PathBuf::from("./"));
}
