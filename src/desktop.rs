use crate::conversation::{ApprovalPolicy, LaunchSettings, Sandbox, StartTarget};
use crate::ipc;
use crate::protocol::{ClientMsg, Command, Query, Reply, ServerMsg};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::PathBuf;

const MAX_TERMINAL_INPUT_BYTES: usize = 16 * 1024;

pub async fn list_sessions(daemon_executable: PathBuf) -> Result<Value> {
    let reply = request(ClientMsg::Query(Query::List), daemon_executable).await?;
    match reply {
        Reply::List(snapshot) => Ok(serde_json::to_value(snapshot)?),
        _ => bail!("daemon returned an unexpected session list reply"),
    }
}

/// Report whether a local daemon speaks an older protocol and can be replaced by this build.
pub async fn daemon_restart_status() -> Result<Value> {
    anyhow::ensure!(
        crate::ipc::remote().is_none(),
        concat!(
            "the GUI can restart only the local ",
            env!("APP_NAME"),
            " service"
        )
    );
    let daemon_protocol = match ipc::open_for_user(false).await {
        Err(error) => ipc::other_version(&error)
            .ok_or_else(|| error.context("checking the local daemon version"))?,
        Ok((reader, writer)) => {
            drop((reader, writer));
            bail!(concat!(
                "the local ",
                env!("APP_NAME"),
                " service is already compatible with this GUI"
            ))
        }
    };
    let activity = crate::restart::read();
    let session_count = activity
        .as_ref()
        .map_or_else(crate::restart::saved_agents, |activity| activity.sessions);
    Ok(json!({
        "daemonProtocol": daemon_protocol,
        "guiProtocol": crate::protocol::PROTOCOL_VERSION,
        "canRestart": daemon_protocol < crate::protocol::PROTOCOL_VERSION,
        "sessionCount": session_count,
        "activeTurnCount": activity.as_ref().map(crate::restart::Activity::busy),
    }))
}

/// Stop only the exact older local daemon the user confirmed replacing.
pub async fn restart_mismatched_daemon(expected_protocol: u32) -> Result<()> {
    anyhow::ensure!(
        crate::ipc::remote().is_none(),
        concat!(
            "the GUI can restart only the local ",
            env!("APP_NAME"),
            " service"
        )
    );
    let daemon_protocol = match ipc::open_for_user(false).await {
        Err(error) => ipc::other_version(&error)
            .ok_or_else(|| error.context("checking the local daemon version"))?,
        Ok((reader, writer)) => {
            drop((reader, writer));
            bail!(concat!(
                "the local ",
                env!("APP_NAME"),
                " service is already compatible with this GUI"
            ))
        }
    };
    validate_restart_confirmation(daemon_protocol, expected_protocol)?;
    tokio::task::spawn_blocking(|| crate::cli::kill_server(false))
        .await
        .context(concat!(
            "waiting for the local ",
            env!("APP_NAME"),
            " service to stop"
        ))??;
    Ok(())
}

fn validate_restart_confirmation(daemon_protocol: u32, expected_protocol: u32) -> Result<()> {
    anyhow::ensure!(
        daemon_protocol == expected_protocol,
        concat!(
            "the local ",
            env!("APP_NAME"),
            " service changed since the restart prompt opened"
        )
    );
    anyhow::ensure!(
        daemon_protocol < crate::protocol::PROTOCOL_VERSION,
        concat!(
            "the local ",
            env!("APP_NAME"),
            " service is newer than this GUI; update the GUI before reconnecting"
        )
    );
    Ok(())
}

pub async fn read_conversation(
    id: u64,
    cursor: Option<Value>,
    limit: usize,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let cursor = cursor
        .map(serde_json::from_value)
        .transpose()
        .context("invalid conversation history cursor")?;
    let reply = request(
        ClientMsg::Query(Query::ConversationRead {
            id: crate::conversation::ConversationId(id),
            cursor,
            limit,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationRead(history) => Ok(serde_json::to_value(history)?),
        _ => bail!("daemon returned an unexpected conversation history reply"),
    }
}

pub async fn conversation_diff(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Query(Query::ConversationDiff {
            id: crate::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationDiff(diff) => Ok(serde_json::to_value(diff)?),
        _ => bail!("daemon returned an unexpected conversation diff reply"),
    }
}

pub async fn terminal_screen(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<String> {
    let reply = request(
        ClientMsg::Query(Query::ConversationTerminalScreen {
            id: crate::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Text(screen) => Ok(screen),
        _ => bail!("daemon returned an unexpected terminal screen reply"),
    }
}

pub async fn terminal_replay(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Vec<u8>> {
    let reply = request(
        ClientMsg::Query(Query::ConversationTerminalReplay {
            id: crate::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::TerminalReplay(replay) => Ok(replay),
        _ => bail!("daemon returned an unexpected terminal replay reply"),
    }
}

pub async fn send_terminal_input(
    id: u64,
    generation: u64,
    data: Vec<u8>,
    daemon_executable: PathBuf,
) -> Result<String> {
    anyhow::ensure!(
        !data.is_empty() && data.len() <= MAX_TERMINAL_INPUT_BYTES,
        "Terminal input must contain between 1 and {MAX_TERMINAL_INPUT_BYTES} bytes"
    );
    let (mut reader, mut writer) = ipc::open_or_spawn_for_user(false, daemon_executable)
        .await
        .context("connecting to the local daemon")?;
    ipc::send(
        &mut writer,
        &ClientMsg::Command(Command::ConversationTerminalInput {
            id: crate::conversation::ConversationId(id),
            generation,
            data,
        }),
    )
    .await?;
    match receive_reply(&mut reader).await? {
        Reply::Ok => {}
        _ => bail!("daemon returned an unexpected terminal input reply"),
    }
    ipc::send(
        &mut writer,
        &ClientMsg::Query(Query::ConversationTerminalScreen {
            id: crate::conversation::ConversationId(id),
            generation,
        }),
    )
    .await?;
    receive_reply(&mut reader)
        .await
        .and_then(|reply| match reply {
            Reply::Text(screen) => Ok(screen),
            _ => bail!("daemon returned an unexpected terminal screen reply"),
        })
}

pub async fn resize_terminal(
    id: u64,
    generation: u64,
    cols: u16,
    rows: u16,
    daemon_executable: PathBuf,
) -> Result<()> {
    let reply = request(
        ClientMsg::Command(Command::ConversationTerminalResize {
            id: crate::conversation::ConversationId(id),
            generation,
            cols,
            rows,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(()),
        _ => bail!("daemon returned an unexpected terminal resize reply"),
    }
}

pub async fn send_conversation(
    id: u64,
    generation: u64,
    operation: u64,
    text: String,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationSend {
            id: crate::conversation::ConversationId(id),
            generation,
            operation,
            text,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationSent(receipt) => Ok(serde_json::to_value(receipt)?),
        _ => bail!("daemon returned an unexpected conversation send reply"),
    }
}

pub async fn approve_conversation(
    id: u64,
    generation: u64,
    request_id: crate::conversation::ProviderRequestId,
    decision: crate::conversation::ApprovalDecision,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationApprove {
            id: crate::conversation::ConversationId(id),
            generation,
            request_id,
            decision,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected conversation approval reply"),
    }
}

pub async fn answer_user_input(
    id: u64,
    generation: u64,
    request_id: crate::conversation::ProviderRequestId,
    answers: std::collections::BTreeMap<String, Vec<String>>,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationAnswerInput {
            id: crate::conversation::ConversationId(id),
            generation,
            request_id,
            answers,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected conversation input reply"),
    }
}

pub async fn cancel_conversation(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationCancel {
            id: crate::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected turn cancellation reply"),
    }
}

pub async fn handoff_conversation_to_native(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationHandoffToNative {
            id: crate::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::Ok => Ok(Value::Null),
        _ => bail!("daemon returned an unexpected native conversation handoff reply"),
    }
}

pub async fn handoff_conversation_to_chat(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationHandoffToChat {
            id: crate::conversation::ConversationId(id),
            generation,
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationStarted { id, generation } => {
            Ok(json!({"id": id.0, "generation": generation}))
        }
        _ => bail!("daemon returned an unexpected chat handoff reply"),
    }
}

pub async fn resume_codex_conversation(
    id: u64,
    generation: u64,
    daemon_executable: PathBuf,
) -> Result<Value> {
    let reply = request(
        ClientMsg::Command(Command::ConversationStart {
            target: StartTarget::Conversation {
                id: crate::conversation::ConversationId(id),
                generation,
            },
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationStarted { id, generation } => {
            Ok(json!({"id": id.0, "generation": generation}))
        }
        _ => bail!("daemon returned an unexpected conversation resume reply"),
    }
}

pub async fn create_codex_conversation(cwd: PathBuf, daemon_executable: PathBuf) -> Result<Value> {
    let cwd = cwd
        .canonicalize()
        .context("resolving the selected project folder")?;
    if !cwd.is_dir() {
        bail!("the selected project folder is not a directory");
    }
    let settings = LaunchSettings {
        provider: crate::conversation::Provider::Codex,
        executable: codex_app_server_executable()?,
        config_root: crate::config::codex_config_dir(),
        cwd,
        model: None,
        model_provider: None,
        approval_policy: ApprovalPolicy::OnRequest,
        sandbox: Sandbox::ReadOnly,
    };
    let reply = request(
        ClientMsg::Command(Command::ConversationStart {
            target: StartTarget::Create(settings),
        }),
        daemon_executable,
    )
    .await?;
    match reply {
        Reply::ConversationStarted { id, generation } => {
            Ok(json!({"id": id, "generation": generation}))
        }
        _ => bail!("daemon returned an unexpected conversation start reply"),
    }
}

pub fn codex_app_server_executable() -> Result<PathBuf> {
    match crate::proc::executable_on_path("codex") {
        Ok(executable) => Ok(executable),
        Err(error) => {
            #[cfg(windows)]
            {
                if let Some(executable) = windows_npm_codex_binary(windows_npm_search_roots(
                    std::env::var_os("PATH")
                        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
                        .unwrap_or_default(),
                    std::env::var_os("NVM_SYMLINK").map(PathBuf::from),
                    std::env::var_os("APPDATA").map(PathBuf::from),
                    std::env::var_os("ProgramFiles").map(PathBuf::from),
                )) {
                    return Ok(executable);
                }
                Err(error.context(
                    "Codex executable not found on PATH or in common Windows npm locations",
                ))
            }
            #[cfg(not(windows))]
            Err(error)
        }
    }
}

#[cfg(windows)]
fn windows_npm_search_roots(
    path_entries: impl IntoIterator<Item = PathBuf>,
    nvm_symlink: Option<PathBuf>,
    app_data: Option<PathBuf>,
    program_files: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = path_entries.into_iter().collect::<Vec<_>>();
    if let Some(nvm_symlink) = nvm_symlink {
        roots.push(nvm_symlink);
    }
    if let Some(app_data) = app_data {
        roots.push(app_data.join("npm"));
    }
    if let Some(program_files) = program_files {
        roots.push(program_files.join("nodejs"));
    }
    roots
}

#[cfg(windows)]
fn windows_npm_codex_binary(paths: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    const PLATFORM_BINARY: &str = "vendor/x86_64-pc-windows-msvc/bin/codex.exe";
    const PACKAGE_LAYOUTS: &[&str] = &[
        "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64",
        "node_modules/@openai/codex-win32-x64",
        "node_modules/@openai/codex",
    ];

    for path in paths {
        for ancestor in path.ancestors() {
            for layout in PACKAGE_LAYOUTS {
                let executable = ancestor.join(layout).join(PLATFORM_BINARY);
                if executable.is_file() {
                    return executable.canonicalize().ok();
                }
            }
        }
    }
    None
}

#[cfg(all(test, windows))]
mod tests {
    use super::{windows_npm_codex_binary, windows_npm_search_roots};
    use std::path::PathBuf;

    #[test]
    fn finds_codex_in_the_nested_windows_npm_package() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-app-server-{}-{}",
            crate::brand::APP_COMMAND,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let prefix = root.join("npm");
        let executable = prefix.join(
            "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let resolved = windows_npm_codex_binary([PathBuf::from(&prefix)]).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_codex_under_nvm_symlink_when_node_is_not_on_path() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-nvm-path-{}-{}",
            crate::brand::APP_COMMAND,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nvm_symlink = root.join("nodejs");
        let executable = nvm_symlink.join(
            "node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let roots = windows_npm_search_roots([], Some(nvm_symlink.clone()), None, None);
        let resolved = super::windows_npm_codex_binary(roots).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_codex_under_the_standard_npm_root_when_node_is_not_on_path() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-npm-root-{}-{}",
            crate::brand::APP_COMMAND,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app_data = root.join("roaming");
        let executable = app_data.join(
            "npm/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let roots = windows_npm_search_roots([], None, Some(app_data), None);
        let resolved = super::windows_npm_codex_binary(roots).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_codex_under_the_system_node_root_when_node_is_not_on_path() {
        let root = std::env::temp_dir().join(format!(
            "{}-codex-system-node-{}-{}",
            crate::brand::APP_COMMAND,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let program_files = root.join("program-files");
        let executable = program_files.join(
            "nodejs/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe",
        );
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"").unwrap();

        let roots = windows_npm_search_roots([], None, None, Some(program_files));
        let resolved = super::windows_npm_codex_binary(roots).unwrap();

        assert_eq!(resolved, executable.canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}

async fn request(message: ClientMsg, daemon_executable: PathBuf) -> Result<Reply> {
    let (mut reader, mut writer) = ipc::open_or_spawn_for_user(false, daemon_executable)
        .await
        .context("connecting to the local daemon")?;
    ipc::send(&mut writer, &message).await?;
    receive_reply(&mut reader).await
}

async fn receive_reply(reader: &mut ipc::Reader) -> Result<Reply> {
    loop {
        match ipc::recv_server(reader).await? {
            Some(ServerMsg::Reply(reply)) => return Ok(reply),
            Some(ServerMsg::Error(error)) => bail!(error),
            Some(ServerMsg::Bye) => bail!("local daemon closed the connection"),
            Some(_) => continue,
            None => bail!("local daemon closed the connection"),
        }
    }
}

#[cfg(test)]
mod restart_confirmation_tests {
    use super::validate_restart_confirmation;

    #[test]
    fn accepts_only_the_confirmed_older_protocol() {
        let current = crate::protocol::PROTOCOL_VERSION;
        assert!(validate_restart_confirmation(current - 1, current - 1).is_ok());
        assert!(validate_restart_confirmation(current - 2, current - 1).is_err());
        assert!(validate_restart_confirmation(current + 1, current + 1).is_err());
    }
}
