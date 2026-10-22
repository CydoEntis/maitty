//! `seshi brief`: what a commander does. An agent is made a commander by being told to run this
//! and follow it, so the instructions are the same however it was started and never have
//! to survive a shell's quoting.

use crate::config::{Config, Family, TeamJob, same_path};
use crate::team;
use std::path::Path;

pub const BRIEF: &str = include_str!("brief.md");

/// What an agent is told to make it a commander: where its instructions are, and the goal if
/// there is one yet. Its own words carry no quotes or backticks for a shell to trip on.
pub fn commander_prompt(goal: &str) -> String {
    let brief = "You are the commander of a fleet of agents in seshi. Run the shell command seshi brief and follow what it prints.";
    match goal.trim() {
        "" => format!("{brief} Then ask me for the goal."),
        goal => format!("{brief} The goal: {goal}"),
    }
}

/// Print the brief for the caller's project. A config that does not load still gives the plain
/// brief, with the reason on stderr.
pub fn print() {
    match Config::load() {
        Ok(cfg) => print!("{}", compose(&cfg, &crate::mcp::caller_root())),
        Err(e) => {
            eprintln!("seshi: config not loaded, so no team in the brief: {e:#}");
            print!("{BRIEF}");
        }
    }
}

/// Where the team section goes: right after `## Start helpers`.
const TEAM_BEFORE: &str = "## Wait and read";

/// The brief for `project` (a main checkout): `BRIEF` as it is, plus the team when one is set.
pub fn compose(cfg: &Config, project: &Path) -> String {
    if cfg.team.jobs.is_empty() {
        return BRIEF.to_string();
    }
    let at = BRIEF.find(TEAM_BEFORE).expect("the brief has a Wait and read section");
    format!("{}{}{}", &BRIEF[..at], team_section(cfg, project), &BRIEF[at..])
}

fn team_section(cfg: &Config, project: &Path) -> String {
    let family = team::active_family(&cfg.team, project);
    let whose = if family == cfg.team.family { "the team's setting" } else { "this project's own setting" };
    let mut out = format!(
        "## Your team\n\nThe person has set a team: a job for each kind of helper, each on its own model. Here they run on {} ({whose}).\n\n",
        family.agent()
    );
    for job in &cfg.team.jobs {
        out.push_str(&job_line(cfg, family, job));
    }
    out.push_str(
        "\n- Start a job's helper with `seshi_start` and its `job` (for example `job: \"builder\"`), or with its\n  shell line above.\n- Use these jobs and their models. Don't choose other agents or models for them.\n",
    );
    let checked = cfg.check.iter().any(|c| same_path(&c.project, project));
    out.push_str(if checked { HANDOFF_AFTER_CHECK } else { HANDOFF_WITHOUT_CHECK });
    out.push('\n');
    out
}

const HANDOFF_AFTER_CHECK: &str = "\
- Work moves on to the next job only after its check has passed. Wait for the helper with
  `seshi_wait`, or `seshi wait --pane <n>`, and start the next job's helper on that work (for
  example the reviewer after the builder) only when the result says the check passed.
- A failed check, or one that could not run, goes back to the helper that did the work, with
  what failed. It does not go on to the next job.
- A check the person overrode (`check overridden`) counts as passed only because they
  overrode it. Say so in your report.
";

const HANDOFF_WITHOUT_CHECK: &str = "\
- This project has no check. Before you hand work on to the next job, read the diff and run
  the project's tests yourself. Suggest the person set a check in Settings › Check.
";

fn job_line(cfg: &Config, family: Family, job: &TeamJob) -> String {
    let agent = family.agent();
    let model = Some(job.model(family)).filter(|m| !m.is_empty());
    let named = model.map_or_else(|| "the agent's default model".to_string(), |m| format!("model {m}"));
    let shell = match cfg.agent_command_quoted(agent, model, "\"<the task>\"") {
        Ok(cmd) => format!("`seshi worktree <branch> -- {cmd}`"),
        Err(e) => e,
    };
    format!("- {}: {agent}, {named}. {shell}\n", job.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// The words after `seshi ` wherever the brief shows a command (in backticks).
    fn commands_named(brief: &str) -> Vec<String> {
        let mut out: Vec<String> = brief.split('`').skip(1).step_by(2).filter_map(|code| code.strip_prefix("seshi ")).filter_map(|rest| rest.split_whitespace().next()).map(String::from).collect();
        out.sort();
        out.dedup();
        out
    }

    fn tools_named(brief: &str) -> Vec<String> {
        let mut out: Vec<String> = brief.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|w| w.starts_with("seshi_")).map(String::from).collect();
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn a_commander_is_pointed_at_the_brief_and_given_the_goal() {
        let p = commander_prompt("  ship the login page  ");
        assert!(p.contains("seshi brief") && p.ends_with("The goal: ship the login page"), "{p}");
        assert!(!commander_prompt("").contains("The goal") && commander_prompt(" ").ends_with("ask me for the goal."));
        let own = commander_prompt("");
        assert!(!own.contains(['\'', '"', '`', '$', '\n']), "nothing in its own words needs quoting: {own}");
    }

    fn team_cfg(family: &str) -> Config {
        let mut cfg: Config = toml::from_str(&format!(
            "[team]\nfamily = '{family}'\n[[team.jobs]]\nname = 'planner'\nclaude = 'opus'\ncodex = 'gpt-6.1-sol'\n[[team.jobs]]\nname = 'builder'\nclaude = 'sonnet'\ncodex = 'gpt-6-luna'\n[[team.jobs]]\nname = 'reviewer'\nclaude = 'opus'\ncodex = ''\n"
        ))
        .unwrap();
        cfg.shell = Some("bash".into());
        cfg
    }

    fn with_check(mut cfg: Config, project: &str) -> Config {
        cfg.check.push(crate::config::CheckDef { project: project.into(), command: "npm test".into(), ..Default::default() });
        cfg
    }

    /// The plain brief and ones composed with the user's three jobs, without and with a check.
    fn briefs() -> [String; 3] {
        let app = Path::new("/code/app");
        [BRIEF.to_string(), compose(&team_cfg("claude"), app), compose(&with_check(team_cfg("claude"), "/code/app"), app)]
    }

    #[test]
    fn with_a_check_work_moves_to_the_next_job_only_after_it_passes() {
        let brief = compose(&with_check(team_cfg("claude"), "/code/app/"), Path::new("/code/app"));
        for says in [
            "only after its check has passed",
            "`seshi_wait`, or `seshi wait --pane <n>`",
            "the reviewer after the builder) only when the result says the check passed",
            "A failed check, or one that could not run, goes back to the helper that did the work, with\n  what failed. It does not go on to the next job.",
            "counts as passed only because they\n  overrode it. Say so in your report.",
        ] {
            assert!(brief.contains(says), "the brief says {says:?}:\n{brief}");
        }
        assert!(!brief.contains("has no check"));
    }

    #[test]
    fn without_a_check_the_commander_checks_the_work_itself_and_suggests_one() {
        let brief = compose(&with_check(team_cfg("claude"), "/code/other"), Path::new("/code/app"));
        assert!(brief.contains("This project has no check."), "another project's check is not this one's:\n{brief}");
        assert!(brief.contains("read the diff and run\n  the project's tests yourself") && brief.contains("Settings › Check"), "{brief}");
        assert!(!brief.contains("only after its check has passed"));
    }

    #[test]
    fn with_a_check_but_no_team_the_brief_is_unchanged() {
        assert_eq!(compose(&with_check(Config::default(), "/code/app"), Path::new("/code/app")), BRIEF);
    }

    #[test]
    fn the_brief_covers_a_commanders_job_in_order() {
        let parts = ["## Split the goal", "## Start helpers", "## Wait and read", "## Prompts and questions", "## Check the work", "## Report back", "## When the repo has a plan"];
        for brief in briefs() {
            let at: Vec<usize> = parts.iter().map(|p| brief.find(p).unwrap_or_else(|| panic!("missing: {p}"))).collect();
            assert!(at.windows(2).all(|w| w[0] < w[1]), "in the order a commander does them");
            for must in ["its own worktree", "permission prompt", "active phase", "One ticket per helper", "blocked by"] {
                assert!(brief.contains(must), "the brief says: {must}");
            }
        }
    }

    #[test]
    fn the_brief_only_names_commands_and_tools_that_exist() {
        let cli = crate::Args::command();
        let subs: Vec<&str> = cli.get_subcommands().map(|c| c.get_name()).collect();
        let tools = crate::mcp::tool_names();
        for brief in briefs() {
            let commands = commands_named(&brief);
            assert!(commands.len() >= 5, "it shows the shell commands: {commands:?}");
            for c in &commands {
                assert!(subs.contains(&c.as_str()), "`seshi {c}` is not a command");
            }
            let named = tools_named(&brief);
            assert!(named.len() >= 5, "it names the tools: {named:?}");
            for t in &named {
                assert!(tools.contains(t), "{t} is not a tool");
            }
        }
    }

    #[test]
    fn with_no_team_the_brief_is_unchanged() {
        assert_eq!(compose(&Config::default(), Path::new("/code/app")), BRIEF);
    }

    #[test]
    fn the_team_comes_right_after_starting_helpers_one_line_per_job_in_order() {
        let brief = compose(&team_cfg("claude"), Path::new("/code/app"));
        let team = brief.find("## Your team").expect("a team section");
        assert!(brief.find("## Start helpers").unwrap() < team && team < brief.find("## Wait and read").unwrap());
        assert!(brief.contains("Here they run on claude (the team's setting)."), "{brief}");
        let jobs: Vec<&str> = brief.lines().filter(|l| ["- planner:", "- builder:", "- reviewer:"].iter().any(|p| l.starts_with(p))).collect();
        assert_eq!(jobs, [
            "- planner: claude, model opus. `seshi worktree <branch> -- claude --model opus \"<the task>\"`",
            "- builder: claude, model sonnet. `seshi worktree <branch> -- claude --model sonnet \"<the task>\"`",
            "- reviewer: claude, model opus. `seshi worktree <branch> -- claude --model opus \"<the task>\"`",
        ]);
    }

    #[test]
    fn a_projects_codex_override_shows_its_models_and_says_where_it_comes_from() {
        let mut cfg = team_cfg("claude");
        cfg.team.projects.push(crate::config::TeamProject { project: "/code/app".into(), family: Family::Codex });
        let brief = compose(&cfg, Path::new("/code/app"));
        assert!(brief.contains("Here they run on codex (this project's own setting)."), "{brief}");
        assert!(brief.contains("- builder: codex, model gpt-6-luna. `seshi worktree <branch> -- codex -m gpt-6-luna \"<the task>\"`"), "{brief}");
        assert!(brief.contains("- reviewer: codex, the agent's default model. `seshi worktree <branch> -- codex \"<the task>\"`"), "{brief}");
        assert!(compose(&cfg, Path::new("/code/lib")).contains("run on claude (the team's setting)"), "another project keeps the team's family");
    }

    #[test]
    fn the_team_section_says_how_to_start_a_job_and_to_keep_its_model() {
        let brief = compose(&team_cfg("claude"), Path::new("/code/app"));
        assert!(brief.contains("`seshi_start` and its `job`"), "{brief}");
        assert!(brief.contains("Don't choose other agents or models"), "{brief}");
    }
}
