//! Syntax colouring for the file viewer, from the same grammars the terminal client uses.

use maitty_highlight::Span;
use serde::Serialize;

#[derive(Serialize)]
pub struct HighlightSpan {
    text: String,
    kind: &'static str,
}

fn to_wire(lines: Vec<Vec<Span>>) -> Vec<Vec<HighlightSpan>> {
    lines
        .into_iter()
        .map(|line| {
            line.into_iter()
                .map(|span| HighlightSpan {
                    text: span.text,
                    kind: span.kind.as_str(),
                })
                .collect()
        })
        .collect()
}

/// Split `text` into colour roles per line as the file at `path`; `None` when its type has no grammar
/// or it is too big, and the window then colours it itself. Runs off the async threads: parsing a large file is CPU work.
#[tauri::command]
pub async fn highlight_file(
    path: String,
    text: String,
) -> Result<Option<Vec<Vec<HighlightSpan>>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        maitty_highlight::highlight(&path, &text).map(to_wire)
    })
    .await
    .map_err(|error| format!("highlighting stopped: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_cross_to_the_window_under_the_names_its_css_uses() {
        let lines = maitty_highlight::highlight("a.rs", "fn x() {} // hi\n").unwrap();
        let wire = serde_json::to_value(to_wire(lines)).unwrap();
        let kinds: Vec<&str> = wire[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["kind"].as_str().unwrap())
            .collect();
        assert!(kinds.contains(&"keyword") && kinds.contains(&"comment"), "{kinds:?}");
    }

    #[test]
    fn an_unknown_type_comes_back_as_null() {
        assert!(maitty_highlight::highlight("a.zzzunknown", "x\n").is_none());
    }
}
