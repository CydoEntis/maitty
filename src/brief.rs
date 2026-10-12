//! `seshi brief`: what a commander does. An agent is made a commander by being told to run this
//! and follow it, so the instructions are the same however it was started and never have
//! to survive a shell's quoting.

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

pub fn print() {
    print!("{BRIEF}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// The words after `seshi ` wherever the brief shows a command (in backticks).
    fn commands_named() -> Vec<String> {
        let mut out: Vec<String> = BRIEF.split('`').skip(1).step_by(2).filter_map(|code| code.strip_prefix("seshi ")).filter_map(|rest| rest.split_whitespace().next()).map(String::from).collect();
        out.sort();
        out.dedup();
        out
    }

    fn tools_named() -> Vec<String> {
        let mut out: Vec<String> = BRIEF.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|w| w.starts_with("seshi_")).map(String::from).collect();
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

    #[test]
    fn the_brief_covers_a_commanders_job_in_order() {
        let parts = ["## Split the goal", "## Start helpers", "## Wait and read", "## Prompts and questions", "## Check the work", "## Report back", "## When the repo has a plan"];
        let at: Vec<usize> = parts.iter().map(|p| BRIEF.find(p).unwrap_or_else(|| panic!("missing: {p}"))).collect();
        assert!(at.windows(2).all(|w| w[0] < w[1]), "in the order a commander does them");
        for must in ["its own worktree", "permission prompt", "active phase", "One ticket per helper", "blocked by"] {
            assert!(BRIEF.contains(must), "the brief says: {must}");
        }
    }

    #[test]
    fn the_brief_only_names_commands_and_tools_that_exist() {
        let cli = crate::Args::command();
        let subs: Vec<&str> = cli.get_subcommands().map(|c| c.get_name()).collect();
        let commands = commands_named();
        assert!(commands.len() >= 5, "it shows the shell commands: {commands:?}");
        for c in &commands {
            assert!(subs.contains(&c.as_str()), "`seshi {c}` is not a command");
        }
        let tools = crate::mcp::tool_names();
        let named = tools_named();
        assert!(named.len() >= 5, "it names the tools: {named:?}");
        for t in &named {
            assert!(tools.contains(t), "{t} is not a tool");
        }
    }
}
