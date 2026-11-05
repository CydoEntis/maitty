use super::*;

const DAY0: u64 = 1_793_750_400; // 2026-11-04 00:00:00 UTC

fn usage(tokens_in: u64, tokens_out: u64, cost: Option<f64>) -> Usage {
    Usage {
        tokens_in: Some(tokens_in),
        tokens_out: Some(tokens_out),
        cost,
        ..Default::default()
    }
}

fn observe(store: &mut UsageStore, now: u64, session: &str, usage: &Usage, subscription: bool) {
    store.observe(
        now,
        Reported {
            session,
            agent: "claude",
            project: "shop",
            usage,
            subscription,
        },
    );
}

fn week(store: &UsageStore, now: u64) -> UsageReport {
    store.report(UsagePeriod::Week, now, &HashMap::new())
}

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("maitty-usage-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("usage.json")
}

#[test]
fn days_are_utc_calendar_dates() {
    assert_eq!(day_string(0), "1970-01-01");
    assert_eq!(day_string(DAY0), "2026-11-04");
    assert_eq!(day_string(DAY0 - 1), "2026-11-03");
    // A leap day, and the day after it.
    assert_eq!(day_string(1_709_164_800), "2024-02-29");
    assert_eq!(day_string(1_709_251_200), "2024-03-01");
}

#[test]
fn running_totals_add_only_what_is_new() {
    let mut store = UsageStore::in_memory();
    observe(&mut store, DAY0, "s1", &usage(100, 10, Some(1.0)), false);
    observe(
        &mut store,
        DAY0 + 60,
        "s1",
        &usage(250, 40, Some(1.5)),
        false,
    );
    let report = week(&store, DAY0 + 60);
    assert_eq!(report.totals.tokens_in, Some(250));
    assert_eq!(report.totals.tokens_out, Some(40));
    assert!((report.totals.cost.unwrap() - 1.5).abs() < 1e-9);
}

#[test]
fn a_total_that_drops_is_a_new_session_counting_from_zero() {
    let mut store = UsageStore::in_memory();
    observe(&mut store, DAY0, "s1", &usage(500, 50, Some(2.0)), false);
    observe(&mut store, DAY0 + 1, "s1", &usage(20, 5, Some(0.25)), false);
    let totals = week(&store, DAY0 + 1).totals;
    assert_eq!((totals.tokens_in, totals.tokens_out), (Some(520), Some(55)));
    assert!((totals.cost.unwrap() - 2.25).abs() < 1e-9);
}

#[test]
fn a_session_running_past_midnight_is_split_by_day() {
    let mut store = UsageStore::in_memory();
    observe(&mut store, DAY0 - 60, "s1", &usage(100, 10, None), false);
    observe(&mut store, DAY0 + 60, "s1", &usage(130, 14, None), false);
    let report = week(&store, DAY0 + 60);
    let days: Vec<_> = report
        .days
        .iter()
        .map(|d| (d.day.as_str(), d.figures.tokens_in))
        .collect();
    assert_eq!(days, [("2026-11-03", Some(100)), ("2026-11-04", Some(30))]);
    assert_eq!(report.session_count, 1);
    assert_eq!(report.sessions[0].first_day, "2026-11-03");
    assert_eq!(report.sessions[0].last_day, "2026-11-04");
}

#[test]
fn a_subscription_session_keeps_no_cost_even_when_the_provider_sends_one() {
    let mut store = UsageStore::in_memory();
    observe(&mut store, DAY0, "sub", &usage(100, 10, Some(3.0)), true);
    // The plan limits may be missing from a later report; the session stays a subscription.
    observe(
        &mut store,
        DAY0 + 5,
        "sub",
        &usage(200, 20, Some(4.0)),
        false,
    );
    observe(&mut store, DAY0 + 6, "api", &usage(10, 1, Some(0.5)), false);
    let report = week(&store, DAY0 + 6);
    let by = |id: &str| report.sessions.iter().find(|s| s.session == id).unwrap();
    assert_eq!(by("sub").figures.cost, None);
    assert_eq!(by("sub").figures.tokens_in, Some(200));
    assert_eq!(by("api").figures.cost, Some(0.5));
    assert_eq!(report.totals.cost, Some(0.5));
    assert_eq!(report.sessions_without_cost, 1);
}

#[test]
fn nothing_reported_stays_none_not_zero() {
    let mut store = UsageStore::in_memory();
    store.turn(DAY0, "codex-1", "codex", "api", 90);
    observe(
        &mut store,
        DAY0,
        "ctx-only",
        &Usage {
            context: Some(40.0),
            ..Default::default()
        },
        false,
    );
    let report = week(&store, DAY0);
    assert_eq!(report.session_count, 1, "a context bar alone is no usage");
    let totals = report.totals;
    assert_eq!(
        (totals.tokens_in, totals.tokens_out, totals.cost),
        (None, None, None)
    );
    assert_eq!((totals.turns, totals.working_secs), (1, 90));
}

#[test]
fn periods_cover_today_and_the_days_before() {
    let mut store = UsageStore::in_memory();
    for (days_ago, id) in [(0, "today"), (3, "three"), (10, "ten"), (40, "forty")] {
        observe(
            &mut store,
            DAY0 - days_ago * SECONDS_PER_DAY,
            id,
            &usage(1, 1, None),
            false,
        );
    }
    let count = |period| store.report(period, DAY0, &HashMap::new()).session_count;
    assert_eq!(count(UsagePeriod::Day), 1);
    assert_eq!(count(UsagePeriod::Week), 2);
    assert_eq!(count(UsagePeriod::Month), 3);
}

#[test]
fn sessions_name_the_open_pane_and_sort_by_cost_then_time() {
    let mut store = UsageStore::in_memory();
    observe(&mut store, DAY0, "cheap", &usage(1, 1, Some(0.1)), false);
    observe(&mut store, DAY0, "dear", &usage(1, 1, Some(9.0)), false);
    store.turn(DAY0, "nocost", "codex", "api", 500);
    let open = HashMap::from([("dear".to_string(), 7u32)]);
    let report = store.report(UsagePeriod::Day, DAY0, &open);
    let order: Vec<_> = report.sessions.iter().map(|s| s.session.as_str()).collect();
    assert_eq!(order, ["dear", "cheap", "nocost"]);
    assert_eq!(report.sessions[0].term, Some(7));
    assert_eq!(report.sessions[1].term, None);
}

#[test]
fn days_past_retention_are_pruned() {
    let mut store = UsageStore::in_memory();
    let old = DAY0 - RETENTION_DAYS * SECONDS_PER_DAY;
    observe(&mut store, old, "old", &usage(5, 5, None), false);
    observe(
        &mut store,
        old + SECONDS_PER_DAY,
        "kept",
        &usage(5, 5, None),
        false,
    );
    store.prune(DAY0);
    assert_eq!(store.rows.len(), 1);
    assert!(store.cursors.contains_key("kept"));
    assert!(!store.cursors.contains_key("old"));
}

#[test]
fn a_restart_does_not_count_a_session_again() {
    let path = temp_file("restart");
    let mut store = UsageStore::open(path.clone(), DAY0);
    observe(&mut store, DAY0, "s1", &usage(100, 10, Some(1.0)), false);
    store.save_now(DAY0);
    let mut again = UsageStore::open(path.clone(), DAY0);
    observe(
        &mut again,
        DAY0 + 30,
        "s1",
        &usage(130, 12, Some(1.25)),
        false,
    );
    let totals = week(&again, DAY0 + 30).totals;
    assert_eq!((totals.tokens_in, totals.tokens_out), (Some(130), Some(12)));
    assert!((totals.cost.unwrap() - 1.25).abs() < 1e-9);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[cfg(unix)]
#[test]
fn the_file_is_private_and_leaves_no_temporary_behind() {
    use std::os::unix::fs::PermissionsExt;
    let path = temp_file("private");
    let mut store = UsageStore::open(path.clone(), DAY0);
    store.turn(DAY0, "s1", "claude", "shop", 5);
    store.save_now(DAY0);
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
    assert!(!path.with_extension("json.tmp").exists());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_missing_file_starts_empty() {
    let path = temp_file("missing");
    let store = UsageStore::open(path.clone(), DAY0);
    assert!(store.rows.is_empty());
    assert!(!path.with_extension("json.bak").exists());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_corrupt_file_is_kept_as_bak_and_the_store_starts_empty() {
    let path = temp_file("corrupt");
    std::fs::write(&path, b"{ not json").unwrap();
    let mut store = UsageStore::open(path.clone(), DAY0);
    assert!(store.rows.is_empty());
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    assert_eq!(std::fs::read(&bak).unwrap(), b"{ not json");
    // The daemon carries on and writes a good file.
    store.turn(DAY0, "s1", "claude", "shop", 5);
    store.save_now(DAY0);
    assert_eq!(UsageStore::open(path.clone(), DAY0).rows.len(), 1);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_file_from_a_newer_build_is_kept_untouched_as_bak() {
    let path = temp_file("newer");
    let newer = format!(
        r#"{{"version":{},"rows":[{{"shape":"unknown"}}]}}"#,
        FORMAT_VERSION + 1
    );
    std::fs::write(&path, &newer).unwrap();
    let store = UsageStore::open(path.clone(), DAY0);
    assert!(store.rows.is_empty());
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), newer);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn fields_added_later_or_missing_do_not_stop_the_file_loading() {
    let path = temp_file("fields");
    std::fs::write(
        &path,
        r#"{"version":1,"added_later":true,
            "rows":[{"day":"2026-11-04","session":"s1","agent":"claude","project":"shop","extra":1}],
            "cursors":[{"session":"s1","last_day":"2026-11-04","extra":2}]}"#,
    )
    .unwrap();
    let store = UsageStore::open(path.clone(), DAY0);
    assert_eq!(store.rows.len(), 1);
    assert_eq!(week(&store, DAY0).totals.tokens_in, None);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
