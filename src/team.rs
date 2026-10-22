//! A commander's team: which agent and model a job runs on in a project. Pure: the caller
//! loads the config and finds the project.

use crate::config::{Family, Team, same_path};
use std::path::Path;

/// What a job's helper runs: the agent's name and its model (`None`: the agent's default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Helper {
    pub agent: &'static str,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamError {
    NoTeam,
    UnknownJob { asked: String, known: Vec<String> },
}

impl std::fmt::Display for TeamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamError::NoTeam => write!(f, "no team is set; add jobs in Settings › Team or under [team] in config.toml"),
            TeamError::UnknownJob { asked, known } => write!(f, "no job \"{asked}\" on the team; use one of: {}", known.join(", ")),
        }
    }
}

/// The family `project` (a main checkout) uses: its own override, else the team's.
pub fn active_family(team: &Team, project: &Path) -> Family {
    team.projects.iter().find(|p| same_path(&p.project, project)).map_or(team.family, |p| p.family)
}

/// The agent and model that `job` (any case) runs on in `project`.
pub fn resolve(team: &Team, job: &str, project: &Path) -> Result<Helper, TeamError> {
    if team.jobs.is_empty() {
        return Err(TeamError::NoTeam);
    }
    let Some(found) = team.jobs.iter().find(|j| j.name.eq_ignore_ascii_case(job.trim())) else {
        return Err(TeamError::UnknownJob { asked: job.to_string(), known: team.jobs.iter().map(|j| j.name.clone()).collect() });
    };
    let family = active_family(team, project);
    let model = found.model(family);
    Ok(Helper { agent: family.agent(), model: (!model.is_empty()).then(|| model.to_string()) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TeamJob, TeamProject};

    fn team() -> Team {
        let job = |name: &str, claude: &str, codex: &str| TeamJob { name: name.into(), claude: claude.into(), codex: codex.into() };
        Team {
            family: Family::Claude,
            jobs: vec![job("planner", "opus", "gpt-6.1-sol"), job("builder", "sonnet", "gpt-6-luna"), job("reviewer", "opus", "")],
            projects: vec![TeamProject { project: "/code/app".into(), family: Family::Codex }],
        }
    }

    #[test]
    fn a_project_without_an_override_uses_the_teams_family() {
        assert_eq!(active_family(&team(), Path::new("/code/lib")), Family::Claude);
    }

    #[test]
    fn a_projects_override_wins_however_its_path_is_spelled() {
        assert_eq!(active_family(&team(), Path::new("/code/app/")), Family::Codex);
    }

    #[test]
    fn an_override_for_another_project_does_not_apply() {
        let mut t = team();
        t.family = Family::Codex;
        t.projects[0].family = Family::Claude;
        assert_eq!(active_family(&t, Path::new("/code/lib")), Family::Codex);
    }

    #[test]
    fn a_job_resolves_to_its_model_for_the_active_family_ignoring_case() {
        let claude = resolve(&team(), "Builder", Path::new("/code/lib")).unwrap();
        assert_eq!(claude, Helper { agent: "claude", model: Some("sonnet".into()) });
        let codex = resolve(&team(), "builder", Path::new("/code/app")).unwrap();
        assert_eq!(codex, Helper { agent: "codex", model: Some("gpt-6-luna".into()) });
    }

    #[test]
    fn an_empty_model_is_the_agents_default() {
        assert_eq!(resolve(&team(), "reviewer", Path::new("/code/app")).unwrap().model, None);
    }

    #[test]
    fn a_job_with_no_team_set_says_so() {
        assert_eq!(resolve(&Team::default(), "builder", Path::new("/code/app")), Err(TeamError::NoTeam));
    }

    #[test]
    fn an_unknown_job_lists_the_jobs_there_are() {
        let err = resolve(&team(), "tester", Path::new("/code/app")).unwrap_err();
        let known = vec!["planner".to_string(), "builder".into(), "reviewer".into()];
        assert_eq!(err, TeamError::UnknownJob { asked: "tester".into(), known });
        assert_eq!(err.to_string(), "no job \"tester\" on the team; use one of: planner, builder, reviewer");
    }
}
