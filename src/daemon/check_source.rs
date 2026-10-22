//! Which checks a session's project has, where each comes from, and whether it may run.
//! For now only the checks you named in config; a repo's own file joins them with approval.

use super::approvals::Approvals;
use crate::config::{CheckDef, same_path};
use crate::protocol::CheckSource;
use std::path::Path;

/// A checkpoint ready to run, or held until you decide.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub def: CheckDef,
    pub source: CheckSource,
    pub gate: Gate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    Run,
    /// A repo's command nobody approved; `previous` is the one you said Always to, if the repo
    /// changed it since.
    NeedsApproval { previous: Option<String> },
    /// You chose never to run it.
    Never,
}

/// The checks for `project` (a repo's main checkout) as run in `worktree`, in order. Blocks:
/// call it only from a background task.
pub fn resolve(user: &[CheckDef], project: &Path, _worktree: &Path, _approvals: &Approvals) -> Vec<Resolved> {
    user.iter()
        .filter(|c| same_path(&c.project, project))
        .map(|def| Resolved { def: def.clone(), source: CheckSource::User, gate: Gate::Run })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn your_checks_for_the_project_run_in_order_without_approval() {
        let def = |project: &str, command: &str| CheckDef { project: project.into(), command: command.into(), ..Default::default() };
        let user = [def("/code/site", "npm run build"), def("/code/other", "make"), def("/code/site/", "npm test")];
        let got = resolve(&user, Path::new("/code/site"), Path::new("/code/site-worktrees/fix"), &Approvals::default());
        let got: Vec<_> = got.iter().map(|r| (r.def.name(), &r.source, &r.gate)).collect();
        assert_eq!(got, [("build", &CheckSource::User, &Gate::Run), ("test", &CheckSource::User, &Gate::Run)]);
        assert!(resolve(&user, Path::new("/code/none"), Path::new("/code/none"), &Approvals::default()).is_empty());
    }
}
