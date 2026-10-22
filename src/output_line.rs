//! Reading one line of a check's output: making it safe to show, and telling a failure from
//! a pass. The daemon picks the line a result is summed up by and the client colours the
//! output with, so both must agree on what a failure looks like.

/// What a line of output says about the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Pass,
    Failure,
    Plain,
}

/// A pass is read before a failure: "ok  pkg (error paths)" is a passing line.
pub fn classify(line: &str) -> LineKind {
    let l = line.trim_start();
    let lower = l.to_lowercase();
    if l.starts_with(['✓', '✔']) || l.starts_with("ok ") || l.starts_with("PASS") {
        LineKind::Pass
    } else if l.starts_with(['✗', '✕', '✘']) || l.starts_with("FAIL") || ["error", "failed", "panicked"].iter().any(|w| lower.contains(w)) {
        LineKind::Failure
    } else {
        LineKind::Plain
    }
}

/// One line as text safe to draw: what a command printed is not trusted to move the cursor
/// or restyle the screen. Colour codes go, tabs become spaces, a carriage return keeps
/// what was written last.
pub fn clean(line: &str) -> String {
    let line = line.rsplit('\r').next().unwrap_or_default();
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                if chars.next_if_eq(&'[').is_some() {
                    while chars.next_if(|c| !('@'..='~').contains(c)).is_some() {}
                    chars.next();
                } else if chars.next_if_eq(&']').is_some() {
                    // A title or link: up to the bell, or the ESC that ends it.
                    while chars.next_if(|c| !matches!(c, '\u{7}' | '\u{1b}')).is_some() {}
                    if chars.next() == Some('\u{1b}') {
                        chars.next();
                    }
                }
            }
            '\t' => out.push_str("    "),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}
