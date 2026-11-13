//! The clean-up list: what comes ticked, and what Remove asks before anything goes.
//! `MAITTY_SHOW=1 cargo test screen::tidy::tests -- --nocapture` prints the frame.
use super::*;
use crate::client::menu::Act;
use crate::client::reference_fixture::{self, Stage};
use crate::tidy::Standing;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn standing(dirty: usize, merged: bool, inactive_days: Option<u32>) -> Standing {
    Standing {
        dirty,
        branch: Some("task".into()),
        merged,
        last_commit: "5 weeks ago".into(),
        inactive_days,
    }
}

fn row(name: &str, standing: Standing) -> TidyRow {
    TidyRow {
        path: PathBuf::from(format!("/code/app-worktrees/{name}")),
        name: name.into(),
        branch: name.into(),
        sessions: 0,
        ticked: comes_ticked(0, &standing),
        standing,
    }
}

fn list(rows: Vec<TidyRow>) -> TidyView {
    TidyView {
        repo: PathBuf::from("/code/app"),
        name: "app".into(),
        rows: Some(rows),
        sel: 0,
    }
}

fn frame(app: &mut App) -> Vec<String> {
    let (w, h) = (160, 45);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f))
        .expect("a frame");
    let buf = term.backend().buffer().clone();
    let rows: Vec<String> = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        rows.iter().for_each(|r| println!("{r}"));
    }
    rows
}

#[test]
fn an_inactive_worktree_comes_ticked_and_says_for_how_long() {
    let stale = row("old-spike", standing(0, false, Some(30)));
    assert!(stale.ticked);
    assert!(stale.why().starts_with("inactive 30 days · not merged anywhere"), "{}", stale.why());
    assert!(!row("fresh", standing(0, false, None)).ticked, "unmerged work alone is never ticked");
}

#[test]
fn an_inactive_worktree_with_uncommitted_changes_is_not_ticked() {
    assert!(!row("old-edit", standing(2, false, Some(30))).ticked);
}

#[test]
fn removing_a_ticked_inactive_worktree_with_unmerged_commits_still_asks() {
    let mut app = reference_fixture::world(Stage::None);
    app.mode = Mode::Tidy(Box::new(list(vec![row(
        "old-spike",
        standing(0, false, Some(30)),
    )])));
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let Mode::Confirm(asked) = &app.mode else {
        panic!("Remove asks first");
    };
    assert!(asked.note.contains("isn't merged"), "{}", asked.note);
    assert_eq!(asked.list, ["old-spike (not merged)"]);
    assert!(
        matches!(&asked.act, Act::Tidy(items) if items == &[(PathBuf::from("/code/app-worktrees/old-spike"), false)]),
        "its folder goes without force, so its branch and commits stay"
    );
}

#[test]
fn the_list_shows_an_inactive_worktree_ticked_with_its_reason() {
    let mut app = reference_fixture::world(Stage::None);
    app.mode = Mode::Tidy(Box::new(list(vec![
        row("old-spike", standing(0, false, Some(30))),
        row("live-work", standing(1, false, None)),
    ])));
    let rows = frame(&mut app);
    let stale = rows
        .iter()
        .find(|r| r.contains("old-spike"))
        .expect("the inactive row is listed");
    assert!(stale.contains("✓ old-spike") && stale.contains("inactive 30 days"), "{stale}");
    let live = rows.iter().find(|r| r.contains("live-work")).expect("listed");
    assert!(live.contains("○ live-work") && !live.contains("inactive"), "{live}");
}

fn detached(standing: Standing) -> Standing {
    Standing {
        branch: None,
        ..standing
    }
}

#[test]
fn an_inactive_detached_worktree_with_unmerged_commits_is_not_ticked() {
    assert!(!row("old-detached", detached(standing(0, false, Some(30)))).ticked);
}

#[test]
fn removing_a_detached_worktree_ticked_by_hand_says_its_commits_are_lost() {
    let mut app = reference_fixture::world(Stage::None);
    let mut picked = row("old-detached", detached(standing(0, false, Some(30))));
    picked.ticked = true;
    app.mode = Mode::Tidy(Box::new(list(vec![picked])));
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let Mode::Confirm(asked) = &app.mode else {
        panic!("Remove asks first");
    };
    assert!(asked.note.contains("commits on no branch, which will be lost"), "{}", asked.note);
    assert!(!asked.note.contains("branch is kept"), "{}", asked.note);
    assert_eq!(asked.list, ["old-detached (commits on no branch)"]);
}

#[test]
fn ticking_all_picks_what_the_list_opens_with() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = vec![
        row("merged", standing(0, true, None)),
        row("old-spike", standing(0, false, Some(30))),
        row("old-detached", detached(standing(0, false, Some(30)))),
        row("live-work", standing(1, false, None)),
    ];
    let opened: Vec<bool> = rows.iter().map(|r| r.ticked).collect();
    let mut view = list(rows);
    view.rows.iter_mut().flatten().for_each(|r| r.ticked = false);
    app.mode = Mode::Tidy(Box::new(view));
    app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    let Mode::Tidy(v) = &app.mode else {
        panic!("the list stays open");
    };
    let ticked: Vec<bool> = v.rows.iter().flatten().map(|r| r.ticked).collect();
    assert_eq!(ticked, opened);
    assert_eq!(opened, [true, true, false, false]);
}
