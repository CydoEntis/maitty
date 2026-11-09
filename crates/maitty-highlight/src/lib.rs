//! Syntax highlighting shared by the terminal client and the desktop app.
//!
//! One grammar set, one mapping to a handful of colour roles, so a file reads the same in both.
//! The roles match the GUI's `tok-*` classes; each client chooses its own colours.

use std::path::Path;
use std::sync::OnceLock;

use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Past this many bytes a file stays plain: highlighting is for reading, not for log dumps.
const MAX_BYTES: usize = 512 * 1024;
/// A minified bundle on one line would stall the regex engine, so a longer line stays plain.
const MAX_LINE_CHARS: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Comment,
    String,
    Number,
    Keyword,
    /// Names a reader scans for: variables, tags, headings, types.
    Name,
}

impl Kind {
    /// The role's name, as the desktop app's `tok-*` classes spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Plain => "plain",
            Kind::Comment => "comment",
            Kind::String => "string",
            Kind::Number => "number",
            Kind::Keyword => "keyword",
            Kind::Name => "variable",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub kind: Kind,
}

struct Roles(Vec<(Scope, Kind)>);

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

fn roles() -> &'static Roles {
    static ROLES: OnceLock<Roles> = OnceLock::new();
    ROLES.get_or_init(|| {
        // Order matters: the first match wins, so the narrow scopes come before their parents.
        let table = [
            ("comment", Kind::Comment),
            ("string", Kind::String),
            ("constant.numeric", Kind::Number),
            ("constant.language", Kind::Keyword),
            ("keyword", Kind::Keyword),
            ("storage", Kind::Keyword),
            ("entity.name", Kind::Name),
            ("entity.other.attribute-name", Kind::Name),
            ("support", Kind::Name),
            ("variable", Kind::Name),
            ("markup.heading", Kind::Name),
            ("markup.underline.link", Kind::Name),
        ];
        Roles(
            table
                .iter()
                .filter_map(|(name, kind)| Scope::new(name).ok().map(|s| (s, *kind)))
                .collect(),
        )
    })
}

fn kind_of(stack: &ScopeStack) -> Kind {
    // The innermost scope decides: a string inside a comment is a comment, a number in a string is a string.
    for scope in stack.as_slice().iter().rev() {
        if let Some((_, kind)) = roles().0.iter().find(|(role, _)| role.is_prefix_of(*scope)) {
            return *kind;
        }
    }
    Kind::Plain
}

fn syntax_for(path: &str, first_line: &str) -> Option<&'static SyntaxReference> {
    let set = syntaxes();
    let name = Path::new(path).file_name()?.to_str()?;
    let extension = Path::new(name).extension().and_then(|e| e.to_str());
    set.find_syntax_by_extension(name)
        .or_else(|| extension.and_then(|e| set.find_syntax_by_extension(e)))
        .or_else(|| set.find_syntax_by_first_line(first_line))
}

fn plain(text: &str) -> Vec<Vec<Span>> {
    text.lines()
        .map(|line| {
            vec![Span {
                text: line.to_string(),
                kind: Kind::Plain,
            }]
        })
        .collect()
}

/// Highlight `text` as the file at `path`: one `Vec<Span>` per line, without line endings.
/// Returns `None` when the file type has no grammar or the file is too big, so callers keep
/// their own plain rendering.
pub fn highlight(path: &str, text: &str) -> Option<Vec<Vec<Span>>> {
    if text.len() > MAX_BYTES {
        return None;
    }
    let syntax = syntax_for(path, text.lines().next().unwrap_or(""))?;
    if syntax.name == "Plain Text" {
        return None;
    }
    let set = syntaxes();
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut lines = Vec::new();
    for raw in LinesWithEndings::from(text) {
        let line = raw.trim_end_matches(['\n', '\r']);
        if line.chars().count() > MAX_LINE_CHARS {
            // Skipping the parse would desync multi-line state, so reset it.
            state = ParseState::new(syntax);
            stack = ScopeStack::new();
            lines.push(vec![Span {
                text: line.to_string(),
                kind: Kind::Plain,
            }]);
            continue;
        }
        let Ok(ops) = state.parse_line(raw, set) else {
            return Some(plain(text));
        };
        let mut spans: Vec<Span> = Vec::new();
        let mut cursor = 0;
        let mut push = |from: usize, to: usize, kind: Kind| {
            let to = to.min(line.len());
            if from >= to {
                return;
            }
            match spans.last_mut() {
                Some(last) if last.kind == kind => last.text.push_str(&line[from..to]),
                _ => spans.push(Span {
                    text: line[from..to].to_string(),
                    kind,
                }),
            }
        };
        for (offset, op) in ops {
            push(cursor, offset, kind_of(&stack));
            cursor = cursor.max(offset);
            if stack.apply(&op).is_err() {
                return Some(plain(text));
            }
        }
        push(cursor, line.len(), kind_of(&stack));
        if spans.is_empty() {
            spans.push(Span {
                text: String::new(),
                kind: Kind::Plain,
            });
        }
        lines.push(spans);
    }
    Some(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &[Span]) -> Vec<(&str, Kind)> {
        line.iter().map(|s| (s.text.as_str(), s.kind)).collect()
    }

    #[test]
    fn rust_keywords_strings_and_comments_get_their_roles() {
        let lines = highlight("main.rs", "fn main() { let s = \"hi\"; } // done\n").unwrap();
        let spans = kinds(&lines[0]);
        assert!(spans.contains(&("fn", Kind::Keyword)));
        assert!(spans.contains(&("\"hi\"", Kind::String)));
        assert!(
            spans
                .iter()
                .any(|(t, k)| t.contains("done") && *k == Kind::Comment)
        );
    }

    #[test]
    fn typescript_and_toml_which_plain_syntect_lacks_are_covered() {
        assert!(highlight("app.tsx", "const x: number = 1;\n").is_some());
        assert!(highlight("Cargo.toml", "[package]\nname = \"x\"\n").is_some());
    }

    #[test]
    fn a_block_comment_keeps_its_role_on_every_line() {
        let lines = highlight("a.rs", "/* one\n two\n*/ fn x() {}\n").unwrap();
        assert_eq!(lines[1][0].kind, Kind::Comment);
        assert_eq!(lines[2][0].kind, Kind::Comment);
    }

    #[test]
    fn spans_rebuild_the_original_line() {
        let source = "let a = 'x'; // note\n\tindented\n";
        let lines = highlight("a.js", source).unwrap();
        let rebuilt: Vec<String> = lines
            .iter()
            .map(|l| l.iter().map(|s| s.text.as_str()).collect())
            .collect();
        assert_eq!(rebuilt, ["let a = 'x'; // note", "\tindented"]);
    }

    #[test]
    fn unknown_types_and_huge_files_are_left_to_the_caller() {
        assert!(highlight("notes.zzzunknown", "hello\n").is_none());
        assert!(highlight("big.rs", &"a".repeat(MAX_BYTES + 1)).is_none());
    }

    #[test]
    fn an_overlong_line_stays_plain_without_breaking_the_next() {
        let source = format!("// {}\nfn x() {{}}\n", "y".repeat(MAX_LINE_CHARS + 5));
        let lines = highlight("a.rs", &source).unwrap();
        assert_eq!(lines[0][0].kind, Kind::Plain);
        assert!(kinds(&lines[1]).contains(&("fn", Kind::Keyword)));
    }

    #[test]
    fn dockerfile_is_found_by_its_name() {
        assert!(highlight("Dockerfile", "FROM rust\n").is_some());
    }
}
