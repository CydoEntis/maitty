use crate::conversation::{
    Conversation, ConversationId, ExecutionKind, NativeBinding, Provider, RecoveryState,
};

pub(crate) fn conversation() -> Conversation {
    Conversation {
        id: ConversationId(1),
        binding: NativeBinding {
            provider: Provider::Codex,
            session_id: "exact-session".into(),
            account: Some("work".into()),
            profile: Some("native-profile".into()),
            config_root: std::env::temp_dir().join("synthetic-config"),
            executable: std::env::temp_dir().join("synthetic-codex.exe"),
            cwd: std::env::temp_dir().join("synthetic-repo"),
            worktree: Some(std::env::temp_dir().join("synthetic-tree")),
        },
        execution: ExecutionKind::Structured,
        generation: 7,
        state: RecoveryState::Stopped,
        settings: None,
        awaiting_first_user_message: false,
        title: None,
        archived: false,
        worktree_origin: None,
    }
}
