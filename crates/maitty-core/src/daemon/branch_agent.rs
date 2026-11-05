//! "New agent on a branch": the checks and the command line for a `NewWorktree` that names an
//! agent and a first prompt. The worktree itself comes from the usual machinery in
//! `worktrees.rs` (naming, hooks, trust, clean-up); this only decides what may start in it.

use super::*;

/// Whether the project at `dir` (its repository's main checkout) is the one `known` folders belong
/// to, the way the sidebar groups them: by repository, a worktree counting as its repo.
fn same_project(dir: &std::path::Path, known: &std::path::Path) -> bool {
    match (repo_of(dir), repo_of(known)) {
        (Some(a), Some(b)) => same_path(&a, &b),
        _ => same_path(dir, known),
    }
}

impl Daemon {
    /// Whether `dir` is a project the app already shows: a folder or repository some workspace or
    /// pane is in. A client cannot ask for a branch in a repository the daemon has never been
    /// shown, so this cannot be used to make worktrees (and run hooks) anywhere on disk.
    pub(super) fn is_known_project(&self, dir: &std::path::Path) -> bool {
        self.workspaces.iter().any(|w| same_project(dir, &w.cwd))
            || self.terms.values().any(|t| same_project(dir, &t.cwd))
    }
}

/// The most characters of a first prompt. A prompt is a command-line argument, and a shell or the
/// OS refuses a line past a few tens of kilobytes, so the limit keeps a start from failing late.
pub const MAX_PROMPT_CHARS: usize = 8_000;

/// The longest branch name git itself would accept in a path on every platform we build for.
const MAX_BRANCH_CHARS: usize = 200;

/// Why `name` is not a branch to make, or `None` when it will do: letters, digits, `_`, `.`, `/`
/// and `-` only (it ends up in a folder name), and git's own rules for a ref name.
pub fn branch_problem(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("name the branch".into());
    }
    if name.chars().count() > MAX_BRANCH_CHARS {
        return Some(format!(
            "a branch name is at most {MAX_BRANCH_CHARS} characters"
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
    {
        return Some("a branch name may hold only letters, digits, _ . / and -".into());
    }
    if name.starts_with('-') {
        return Some("a branch name cannot start with a dash".into());
    }
    if name.starts_with('/') || name.ends_with('/') || name.contains("//") {
        return Some("a branch name cannot start or end with a slash, or hold //".into());
    }
    if name.contains("..") || name == "@" {
        return Some("a branch name cannot hold .. or be @".into());
    }
    if name.ends_with('.') {
        return Some("a branch name cannot end with a dot".into());
    }
    // `@{` cannot occur (the alphabet above has no braces); a component that starts with a dot or
    // ends in `.lock` is a ref name git refuses.
    if name
        .split('/')
        .any(|part| part.starts_with('.') || part.ends_with(".lock"))
    {
        return Some(
            "a branch name cannot hold a part that starts with a dot or ends in .lock".into(),
        );
    }
    None
}

/// Why `prompt` cannot be handed to an agent, or `None`. A prompt that starts with a dash would be
/// read as an option by the agent's own command line (`--dangerously-skip-permissions`), and a
/// control character other than a line break or tab has no business in a prompt.
fn prompt_problem(prompt: &str) -> Option<String> {
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Some(format!(
            "the first prompt is at most {MAX_PROMPT_CHARS} characters"
        ));
    }
    if prompt.starts_with('-') {
        return Some("the first prompt cannot start with a dash; begin with a word".into());
    }
    if prompt
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Some("the first prompt holds a control character".into());
    }
    None
}

/// The command line that starts `agent` (a preset or built-in agent by name; the configured
/// default when `None`) with `prompt` as its first prompt, quoted for the shell the panes run.
/// An empty prompt starts the agent bare. Errors say what to change.
pub fn start_line(
    cfg: &Config,
    agent: Option<&str>,
    prompt: Option<&str>,
) -> Result<String, String> {
    let prompt = prompt.map(str::trim).unwrap_or_default();
    if let Some(problem) = prompt_problem(prompt) {
        return Err(problem);
    }
    let agent = agent.map_or_else(|| cfg.default_agent(), String::from);
    if prompt.is_empty() {
        // `agent_command_quoted` swaps the prompt into the preset's template; a bare start
        // leaves the slot empty.
        return cfg
            .agent_command_quoted(&agent, None, "")
            .map(|line| line.trim().to_string());
    }
    cfg.agent_command(Some(&agent), None, prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_branch_name_follows_the_alphabet_and_git_s_rules() {
        for ok in ["agent/fix-login", "feat/a_b.c", "x", "release-1.2"] {
            assert_eq!(branch_problem(ok), None, "{ok}");
        }
        for bad in [
            "",
            "-x",
            "/x",
            "x/",
            "a//b",
            "a..b",
            "a b",
            "a~b",
            "a:b",
            "a?b",
            "a\\b",
            "a@{b",
            "x.",
            "x.lock",
            "a/.hidden",
            "a/b.lock/c",
            "@",
            "naïve",
            "a\nb",
        ] {
            assert!(branch_problem(bad).is_some(), "{bad:?} should be refused");
        }
        assert!(branch_problem(&"a".repeat(MAX_BRANCH_CHARS + 1)).is_some());
    }

    #[test]
    fn a_prompt_is_bounded_and_cannot_pose_as_an_option() {
        assert_eq!(prompt_problem("fix the login bug\nthen test it"), None);
        assert_eq!(prompt_problem(&"x".repeat(MAX_PROMPT_CHARS)), None);
        assert!(prompt_problem(&"x".repeat(MAX_PROMPT_CHARS + 1)).is_some());
        assert!(prompt_problem("--dangerously-skip-permissions").is_some());
        assert!(prompt_problem("a\u{0}b").is_some());
        assert!(prompt_problem("a\u{1b}[31mb").is_some());
    }

    #[test]
    fn the_start_line_quotes_the_prompt_and_refuses_an_unknown_agent() {
        let cfg = Config::default();
        let line = start_line(&cfg, Some("claude"), Some("it's $(rm -rf ~) `x` \"y\"")).unwrap();
        assert!(line.starts_with("claude "), "{line}");
        // Quoted as one argument for the configured shell, so nothing in it is interpreted.
        assert_eq!(
            line,
            format!(
                "claude {}",
                cfg.quote_for_shell("it's $(rm -rf ~) `x` \"y\"")
            )
        );
        assert_eq!(start_line(&cfg, Some("codex"), None).unwrap(), "codex");
        assert_eq!(
            start_line(&cfg, Some("claude"), Some("  ")).unwrap(),
            "claude"
        );
        assert!(start_line(&cfg, Some("rm"), Some("x")).is_err());
        assert!(start_line(&cfg, Some("claude; rm -rf ~"), None).is_err());
        assert!(start_line(&cfg, Some("claude"), Some("-p")).is_err());
    }
}
