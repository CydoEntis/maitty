use super::super::history::tests::entry;
use super::super::history::{Completeness, Content, IncompleteReason, Update};
use super::*;
use crate::daemon::persist::scratch_dir;

#[test]
fn atomic_manifest_is_the_only_committed_revision_and_orphans_do_not_count() {
    let scratch = scratch_dir();
    let root = scratch.path().join("history");
    let cache = Cache::open(root.clone(), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(0))).unwrap();
    assert!(cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap());
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.set_completeness(Completeness::Complete).unwrap();
    assert!(
        cache
            .commit_with(cache.prepare(&binding, &h).unwrap(), || bail!(
                "synthetic interruption"
            ))
            .is_err()
    );
    let saved = cache.load(h.conversation, &binding).unwrap().unwrap();
    assert_eq!(saved.entries, vec![entry(0)]);
    assert_eq!(saved.completeness, Completeness::Unknown);
    assert!(cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap());
    assert_eq!(cache.load(h.conversation, &binding).unwrap(), Some(h));
}
#[test]
fn missing_and_corrupt_pages_cannot_claim_complete_history() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(0))).unwrap();
    h.set_completeness(Completeness::Complete).unwrap();
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    let path = cache.root.join("1/1.json");
    std::fs::write(&path, b"[]").unwrap();
    assert!(cache.load(h.conversation, &binding).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(cache.load(h.conversation, &binding).is_err());
    h.set_completeness(Completeness::Incomplete(IncompleteReason::MissingPages))
        .unwrap();
    assert_ne!(h.completeness, Completeness::Complete);
}
#[test]
fn disabling_storage_fences_queued_writes_and_deletes_existing_content() {
    let scratch = scratch_dir();
    let root = scratch.path().join("history");
    let cache = Cache::open(root.clone(), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let h = History::new(ConversationId(1));
    let queued = cache.prepare(&binding, &h).unwrap();
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    cache.configure(true, false).unwrap();
    assert!(!root.exists());
    assert!(!cache.commit(queued).unwrap());
    assert!(!root.exists());
    assert!(cache.prepare(&binding, &h).is_err());
    assert!(cache.load(h.conversation, &binding).unwrap().is_none());
    cache.configure(true, true).unwrap();
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    Cache::open(root.clone(), false, true).unwrap();
    assert!(!root.exists());
}
#[test]
fn disabled_cache_cleanup_errors_are_explicit_and_leave_writes_disabled() {
    let scratch = scratch_dir();
    let root = scratch.path().join("history");
    std::fs::create_dir_all(scratch.path()).unwrap();
    std::fs::write(&root, "synthetic").unwrap();
    assert!(Cache::open(root, true, false).is_err());
}
#[test]
fn private_pages_are_bounded_and_binding_changes_require_native_hydration() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let mut binding = crate::conversation::tests::conversation().binding;
    binding.session_id = "../../not-a-path".into();
    let mut h = History::new(ConversationId(1));
    for i in 0..300 {
        h.apply(Update::UpsertItem(entry(i))).unwrap();
    }
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    assert_eq!(
        cache.load(h.conversation, &binding).unwrap(),
        Some(h.clone())
    );
    let manifest: Manifest =
        serde_json::from_slice(&std::fs::read(cache.root.join("1/manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest.pages.len(), 3);
    for page in manifest.pages {
        assert!(page.bytes <= MAX_READ_BYTES);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(cache.root.join("1/1.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&cache.root).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    binding.account = Some("other".into());
    assert!(cache.load(h.conversation, &binding).is_err());
    let mut oversized = entry(400);
    oversized.content = Content::Message {
        role: super::super::history::Role::User,
        text: "x".repeat(MAX_READ_BYTES),
    };
    h.apply(Update::UpsertItem(oversized)).unwrap();
    assert!(cache.commit(cache.prepare(&binding, &h).unwrap()).is_err());
}
#[test]
fn session_paths_isolate_history_and_old_queued_revisions_cannot_replace_newer_ones() {
    let scratch = scratch_dir();
    let first = scratch.path().join("session-first.json");
    let cache = Cache::for_session(&first, true, true).unwrap();
    let other =
        Cache::for_session(&scratch.path().join("session-second.json"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut h = History::new(ConversationId(1));
    let older = cache.prepare(&binding, &h).unwrap();
    h.apply(Update::UpsertItem(entry(0))).unwrap();
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    assert!(!cache.commit(older).unwrap());
    assert_eq!(cache.load(h.conversation, &binding).unwrap(), Some(h));
    assert!(other.load(ConversationId(1), &binding).unwrap().is_none());
}
#[test]
fn concurrent_disable_always_wins_over_a_prepared_write() {
    let scratch = scratch_dir();
    let root = scratch.path().join("history");
    let cache = Cache::open(root.clone(), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    let queued = cache.prepare(&binding, &h).unwrap();
    let writer = cache.clone();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let ready = barrier.clone();
    let job = std::thread::spawn(move || {
        ready.wait();
        writer.commit(queued).unwrap()
    });
    barrier.wait();
    cache.configure(true, false).unwrap();
    job.join().unwrap();
    assert!(!root.exists());
    let stale = {
        cache.configure(true, true).unwrap();
        cache.prepare(&binding, &h).unwrap()
    };
    cache.configure(true, false).unwrap();
    cache.configure(true, true).unwrap();
    assert!(!cache.commit(stale).unwrap());
    assert!(!root.exists());
}
#[test]
fn empty_committed_history_is_distinct_from_an_absent_cache() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut h = History::new(ConversationId(1));
    h.set_completeness(Completeness::Complete).unwrap();
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    assert_eq!(cache.load(h.conversation, &binding).unwrap(), Some(h));
}
#[test]
fn cached_completeness_never_claims_fresh_native_completeness_after_an_attachment_gap() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut h = History::new(ConversationId(1));
    h.apply(Update::UpsertItem(entry(1))).unwrap();
    h.set_completeness(Completeness::Complete).unwrap();
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    let historical = cache.load(h.conversation, &binding).unwrap().unwrap();
    assert_eq!(historical.completeness, Completeness::Complete);
    let mut attachments = super::super::history::Attachments::default();
    attachments.attach(1, h.conversation);
    attachments.hydrate_cached(1, historical).unwrap();
    assert_eq!(
        attachments.get(1).unwrap().completeness,
        Completeness::Incomplete(IncompleteReason::Disconnected)
    );
    assert_eq!(attachments.get(1).unwrap().entries, vec![entry(1)]);
}
#[test]
fn reattached_cached_revision_can_be_updated_and_committed_immediately() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut initial = History::new(ConversationId(1));
    initial.apply(Update::UpsertItem(entry(1))).unwrap();
    initial.revision = 100;
    cache
        .commit(cache.prepare(&binding, &initial).unwrap())
        .unwrap();
    let loaded = cache.load(initial.conversation, &binding).unwrap().unwrap();
    let mut attachments = super::super::history::Attachments::default();
    attachments.attach(1, initial.conversation);
    attachments.hydrate_cached(1, loaded).unwrap();
    attachments.notification(initial.conversation, &binding.session_id, "item/completed", &serde_json::json!({"threadId": binding.session_id, "turnId":"turn", "item":{"id":"2", "type":"agentMessage", "text":"new synthetic response"}})).unwrap();
    let updated = attachments.get(1).unwrap();
    assert!(
        cache
            .commit(cache.prepare(&binding, updated).unwrap())
            .unwrap()
    );
    assert_eq!(
        cache
            .load(initial.conversation, &binding)
            .unwrap()
            .unwrap()
            .entries
            .len(),
        2
    );
}
#[test]
fn equal_revision_from_competing_attachment_cannot_replace_a_committed_snapshot() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut attachments = super::super::history::Attachments::default();
    for attachment in [1, 2] {
        attachments.attach(attachment, ConversationId(1));
        attachments
            .hydrate_native(attachment, History::new(ConversationId(1)))
            .unwrap();
    }
    let mut first = attachments.get(1).unwrap().clone();
    first.apply(Update::UpsertItem(entry(1))).unwrap();
    let mut stale = attachments.get(2).unwrap().clone();
    stale.apply(Update::UpsertItem(entry(2))).unwrap();
    assert_eq!(first.revision, stale.revision);
    let queued = cache.prepare(&binding, &stale).unwrap();
    cache
        .commit(cache.prepare(&binding, &first).unwrap())
        .unwrap();
    assert!(!cache.commit(queued).unwrap());
    assert_eq!(
        cache
            .load(first.conversation, &binding)
            .unwrap()
            .unwrap()
            .entries,
        first.entries
    );
}
#[test]
fn exhausted_manifest_revision_is_rejected_before_attachment_hydration() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let h = History::new(ConversationId(1));
    cache.commit(cache.prepare(&binding, &h).unwrap()).unwrap();
    let path = cache.root.join("1/manifest.json");
    let mut manifest: Manifest = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    manifest.revision = u64::MAX;
    std::fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(cache.load(h.conversation, &binding).is_err());
}
#[test]
fn exhausted_projection_cannot_be_prepared_as_an_unloadable_cache_revision() {
    let scratch = scratch_dir();
    let cache = Cache::open(scratch.path().join("history"), true, true).unwrap();
    let binding = crate::conversation::tests::conversation().binding;
    let mut h = History::new(ConversationId(1));
    h.revision = u64::MAX;
    let error = match cache.prepare(&binding, &h) {
        Err(error) => error,
        Ok(_) => panic!("exhausted revision admitted"),
    };
    assert_eq!(
        error.downcast_ref::<IncompleteReason>(),
        Some(&IncompleteReason::RevisionExhausted)
    );
}
