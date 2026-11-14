//! The specialist agent files of a pane's project and its user's home, listed read-only for
//! `Query::Specialists` (ADR 0010). Claude Code keeps them as Markdown with a front matter
//! (`.claude/agents/*.md`), Codex as TOML (`.codex/agents/*.toml`). Nothing here writes a file.

use crate::hidden::sanitize;
use crate::protocol::TermId;
use crate::specialists::{
    SPECIALIST_FIELD_MAX, SPECIALIST_TEXT_MAX, SPECIALISTS_MAX, SpecialistFile, SpecialistList,
    SpecialistScope, SpecialistTool,
};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Where each tool keeps its agent files under a project or home folder, and their extension.
const CLAUDE_AGENTS: (&str, &str) = (".claude/agents", "md");
const CODEX_AGENTS: (&str, &str) = (".codex/agents", "toml");

/// Lists the files. `codex_home` is Codex's own folder (`$CODEX_HOME`, else `~/.codex`), which
/// holds its home agents. A folder that is missing or unreadable has no files.
pub fn list(
    term: TermId,
    cwd: &Path,
    home: Option<&Path>,
    codex_home: Option<&Path>,
) -> SpecialistList {
    let project = project_root(cwd);
    let mut places = vec![
        (
            SpecialistTool::Claude,
            SpecialistScope::Project,
            project.join(CLAUDE_AGENTS.0),
            CLAUDE_AGENTS.1,
        ),
        (
            SpecialistTool::Codex,
            SpecialistScope::Project,
            project.join(CODEX_AGENTS.0),
            CODEX_AGENTS.1,
        ),
    ];
    if let Some(home) = home {
        places.push((
            SpecialistTool::Claude,
            SpecialistScope::Home,
            home.join(CLAUDE_AGENTS.0),
            CLAUDE_AGENTS.1,
        ));
    }
    if let Some(codex) = codex_home {
        places.push((
            SpecialistTool::Codex,
            SpecialistScope::Home,
            codex.join("agents"),
            CODEX_AGENTS.1,
        ));
    }
    let mut files = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut truncated = false;
    for (tool, scope, dir, extension) in places {
        // A project that is the home folder would list the same files twice.
        if seen.contains(&dir) {
            continue;
        }
        seen.push(dir.clone());
        for path in agent_files(&dir, extension) {
            if files.len() == SPECIALISTS_MAX {
                truncated = true;
                break;
            }
            if let Some(file) = read_one(&path, tool, scope) {
                files.push(file);
            }
        }
    }
    SpecialistList {
        term,
        project: project.display().to_string(),
        files,
        truncated,
    }
}

/// The repository `cwd` is in (the nearest folder holding `.git`), else `cwd` itself.
fn project_root(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|dir| dir.join(".git").exists())
        .unwrap_or(cwd)
        .to_path_buf()
}

/// The regular files in `dir` with this extension, by name.
fn agent_files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|e| e.to_str()) == Some(extension) && path.is_file()
        })
        .collect();
    found.sort();
    found
}

fn read_one(path: &Path, tool: SpecialistTool, scope: SpecialistScope) -> Option<SpecialistFile> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(SPECIALIST_TEXT_MAX as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    let text_cut = bytes.len() > SPECIALIST_TEXT_MAX;
    bytes.truncate(SPECIALIST_TEXT_MAX);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (name, description) = match tool {
        SpecialistTool::Codex => codex_fields(&text),
        _ => claude_fields(&text),
    };
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("agent");
    Some(SpecialistFile {
        tool,
        scope,
        name: name.and_then(|n| field(&n)).unwrap_or_else(|| clip(stem)),
        description: description.and_then(|d| field(&d)),
        path: path.display().to_string(),
        text: sanitize(&text, true),
        text_cut,
    })
}

/// `name` and `description` from a Claude agent file's front matter: the lines between a first
/// line `---` and the next. A value may be quoted, or a `|` / `>` block of indented lines.
fn claude_fields(text: &str) -> (Option<String>, Option<String>) {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return (None, None);
    }
    let head: Vec<&str> = lines.take_while(|line| line.trim_end() != "---").collect();
    let value = |key: &str| -> Option<String> {
        let at = head.iter().position(|line| {
            line.strip_prefix(key)
                .is_some_and(|rest| rest.starts_with(':'))
        })?;
        let raw = head[at][key.len() + 1..].trim();
        if raw.starts_with('|') || raw.starts_with('>') {
            let block: Vec<&str> = head[at + 1..]
                .iter()
                .take_while(|line| {
                    line.starts_with(' ') || line.starts_with('\t') || line.is_empty()
                })
                .map(|line| line.trim())
                .collect();
            return Some(block.join(" "));
        }
        Some(unquote(raw).to_string())
    };
    (value("name"), value("description"))
}

fn unquote(raw: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = raw.strip_prefix(quote).and_then(|r| r.strip_suffix(quote)) {
            return inner;
        }
    }
    raw
}

/// `name` and `description` from a Codex agent file. A file that is not TOML has neither.
fn codex_fields(text: &str) -> (Option<String>, Option<String>) {
    let Ok(table) = text.parse::<toml::Table>() else {
        return (None, None);
    };
    let get = |key: &str| {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    };
    (get("name"), get("description"))
}

/// A name or description made safe to draw and bounded; blank is none.
fn field(text: &str) -> Option<String> {
    let clean = sanitize(text, false).trim().to_string();
    (!clean.is_empty()).then(|| clip(&clean))
}

fn clip(text: &str) -> String {
    text.chars().take(SPECIALIST_FIELD_MAX).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn project_and_home_files_of_both_tools_are_listed_project_first() {
        let tmp = super::super::persist::scratch_dir();
        let (project, home) = (tmp.path().join("app"), tmp.path().join("home"));
        std::fs::create_dir_all(project.join(".git")).unwrap();
        write(
            &project.join(".claude/agents/reviewer.md"),
            "---\nname: reviewer\ndescription: \"Reviews a change\"\ntools: Read\n---\nYou review.\n",
        );
        write(
            &project.join(".codex/agents/builder.toml"),
            "name = \"builder\"\ndescription = \"Builds it\"\ndeveloper_instructions = '''\nBuild.\n'''\n",
        );
        write(
            &home.join(".claude/agents/writer.md"),
            "---\nname: writer\ndescription: |\n  Writes\n  docs.\n---\nWrite.\n",
        );
        write(&home.join(".codex/agents/notes.txt"), "not an agent");
        let listed = list(
            7,
            &project.join("src"),
            Some(&home),
            Some(&home.join(".codex")),
        );
        let seen: Vec<_> = listed
            .files
            .iter()
            .map(|f| (f.tool, f.scope, f.name.as_str(), f.description.as_deref()))
            .collect();
        assert_eq!(
            seen,
            vec![
                (
                    SpecialistTool::Claude,
                    SpecialistScope::Project,
                    "reviewer",
                    Some("Reviews a change")
                ),
                (
                    SpecialistTool::Codex,
                    SpecialistScope::Project,
                    "builder",
                    Some("Builds it")
                ),
                (
                    SpecialistTool::Claude,
                    SpecialistScope::Home,
                    "writer",
                    Some("Writes docs.")
                ),
            ]
        );
        assert_eq!(
            listed.project,
            project.display().to_string(),
            "the repository, not the pane's subfolder"
        );
        assert!(listed.files[1].text.contains("Build."));
        assert!(!listed.truncated);
    }

    #[test]
    fn a_file_without_a_name_is_called_by_its_file_name_and_a_long_one_is_cut() {
        let tmp = super::super::persist::scratch_dir();
        let long = format!(
            "---\ndescription: x\n---\n{}",
            "a".repeat(SPECIALIST_TEXT_MAX + 10)
        );
        write(&tmp.path().join(".claude/agents/plain.md"), &long);
        write(&tmp.path().join(".codex/agents/broken.toml"), "name = ");
        let listed = list(1, tmp.path(), None, None);
        assert_eq!(listed.files[0].name, "plain");
        assert!(listed.files[0].text_cut);
        assert_eq!(listed.files[0].text.len(), SPECIALIST_TEXT_MAX);
        assert_eq!(
            (
                listed.files[1].name.as_str(),
                listed.files[1].description.as_deref()
            ),
            ("broken", None)
        );
    }

    #[test]
    fn a_home_that_is_the_project_lists_its_files_once() {
        let tmp = super::super::persist::scratch_dir();
        write(
            &tmp.path().join(".claude/agents/a.md"),
            "---\nname: a\n---\n",
        );
        assert_eq!(list(1, tmp.path(), Some(tmp.path()), None).files.len(), 1);
    }

    #[test]
    fn hidden_and_control_characters_never_reach_a_client() {
        let tmp = super::super::persist::scratch_dir();
        write(
            &tmp.path().join(".claude/agents/a.md"),
            "---\nname: a\u{1b}[31mred\ndescription: b\u{200b}c\n---\n\u{1b}]0;x\u{7}body\n",
        );
        let file = &list(1, tmp.path(), None, None).files[0];
        assert_eq!(
            (file.name.as_str(), file.description.as_deref()),
            ("a[31mred", Some("bc"))
        );
        assert!(!file.text.contains('\u{1b}'));
    }
}
