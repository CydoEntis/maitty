use super::*;
use serde_json::json;

pub(in crate::daemon::conversation) fn entry(index: usize) -> Entry {
    Entry {
        source: SourceId {
            conversation: ConversationId(1),
            turn: "turn".into(),
            item: index.to_string(),
        },
        content: Content::Message {
            role: Role::Assistant,
            text: format!("synthetic {index}"),
        },
        state: ItemState::Completed,
    }
}
#[test]
fn completion_replay_replaces_the_stable_item_in_its_original_position() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.apply(Update::UpsertItem(entry(2))).unwrap();
    let revision = h.revision;
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    assert_eq!(h.revision, revision);
    let mut changed = entry(1);
    changed.state = ItemState::Failed;
    h.apply(Update::UpsertItem(changed.clone())).unwrap();
    assert_eq!(
        h.read(None, 10, MAX_READ_BYTES).unwrap().entries,
        vec![changed, entry(2)]
    );
}
#[test]
fn identical_delta_chunks_are_legitimate_until_authoritative_completion() {
    let mut a = Attachments::default();
    a.attach(7, ConversationId(1));
    for _ in 0..2 {
        a.notification(
            ConversationId(1),
            "native",
            "item/agentMessage/delta",
            &json!({"threadId":"native","turnId":"turn","itemId":"1","delta":"same"}),
        )
        .unwrap();
    }
    let h = a.get(7).unwrap();
    assert_eq!(
        h.entries[0].content,
        Content::Message {
            role: Role::Assistant,
            text: "samesame".into()
        }
    );
    assert!(h.durable().entries.is_empty());
    a.reconnect(ConversationId(1)).unwrap();
    assert!(a.get(7).unwrap().entries.is_empty());
    a.notification(ConversationId(1), "native", "item/completed", &json!({"threadId":"native","turnId":"turn","item":{"id":"1","type":"agentMessage","text":"authoritative"}})).unwrap();
    assert_eq!(a.get(7).unwrap().entries.len(), 1);
    a.detach(7);
    assert!(a.get(7).is_none());
    a.attach(8, ConversationId(1));
    assert!(a.get(8).unwrap().entries.is_empty());
}
#[test]
fn turn_status_has_an_independent_stable_row_even_without_items() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::SetTurnStatus {
        turn: "turn".into(),
        state: ItemState::Running,
    })
    .unwrap();
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.apply(Update::SetTurnStatus {
        turn: "turn".into(),
        state: ItemState::Failed,
    })
    .unwrap();
    assert_eq!(h.entries.len(), 2);
    assert_eq!(h.entries[0].state, ItemState::Failed);
    assert_eq!(h.entries[1].state, ItemState::Completed);
}

#[test]
fn interrupting_a_turn_marks_its_running_tools_interrupted() {
    let mut history = History::new(ConversationId(1));
    let tool = |turn: &str| Entry {
        source: SourceId {
            conversation: ConversationId(1),
            turn: turn.into(),
            item: "command".into(),
        },
        content: Content::Tool {
            kind: "commandExecution".into(),
            output: String::new(),
            label: "Run command".into(),
        },
        state: ItemState::Running,
    };
    history
        .apply(Update::UpsertItem(tool("interrupted")))
        .unwrap();
    history.apply(Update::UpsertItem(tool("active"))).unwrap();

    history
        .apply(Update::SetTurnStatus {
            turn: "interrupted".into(),
            state: ItemState::Interrupted,
        })
        .unwrap();

    assert_eq!(history.entries[0].state, ItemState::Interrupted);
    assert_eq!(history.entries[1].state, ItemState::Running);
}

#[test]
fn cursor_exhaustion_never_claims_native_completeness() {
    let mut h = History::new(ConversationId(1));
    assert_eq!(
        h.read(None, 1, MAX_READ_BYTES).unwrap().completeness,
        Completeness::Unknown
    );
    h.set_completeness(Completeness::Loading).unwrap();
    for i in 0..3 {
        h.apply(Update::UpsertItem(entry(i))).unwrap();
    }
    let first = h.read(None, 2, MAX_READ_BYTES).unwrap();
    let last = h.read(first.next.as_ref(), 2, MAX_READ_BYTES).unwrap();
    assert_eq!(last.entries, vec![entry(2)]);
    assert!(last.next.is_none());
    assert_eq!(last.completeness, Completeness::Loading);
    h.set_completeness(Completeness::Complete).unwrap();
    assert_eq!(
        h.read(first.next.as_ref(), 2, MAX_READ_BYTES),
        Err(ReadError::StaleCursor)
    );
}
#[test]
fn cursor_is_invalidated_by_content_updates_and_other_conversations() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(0))).unwrap();
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    let first = h.read(None, 1, MAX_READ_BYTES).unwrap();
    let other = History::new(ConversationId(2));
    assert_eq!(
        other.read(first.next.as_ref(), 1, MAX_READ_BYTES),
        Err(ReadError::StaleCursor)
    );
    let mut changed = entry(1);
    changed.state = ItemState::Failed;
    h.apply(Update::UpsertItem(changed)).unwrap();
    assert_eq!(
        h.read(first.next.as_ref(), 1, MAX_READ_BYTES),
        Err(ReadError::StaleCursor)
    );
}
#[test]
fn serialized_read_bounds_never_silently_truncate_an_oversized_item() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.apply(Update::UpsertItem(entry(2))).unwrap();
    let one = h.read(None, 1, MAX_READ_BYTES).unwrap();
    let limit = serde_json::to_vec(&one).unwrap().len();
    let page = h.read(None, 2, limit).unwrap();
    assert_eq!(page.entries.len(), 1);
    assert!(serde_json::to_vec(&page).unwrap().len() <= limit);
    assert_eq!(h.read(None, 2, 1), Err(ReadError::EntryTooLarge));
    assert_eq!(
        h.read(None, 0, MAX_READ_BYTES),
        Err(ReadError::InvalidLimit)
    );
}
#[test]
fn wrong_thread_and_unsupported_input_mark_history_incomplete_without_raw_payload() {
    let mut a = Attachments::default();
    a.attach(1, ConversationId(1));
    a.notification(
        ConversationId(1),
        "native",
        "item/completed",
        &json!({"threadId":"other"}),
    )
    .unwrap();
    assert_eq!(
        a.get(1).unwrap().completeness,
        Completeness::Incomplete(IncompleteReason::InvalidEvent)
    );
    a.notification(ConversationId(1), "native", "item/completed", &json!({"threadId":"native","turnId":"turn","item":{"id":"1","type":"userMessage","content":[{"type":"image","url":"synthetic"}]}})).unwrap();
    assert_eq!(
        a.get(1).unwrap().completeness,
        Completeness::Incomplete(IncompleteReason::UnsupportedItem)
    );
}
#[test]
fn authoritative_hydration_replaces_drafts_in_native_order() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::AppendDelta {
        source: entry(1).source,
        delta: "uncertain".into(),
    })
    .unwrap();
    h.reconnect().unwrap();
    h.hydrate(vec![entry(20), entry(1)], Completeness::Complete)
        .unwrap();
    let page = h.read(None, 10, MAX_READ_BYTES).unwrap();
    assert_eq!(page.entries, vec![entry(20), entry(1)]);
    assert_eq!(page.completeness, Completeness::Complete);
    assert!(
        h.hydrate(vec![entry(1), entry(1)], Completeness::Complete)
            .is_err()
    );
}
#[test]
fn a_codex_file_edit_keeps_its_files_and_diff_for_review() {
    let mut a = Attachments::default();
    a.attach(1, ConversationId(1));
    a.notification(ConversationId(1), "native", "item/completed", &json!({"threadId":"native","turnId":"turn","item":{"id":"edit","type":"fileChange","status":"completed","changes":[{"path":"main.go","kind":{"type":"update"},"diff":"@@ -1 +1 @@
-old
+new
"},{"path":"notes.txt","kind":"add","diff":"hello"}]}})).unwrap();
    let history = a.get(1).unwrap();
    // A file edit is understood now; it must not mark the history as unsupported.
    assert_ne!(
        history.completeness,
        Completeness::Incomplete(IncompleteReason::UnsupportedItem)
    );
    assert_eq!(history.entries[0].state, ItemState::Completed);
    let Content::Tool { kind, output, label } = &history.entries[0].content else {
        panic!("a file edit is shown as a tool row");
    };
    assert_eq!(kind, "fileChange");
    assert_eq!(label, "Edited 2 files");
    assert!(output.contains("main.go (update)") && output.contains("+new"));
    assert!(output.contains("notes.txt (add)
hello"));
}

#[test]
fn provider_tool_lifecycle_replaces_one_row_and_preserves_failed_status() {
    let mut a = Attachments::default();
    a.attach(1, ConversationId(1));
    for (method, status) in [
        ("item/started", "inProgress"),
        ("item/completed", "failed"),
        ("item/completed", "failed"),
    ] {
        a.notification(ConversationId(1), "native", method, &json!({"threadId":"native","turnId":"turn","item":{"id":"1","type":"commandExecution","command":"pwsh -NoProfile -Command Write-Output smoke","aggregatedOutput":"synthetic output","status":status}})).unwrap();
    }
    assert_eq!(a.get(1).unwrap().entries.len(), 1);
    assert_eq!(a.get(1).unwrap().entries[0].state, ItemState::Failed);
    assert!(matches!(
        &a.get(1).unwrap().entries[0].content,
        Content::Tool { label, .. } if label == "pwsh -NoProfile -Command Write-Output smoke"
    ));
    a.notification(ConversationId(1), "native", "item/completed", &json!({"threadId":"native","turnId":"turn","item":{"id":"2","type":"unknownTool","secret":"not retained"}})).unwrap();
    assert_eq!(
        a.get(1).unwrap().completeness,
        Completeness::Incomplete(IncompleteReason::UnsupportedItem)
    );
    a.notification(
        ConversationId(1),
        "native",
        "turn/completed",
        &json!({"threadId":"native","turn":{"id":"turn","status":"completed"}}),
    )
    .unwrap();
    a.notification(ConversationId(1), "native", "ignored", &json!({}))
        .unwrap();
}

#[test]
fn private_reasoning_items_do_not_make_user_visible_history_incomplete() {
    let mut attachments = Attachments::default();
    attachments.attach(1, ConversationId(1));

    attachments
        .notification(
            ConversationId(1),
            "native",
            "item/completed",
            &json!({
                "threadId": "native",
                "turnId": "turn",
                "item": {"id": "private", "type": "reasoning", "text": "not retained"}
            }),
        )
        .unwrap();

    let history = attachments.get(1).unwrap();
    assert!(history.entries.is_empty());
    assert_eq!(history.completeness, Completeness::Unknown);
}

#[test]
fn authoritative_items_ignore_replayed_deltas_and_hydration_dies_with_attachment() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.apply(Update::AppendDelta {
        source: entry(1).source,
        delta: "replayed".into(),
    })
    .unwrap();
    assert_eq!(h.entries, vec![entry(1)]);
    let mut a = Attachments::default();
    a.attach(4, ConversationId(1));
    a.hydrate_native(4, h.clone()).unwrap();
    assert_eq!(a.get(4).unwrap().entries, vec![entry(1)]);
    a.detach(4);
    assert!(a.hydrate_native(4, h).is_err());
}
#[test]
fn completed_native_item_cannot_regress_when_started_is_replayed() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    let mut started = entry(1);
    started.state = ItemState::Running;
    h.apply(Update::UpsertItem(started)).unwrap();
    assert_eq!(h.entries, vec![entry(1)]);
}
#[test]
fn hydrated_active_prefix_survives_following_delta() {
    let mut h = History::new(ConversationId(1));
    let mut active = entry(1);
    active.state = ItemState::Running;
    active.content = Content::Message {
        role: Role::Assistant,
        text: "synthetic prefix".into(),
    };
    h.hydrate(vec![active], Completeness::Complete).unwrap();
    h.apply(Update::AppendDelta {
        source: entry(1).source,
        delta: " suffix".into(),
    })
    .unwrap();
    assert_eq!(
        h.entries[0].content,
        Content::Message {
            role: Role::Assistant,
            text: "synthetic prefix suffix".into()
        }
    );
}
#[test]
fn hydrated_active_content_never_enters_a_complete_durable_projection() {
    let mut h = History::new(ConversationId(1));
    let mut active = entry(1);
    active.state = ItemState::Running;
    h.hydrate(vec![active], Completeness::Complete).unwrap();
    let saved = h.durable();
    assert!(saved.entries.is_empty());
    assert_ne!(saved.completeness, Completeness::Complete);
}
#[test]
fn empty_delta_preserves_content_and_existing_cursor() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.apply(Update::UpsertItem(entry(2))).unwrap();
    let cursor = h.read(None, 1, MAX_READ_BYTES).unwrap().next.unwrap();
    let before = h.clone();
    h.apply(Update::AppendDelta {
        source: entry(3).source,
        delta: String::new(),
    })
    .unwrap();
    assert_eq!(h, before);
    assert_eq!(
        h.read(Some(&cursor), 2, MAX_READ_BYTES).unwrap().entries,
        vec![entry(2)]
    );
}
#[test]
fn exhausted_revision_rejects_apply_before_content_or_cursor_changes() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.apply(Update::UpsertItem(entry(2))).unwrap();
    h.revision = u64::MAX;
    let cursor = h.read(None, 1, MAX_READ_BYTES).unwrap().next.unwrap();
    let before = h.clone();
    assert_eq!(
        h.apply(Update::UpsertItem(entry(3))),
        Err(IncompleteReason::RevisionExhausted)
    );
    assert_eq!(h, before);
    assert_eq!(
        h.read(Some(&cursor), 1, MAX_READ_BYTES).unwrap().entries,
        vec![entry(2)]
    );
}
#[test]
fn exhausted_reconnect_and_hydration_leave_existing_content_unchanged() {
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.revision = u64::MAX;
    let before = h.clone();
    assert_eq!(h.reconnect(), Err(IncompleteReason::RevisionExhausted));
    assert_eq!(h, before);
    assert!(h.hydrate(vec![entry(2)], Completeness::Complete).is_err());
    assert_eq!(h, before);
}
#[test]
fn exhausted_imported_revisions_cannot_partially_replace_an_attachment() {
    let mut attachments = Attachments::default();
    attachments.attach(1, ConversationId(1));
    let mut original = History::new(ConversationId(1));
    original.apply(Update::UpsertItem(entry(1))).unwrap();
    attachments.hydrate_native(1, original).unwrap();
    let before = attachments.get(1).unwrap().clone();
    for revision in [u64::MAX, u64::MAX - 1] {
        let mut imported = History::new(ConversationId(1));
        imported.apply(Update::UpsertItem(entry(2))).unwrap();
        imported.revision = revision;
        if revision == u64::MAX {
            assert!(attachments.hydrate_native(1, imported.clone()).is_err());
            assert_eq!(attachments.get(1), Some(&before));
        }
        assert!(attachments.hydrate_cached(1, imported).is_err());
        assert_eq!(attachments.get(1), Some(&before));
    }
}
#[test]
fn last_available_revision_advances_once_and_then_rejects_without_wrapping() {
    let mut h = History::new(ConversationId(1));
    h.revision = u64::MAX - 1;
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    assert_eq!(h.revision, u64::MAX);
    let before = h.clone();
    assert_eq!(
        h.set_completeness(Completeness::Complete),
        Err(IncompleteReason::RevisionExhausted)
    );
    assert_eq!(h, before);
    assert_eq!(
        h.apply(Update::AppendDelta {
            source: entry(2).source,
            delta: "synthetic".into()
        }),
        Err(IncompleteReason::RevisionExhausted)
    );
    assert_eq!(h, before);
    assert_eq!(
        h.hydrate(vec![entry(3)], Completeness::Complete),
        Err(IncompleteReason::RevisionExhausted)
    );
    assert_eq!(h, before);
}
#[test]
fn near_exhausted_cached_hydration_uses_only_available_revisions() {
    let mut attachments = Attachments::default();
    attachments.attach(1, ConversationId(1));
    let mut imported = History::new(ConversationId(1));
    imported.apply(Update::UpsertItem(entry(1))).unwrap();
    imported.revision = u64::MAX - 2;
    attachments.hydrate_cached(1, imported).unwrap();
    let h = attachments.get(1).unwrap();
    assert_eq!(h.revision, u64::MAX);
    assert_eq!(h.entries, vec![entry(1)]);
    let before = h.clone();
    assert_eq!(
        attachments.reconnect(ConversationId(1)),
        Err(IncompleteReason::RevisionExhausted)
    );
    assert_eq!(attachments.get(1), Some(&before));
}
#[test]
fn exhausted_attachment_prevents_partial_notification_or_reconnect_updates() {
    let mut attachments = Attachments::default();
    for attachment in [1, 2] {
        attachments.attach(attachment, ConversationId(1));
    }
    attachments.histories.get_mut(&2).unwrap().revision = u64::MAX;
    let first = attachments.get(1).unwrap().clone();
    let second = attachments.get(2).unwrap().clone();
    assert_eq!(attachments.notification(ConversationId(1), "native", "item/completed", &json!({"threadId":"native","turnId":"turn","item":{"id":"1","type":"agentMessage","text":"synthetic"}})), Err(IncompleteReason::RevisionExhausted));
    assert_eq!(attachments.get(1), Some(&first));
    assert_eq!(attachments.get(2), Some(&second));
    assert_eq!(
        attachments.reconnect(ConversationId(1)),
        Err(IncompleteReason::RevisionExhausted)
    );
    assert_eq!(attachments.get(1), Some(&first));
    assert_eq!(attachments.get(2), Some(&second));
}
