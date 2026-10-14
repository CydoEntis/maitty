//! Portable roles for agent launches and native workflow definitions.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const OWNED: &str = "Managed by Seshi agent profiles.";
const MAX_INSTRUCTIONS: usize = 16_384;
pub const MAX_TASK: usize = 8_192;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runner {
    #[default]
    Claude,
    Codex,
}

impl Runner {
    pub fn command(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub agent: Runner,
    pub model: String,
    pub effort: String,
    pub instructions: String,
    pub skills: Vec<String>,
}

impl Profile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            identifier(&self.name),
            "profile name must start with a letter and use letters, digits, - or _ (max 64 characters)"
        );
        ensure!(
            self.model.len() <= 128 && !self.model.chars().any(char::is_control),
            "invalid model name"
        );
        ensure!(
            matches!(
                self.effort.as_str(),
                "" | "low" | "medium" | "high" | "xhigh" | "max"
            ),
            "effort must be default, low, medium, high, xhigh or max"
        );
        ensure!(
            self.instructions.len() <= MAX_INSTRUCTIONS && !self.instructions.contains('\0'),
            "instructions must be at most {MAX_INSTRUCTIONS} bytes and contain no NUL"
        );
        ensure!(
            self.skills.len() <= 32,
            "a profile may request at most 32 skills"
        );
        for name in &self.skills {
            ensure!(
                !name.is_empty()
                    && name.len() <= 128
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_:./".contains(c)),
                "invalid skill name: {name}"
            );
        }
        Ok(())
    }

    pub fn instructions_for_runner(&self) -> String {
        let mut text = self.instructions.clone();
        if !self.skills.is_empty() {
            text.push_str(&skill_request(&self.skills));
        }
        text
    }

    /// Arguments are passed through a shell only after each one has been quoted.
    pub fn args(&self, task: &str) -> Result<Vec<String>> {
        self.validate()?;
        ensure!(
            task.len() <= MAX_TASK && !task.contains('\0'),
            "task must be at most {MAX_TASK} bytes and contain no NUL"
        );
        let mut args = Vec::new();
        match self.agent {
            Runner::Claude => {
                let mut definition = serde_json::json!({
                    "description": format!("The {} role", self.name),
                    "prompt": self.instructions_for_runner(),
                    "skills": self.skills,
                });
                if !self.model.is_empty() {
                    definition["model"] = self.model.clone().into();
                }
                if !self.effort.is_empty() {
                    definition["effort"] = self.effort.clone().into();
                }
                args.extend([
                    "--agents".into(),
                    serde_json::json!({self.name.clone(): definition}).to_string(),
                    "--agent".into(),
                    self.name.clone(),
                ]);
                // Session flags also apply when the definition is used as the main agent.
                if !self.model.is_empty() {
                    args.extend(["--model".into(), self.model.clone()]);
                }
                if !self.effort.is_empty() {
                    args.extend(["--effort".into(), self.effort.clone()]);
                }
            }
            Runner::Codex => {
                if !self.model.is_empty() {
                    args.extend(["--model".into(), self.model.clone()]);
                }
                if !self.effort.is_empty() {
                    args.extend([
                        "-c".into(),
                        format!("model_reasoning_effort={}", toml_string(&self.effort)),
                    ]);
                }
                let instructions = self.instructions_for_runner();
                if !instructions.is_empty() {
                    args.extend([
                        "-c".into(),
                        format!("developer_instructions={}", toml_string(&instructions)),
                    ]);
                }
            }
        }
        if !task.trim().is_empty() {
            args.extend(["--".into(), task.into()]);
        }
        Ok(args)
    }

    pub fn summary(&self) -> String {
        format!(
            "{} · {} · {}",
            self.agent.command(),
            if self.model.is_empty() {
                "default model"
            } else {
                &self.model
            },
            if self.effort.is_empty() {
                "default effort"
            } else {
                &self.effort
            }
        )
    }
}

fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn toml_string(text: &str) -> String {
    toml::Value::String(text.into()).to_string()
}

fn skill_request(skills: &[String]) -> String {
    format!(
        "\n\nUse these installed skills when carrying out this role: {}. If any requested skill is unavailable, report it before doing the task. Project instructions still apply.",
        skills.join(", ")
    )
}

pub fn validate(profiles: &[Profile]) -> Result<()> {
    let mut names = std::collections::HashSet::new();
    for profile in profiles {
        profile.validate()?;
        ensure!(
            names.insert(&profile.name),
            "duplicate profile: {}",
            profile.name
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub profile: Profile,
    pub project: bool,
}

pub fn resolve(global: &[Profile], project: &[Profile]) -> Result<Vec<Resolved>> {
    validate(global)?;
    validate(project)?;
    let mut result: Vec<_> = global
        .iter()
        .cloned()
        .map(|profile| Resolved {
            profile,
            project: false,
        })
        .collect();
    for profile in project {
        let resolved = Resolved {
            profile: profile.clone(),
            project: true,
        };
        match result.iter().position(|p| p.profile.name == profile.name) {
            Some(i) => result[i] = resolved,
            None => result.push(resolved),
        }
    }
    Ok(result)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ProjectProfiles {
    profiles: Vec<Profile>,
}

/// A worktree's own config wins; otherwise use the main checkout's config.
pub fn project_profiles(dir: &Path) -> Result<Vec<Profile>> {
    let main = crate::gitfs::head(dir).map(|h| h.main_root);
    for root in std::iter::once(dir.to_path_buf()).chain(main) {
        let path = root.join(crate::project::FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let project: ProjectProfiles = toml::from_str(&text)
                    .with_context(|| format!("parsing profiles in {}", path.display()))?;
                validate(&project.profiles)?;
                return Ok(project.profiles);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }
    Ok(Vec::new())
}

pub fn load(cfg: &crate::config::Config, dir: &Path) -> Result<Vec<Resolved>> {
    resolve(&cfg.profiles, &project_profiles(dir)?)
}

/// Change only the selected profile; preserve other settings and TOML comments.
pub fn save(path: &Path, original: Option<&str>, profile: &Profile) -> Result<()> {
    profile.validate()?;
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context("reading profile settings"),
    };
    let mut doc: toml_edit::DocumentMut = text.parse().context("parsing profile settings")?;
    let current: ProjectProfiles = toml::from_str(&text).context("parsing existing profiles")?;
    validate(&current.profiles)?;
    ensure!(
        !current
            .profiles
            .iter()
            .any(|p| p.name == profile.name && Some(p.name.as_str()) != original),
        "profile {} already exists in this scope",
        profile.name
    );
    if doc.get("profiles").is_none() {
        doc["profiles"] = toml_edit::Item::ArrayOfTables(Default::default());
    }
    let tables = doc["profiles"]
        .as_array_of_tables_mut()
        .context("profiles must be [[profiles]] tables")?;
    let index = original.and_then(|name| {
        tables
            .iter()
            .position(|t| t.get("name").and_then(toml_edit::Item::as_str) == Some(name))
    });
    let mut table = toml_edit::Table::new();
    for (key, value) in [
        ("name", profile.name.as_str()),
        ("agent", profile.agent.command()),
        ("model", &profile.model),
        ("effort", &profile.effort),
        ("instructions", &profile.instructions),
    ] {
        table[key] = toml_edit::value(value);
    }
    let mut skills = toml_edit::Array::new();
    for skill in &profile.skills {
        skills.push(skill.as_str());
    }
    table["skills"] = toml_edit::value(skills);
    match index {
        Some(i) => *tables.get_mut(i).context("profile changed while saving")? = table,
        None => tables.push(table),
    }
    crate::config::write_atomic(path, doc.to_string().as_bytes()).context("saving profile settings")
}

pub fn starter(runner: Runner, role: usize) -> Profile {
    let (name, instructions) = match role {
        0 => (
            "planner",
            "Investigate the request and existing code. Produce a scoped plan with acceptance criteria and verification steps. Ask about unresolved requirements. Do not implement the plan.",
        ),
        1 => (
            "worker",
            "Implement the approved plan. Follow project instructions, keep the change focused, run the required verification, and report actual results and limitations.",
        ),
        _ => (
            "reviewer",
            "Review the requirements, plan, actual changes, surrounding code, and verification evidence independently. Look for correctness, security, regressions and missing tests. Report actionable findings with file references. Do not edit code. Do not rely on the worker's claims as proof.",
        ),
    };
    Profile {
        name: format!("{}-{name}", runner.command()),
        agent: runner,
        model: if runner == Runner::Claude {
            if role == 1 { "sonnet" } else { "opus" }.into()
        } else {
            String::new()
        },
        effort: "medium".into(),
        instructions: instructions.into(),
        skills: Vec::new(),
    }
}

pub fn native_text(profile: &Profile) -> Result<String> {
    profile.validate()?;
    match profile.agent {
        Runner::Claude => {
            let mut text = format!(
                "---\nname: {}\ndescription: {}\n",
                serde_json::to_string(&profile.name)?,
                serde_json::to_string(&format!("The {} role", profile.name))?
            );
            for (key, value) in [("model", &profile.model), ("effort", &profile.effort)] {
                if !value.is_empty() {
                    text.push_str(&format!("{key}: {}\n", serde_json::to_string(value)?));
                }
            }
            text.push_str(&format!(
                "skills: {}\n---\n\n<!-- {OWNED} -->\n\n{}\n",
                serde_json::to_string(&profile.skills)?,
                profile.instructions_for_runner()
            ));
            Ok(text)
        }
        Runner::Codex => {
            let mut text = format!(
                "# {OWNED}\n# seshi-skills: {}\nname = {}\ndescription = {}\ndeveloper_instructions = {}\n",
                serde_json::to_string(&profile.skills)?,
                toml_string(&profile.name),
                toml_string(&format!("The {} role", profile.name)),
                toml_string(&profile.instructions_for_runner())
            );
            if !profile.model.is_empty() {
                text.push_str(&format!("model = {}\n", toml_string(&profile.model)));
            }
            if !profile.effort.is_empty() {
                text.push_str(&format!(
                    "model_reasoning_effort = {}\n",
                    toml_string(&profile.effort)
                ));
            }
            Ok(text)
        }
    }
}

pub fn export(profile: &Profile, root: &Path) -> Result<PathBuf> {
    let (folder, extension) = match profile.agent {
        Runner::Claude => (".claude", "md"),
        Runner::Codex => (".codex", "toml"),
    };
    let path = root
        .join(folder)
        .join("agents")
        .join(format!("{}.{extension}", profile.name));
    export_to(profile, path)
}

pub fn export_personal(profile: &Profile) -> Result<PathBuf> {
    let dir = crate::config::native_agent_dir(profile.agent)?;
    let extension = if profile.agent == Runner::Claude {
        "md"
    } else {
        "toml"
    };
    export_to(profile, dir.join(format!("{}.{extension}", profile.name)))
}

fn export_to(profile: &Profile, path: PathBuf) -> Result<PathBuf> {
    let text = native_text(profile)?;
    match std::fs::read_to_string(&path) {
        Ok(existing) => ensure!(
            existing.contains(OWNED),
            "{} already exists and is not managed by Seshi; choose another profile name",
            path.display()
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("reading native agent definition"),
    }
    crate::config::write_atomic(&path, text.as_bytes())
        .context("exporting native agent definition")?;
    Ok(path)
}

/// Import the portable fields only; refuse native settings we cannot preserve.
pub fn import(path: &Path) -> Result<Profile> {
    let text = std::fs::read_to_string(path).context("reading native agent definition")?;
    let mut profile = if path.extension().is_some_and(|e| e == "toml") {
        let mut table: toml::Table = toml::from_str(&text).context("parsing Codex agent")?;
        let mut take = |key: &str| -> Result<String> {
            match table.remove(key) {
                Some(toml::Value::String(s)) => Ok(s),
                None => Ok(String::new()),
                _ => bail!("{key} must be a string"),
            }
        };
        let skills = text
            .lines()
            .find_map(|l| l.strip_prefix("# seshi-skills: "))
            .map(serde_json::from_str)
            .transpose()
            .context("parsing exported skills")?
            .unwrap_or_default();
        let profile = Profile {
            name: take("name")?,
            agent: Runner::Codex,
            model: take("model")?,
            effort: take("model_reasoning_effort")?,
            instructions: take("developer_instructions")?,
            skills,
        };
        table.remove("description");
        ensure!(
            table.is_empty(),
            "this agent has settings Seshi cannot import: {}",
            table.keys().cloned().collect::<Vec<_>>().join(", ")
        );
        profile
    } else {
        let normalized = text.replace("\r\n", "\n");
        let rest = normalized
            .strip_prefix("---\n")
            .context("Claude agent needs YAML frontmatter")?;
        let (header, body) = rest
            .split_once("\n---\n")
            .context("Claude agent needs a closing frontmatter delimiter")?;
        let mut profile = Profile {
            instructions: body.replace(&format!("<!-- {OWNED} -->"), "").trim().into(),
            ..Default::default()
        };
        for line in header
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        {
            let (key, value) = line.split_once(':').context(
                "unsupported Claude frontmatter; use scalar fields and a JSON skills array",
            )?;
            let value = value.trim();
            let scalar = || -> Result<String> {
                if value.starts_with('"') {
                    Ok(serde_json::from_str(value).context("parsing quoted frontmatter")?)
                } else {
                    ensure!(
                        !matches!(value, "|" | ">") && !value.starts_with('['),
                        "unsupported multiline frontmatter"
                    );
                    Ok(value.trim_matches('\'').into())
                }
            };
            match key.trim() {
                "name" => profile.name = scalar()?,
                "description" => {}
                "model" => profile.model = scalar()?,
                "effort" => profile.effort = scalar()?,
                "skills" => {
                    profile.skills =
                        serde_json::from_str(value).context("skills must be a JSON array")?
                }
                key => bail!("this agent has a setting Seshi cannot import: {key}"),
            }
        }
        profile
    };
    if text.contains(OWNED)
        && !profile.skills.is_empty()
        && let Some(instructions) = profile
            .instructions
            .strip_suffix(&skill_request(&profile.skills))
    {
        profile.instructions = instructions.into();
    }
    profile.validate()?;
    Ok(profile)
}

pub fn run(name: &str, task: &str) -> Result<()> {
    let cfg = crate::config::Config::load()?;
    let dir = std::env::current_dir().context("finding the working directory")?;
    let profiles = load(&cfg, &dir)?;
    let profile = &profiles
        .iter()
        .find(|p| p.profile.name == name)
        .with_context(|| format!("no profile named {name}"))?
        .profile;
    let args = profile.args(task)?;
    let status = crate::proc::agent(profile.agent.command(), &args)?
        .status()
        .with_context(|| format!("starting {}", profile.agent.command()))?;
    ensure!(
        status.success(),
        "{} exited with {status}",
        profile.agent.command()
    );
    Ok(())
}

pub fn encode_task(task: &str) -> Result<String> {
    ensure!(
        task.len() <= MAX_TASK && !task.contains('\0'),
        "task must be at most {MAX_TASK} bytes and contain no NUL"
    );
    Ok(task.as_bytes().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn decode_task(encoded: &str) -> Result<String> {
    ensure!(
        encoded.len() <= MAX_TASK * 2
            && encoded.len().is_multiple_of(2)
            && encoded.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid encoded task"
    );
    let bytes: Result<Vec<u8>, _> = (0..encoded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16))
        .collect();
    let task = String::from_utf8(bytes?).context("task must be UTF-8")?;
    ensure!(!task.contains('\0'), "task must contain no NUL");
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Temp(PathBuf);
    impl Temp {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("seshi-profiles-{}-{name}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn project_roles_replace_matching_personal_roles_and_keep_other_roles() {
        let planner = starter(Runner::Claude, 0);
        let worker = starter(Runner::Claude, 1);
        let mut override_planner = planner.clone();
        override_planner.agent = Runner::Codex;
        override_planner.model = "my-model".into();
        let result = resolve(&[planner, worker.clone()], &[override_planner.clone()]).unwrap();
        assert_eq!(
            result,
            vec![
                Resolved {
                    profile: override_planner,
                    project: true
                },
                Resolved {
                    profile: worker,
                    project: false
                }
            ]
        );
    }

    #[test]
    fn rejects_ambiguous_names_unknown_fields_and_unsafe_export_paths() {
        let p = starter(Runner::Codex, 0);
        assert!(resolve(&[p.clone(), p.clone()], &[]).is_err());
        for name in ["../outside", "-argument", "", "a/b", "a\\b", "two roles"] {
            assert!(
                Profile {
                    name: name.into(),
                    ..p.clone()
                }
                .validate()
                .is_err(),
                "{name}"
            );
        }
        assert!(toml::from_str::<Profile>("name = 'planner'\nagnt = 'codex'").is_err());
        assert!(
            Profile {
                effort: "magic".into(),
                ..p
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn launches_each_runner_with_exact_model_effort_instructions_and_a_fresh_task() {
        let mut p = starter(Runner::Claude, 2);
        p.model = "opus-pinned".into();
        p.skills = vec!["ponytail".into()];
        let task = "review 'quoted' $text\n--resume ignored";
        let args = p.args(task).unwrap();
        let definitions: serde_json::Value = serde_json::from_str(&args[1]).unwrap();
        let role = &definitions[&p.name];
        assert_eq!(role["model"], "opus-pinned");
        assert_eq!(role["effort"], "medium");
        assert_eq!(role["skills"], serde_json::json!(["ponytail"]));
        assert!(
            role["prompt"]
                .as_str()
                .unwrap()
                .contains("Do not edit code")
        );
        assert_eq!(&args[args.len() - 2..], &["--", task]);
        assert!(!args.iter().any(|a| matches!(
            a.as_str(),
            "--resume" | "--continue" | "--dangerously-skip-permissions"
        )));

        p.agent = Runner::Codex;
        let args = p.args(task).unwrap();
        assert_eq!(&args[..2], &["--model", "opus-pinned"]);
        let overrides = args
            .iter()
            .filter(|a| a.contains('='))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let table: toml::Table = toml::from_str(&overrides).unwrap();
        assert_eq!(table["model_reasoning_effort"].as_str(), Some("medium"));
        assert_eq!(
            table["developer_instructions"].as_str(),
            Some(p.instructions_for_runner().as_str())
        );
        assert!(
            !args
                .iter()
                .any(|a| a == "resume" || a == "--dangerously-bypass-approvals-and-sandbox")
        );
    }

    #[test]
    fn saving_profiles_preserves_other_settings_and_rejects_collisions() {
        let temp = Temp::new("save");
        let path = temp.0.join(".seshi.toml");
        std::fs::write(
            &path,
            "# project settings\n[hooks]\non_create = 'npm install' # keep this\n",
        )
        .unwrap();
        let mut p = starter(Runner::Codex, 1);
        save(&path, None, &p).unwrap();
        assert!(save(&path, None, &p).is_err());
        p.model = "my-model".into();
        save(&path, Some(&p.name), &p).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("# project settings") && text.contains("# keep this"));
        let project: crate::project::Project = toml::from_str(&text).unwrap();
        assert_eq!(project.hooks.on_create, "npm install");
        assert_eq!(project_profiles(&temp.0).unwrap(), [p]);
    }

    #[test]
    fn broken_project_profiles_report_errors_instead_of_falling_back() {
        let temp = Temp::new("broken");
        let file = temp.0.join(crate::project::FILE);
        std::fs::write(&file, "[[profiles]]\nname='planner'\neffort='magic'\n").unwrap();
        assert!(load(&crate::config::Config::default(), &temp.0).is_err());
        std::fs::write(&file, "[[profiles]\n").unwrap();
        assert!(project_profiles(&temp.0).is_err());
    }

    #[test]
    fn native_exports_work_for_both_runners_and_preserve_unowned_files() {
        let temp = Temp::new("export");
        for agent in [Runner::Claude, Runner::Codex] {
            let mut p = starter(agent, 0);
            p.skills = vec!["ponytail".into(), "show-me".into()];
            let path = export(&p, &temp.0).unwrap();
            let imported = import(&path).unwrap();
            assert_eq!(imported, p);
            export(&p, &temp.0).unwrap();
            std::fs::write(&path, "user's existing agent").unwrap();
            assert!(export(&p, &temp.0).is_err());
            assert_eq!(
                std::fs::read_to_string(path).unwrap(),
                "user's existing agent"
            );
        }
    }

    #[test]
    fn importing_refuses_to_silently_drop_native_capabilities() {
        let temp = Temp::new("import");
        let file = temp.0.join("native.toml");
        std::fs::write(
            &file,
            "name='reviewer'\nmcp_servers={test={command='other'}}",
        )
        .unwrap();
        assert!(import(&file).is_err());
        let file = temp.0.join("native.md");
        std::fs::write(&file, "---\nname: reviewer\ntools: Read\n---\nReview.").unwrap();
        assert!(import(&file).is_err());
    }

    #[test]
    fn task_transport_preserves_text_and_rejects_malformed_input() {
        let task = "Quotes: '\" & ; $() `\nUnicode: 🐎";
        assert_eq!(decode_task(&encode_task(task).unwrap()).unwrap(), task);
        assert_eq!(decode_task("").unwrap(), "");
        for invalid in ["0", "zz", "é", "ff", "00"] {
            assert!(decode_task(invalid).is_err());
        }
        assert!(encode_task(&"x".repeat(MAX_TASK + 1)).is_err());
    }
}
