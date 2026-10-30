mod alert;
mod app;
mod brand;
mod cli;
mod client;
mod clock;
mod config;
mod daemon;
pub use maitty_shared::{conversation, layout, protocol};
mod desktop;
mod detection;
mod gitfs;
mod hidden;
mod ipc;
mod keys;
mod mcp;
mod output_line;
mod proc;
mod project;
mod references;
mod restart;
mod reveal;
mod secret_files;
mod shim;
mod snapshot;
mod sync;
mod theme;
mod tidy;
mod update;

#[cfg(test)]
mod test_support;

pub use conversation::{ApprovalDecision, ProviderRequestId};

pub use desktop::{
    answer_user_input, approve_conversation, cancel_conversation, codex_app_server_executable,
    conversation_diff, create_codex_conversation, daemon_restart_status,
    handoff_conversation_to_chat, handoff_conversation_to_native, list_sessions, read_conversation,
    resize_terminal, restart_mismatched_daemon, resume_codex_conversation, send_conversation,
    send_terminal_input, terminal_replay, terminal_screen,
};

pub fn run() {
    app::run();
}
