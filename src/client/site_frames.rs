//! Real frames for the website: app states drawn by the real draw code into a `TestBackend`,
//! each written as a self-contained HTML fragment, with `frames.json` listing them. Runs only
//! on demand (see `docs/TESTING.md`):
//! `<APP>_SITE_FRAMES=<dir> cargo test export_site_frames -- --ignored`.
//!
//! Everything shown is made up: three projects (acme-store, api-gateway, docs-site), their
//! sessions in mixed states, and agent turns typed out as escape sequences. The clock is pinned
//! and motion is off, so every run writes the same bytes.

mod html;
mod output;

use super::screen::FIXED_NOW;
use super::v7_fixture::{entry, git, in_repo, now, term, workspace};
use super::*;
use crate::layout::{Dir, Node};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use std::path::Path;

const THEMES: [&str; 2] = ["night", "day"];
const MIN: u64 = 60;
const HOUR: u64 = 3600;

const CHECKOUT: TermId = 1;
const CART: TermId = 2;
const ORDERS: TermId = 3;
const RATE: TermId = 4;
const AUTH: TermId = 5;
const QUICKSTART: TermId = 6;
/// A session only the grid shows, so its tab holds four panes.
const PAYMENTS: TermId = 7;

const STORE_WS: WsId = 10;
const DOCS_WS: WsId = 30;

/// A frame for the site: its name, size, what it shows, and how to set it up.
struct Shot {
    name: &'static str,
    cols: u16,
    rows: u16,
    caption: &'static str,
    setup: fn(&mut App),
}

const SHOTS: &[Shot] = &[
    Shot { name: "hero", cols: 160, rows: 45, caption: "Agents across three projects; Claude waits for permission to run the tests.", setup: hero },
    Shot { name: "grid", cols: 160, rows: 45, caption: "Grid: four agents in one tab, each in its own colour.", setup: grid },
    Shot { name: "changes", cols: 160, rows: 45, caption: "Changes: review what an agent changed before you merge it.", setup: changes },
    Shot { name: "files", cols: 160, rows: 45, caption: "Files: the project's tree beside its README, drawn as markdown.", setup: files },
    Shot { name: "inbox", cols: 120, rows: 34, caption: "The Inbox: a question, a failed check and finished work, in the order they need you.", setup: inbox },
    Shot { name: "layout-floating", cols: 120, rows: 34, caption: "Floating: cards with room between them.", setup: floating },
    Shot { name: "layout-tiled", cols: 120, rows: 34, caption: "Tiled: cards edge to edge.", setup: tiled },
    Shot { name: "layout-compact", cols: 120, rows: 34, caption: "Compact: flat panes, the most room for output.", setup: compact },
    Shot { name: "layout-grid", cols: 120, rows: 34, caption: "Grid: each pane in its own colour.", setup: grid_look },
    Shot { name: "shortcut", cols: 120, rows: 34, caption: "Shortcut mode: one key moves, splits or opens; Esc goes back to typing.", setup: shortcut },
    Shot { name: "splash", cols: 120, rows: 34, caption: "The welcome screen: what happened while you were away.", setup: splash },
];

#[test]
#[ignore = "writes the website's frames; run on demand"]
fn export_site_frames() {
    let var = concat!(env!("APP_ENV"), "_SITE_FRAMES");
    let dir = PathBuf::from(std::env::var(var).unwrap_or_else(|_| panic!("set {var} to the folder to write the frames into")));
    std::fs::create_dir_all(&dir).expect("the frames folder");
    let mut listed = Vec::new();
    for shot in SHOTS {
        for theme in THEMES {
            let (page, app) = render(shot, theme);
            let name = format!("{}-{theme}", shot.name);
            std::fs::write(dir.join(format!("{name}.html")), page).expect("a frame file");
            listed.push(serde_json::json!({
                "name": name,
                "frame": shot.name,
                "cols": shot.cols,
                "rows": shot.rows,
                "theme": theme,
                "bg": html::hex(app.theme.bg),
                "caption": shot.caption,
            }));
        }
    }
    let json = serde_json::to_string_pretty(&listed).expect("the frame list");
    std::fs::write(dir.join("frames.json"), json + "\n").expect("frames.json");
}

/// `shot` in `theme` as HTML, and the app that drew it.
fn render(shot: &Shot, theme: &str) -> (String, App) {
    let mut app = world(theme);
    (shot.setup)(&mut app);
    let buf = draw(&mut app, shot.cols, shot.rows);
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        let a = buf.area;
        let text: String = (0..a.height).map(|y| (0..a.width).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_string() + "\n").collect();
        println!("===== {} {theme}\n{text}", shot.name);
    }
    (html::frame(&buf, &app.theme), app)
}

/// Draw once so the panes get their sizes, write each pane's output at its size, then draw
/// the frame that is kept.
fn draw(app: &mut App, w: u16, h: u16) -> Buffer {
    FIXED_NOW.with(|n| n.set(Some(now())));
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    app.screen_fresh();
    term.draw(|f| render::draw(app, f)).expect("a frame");
    app.sync_sizes();
    let ids: Vec<TermId> = app.snap.terms.keys().copied().collect();
    for id in ids {
        let (rows, cols) = app.parsers[&id].screen().size();
        app.parsers.insert(id, vt100::Parser::new(rows, cols, 0));
        let out = pane_output(id, app.snap.terms[&id].status, cols);
        app.feed(id, out.as_bytes());
    }
    app.screen_fresh();
    term.draw(|f| render::draw(app, f)).expect("a frame");
    term.backend().buffer().clone()
}

fn pane_output(id: TermId, status: Status, cols: u16) -> String {
    match id {
        CHECKOUT => output::claude_asks(cols),
        CART if status == Status::Working => output::codex_working(),
        CART => output::codex_done(),
        ORDERS => output::codex_emails(),
        RATE | PAYMENTS => output::gemini_done(),
        AUTH => output::opencode_idle(),
        _ => output::claude_done(),
    }
}

/// The site's world in `theme`: acme-store open on Claude asking to run the tests beside Codex
/// at work, api-gateway and docs-site below it.
fn world(theme: &str) -> App {
    FIXED_NOW.with(|n| n.set(Some(now())));
    let (tx, _rx) = mpsc::unbounded_channel();
    let (bg_tx, _bg_rx) = mpsc::unbounded_channel();
    let mut cfg = Config { theme: theme.into(), ..Default::default() };
    (cfg.ui.layout, cfg.ui.padding, cfg.ui.rounded, cfg.ui.motion) = ("floating".into(), "1".into(), crate::config::Rounding::Full, false);
    cfg.ui.splash_animation = "never".into();
    let mut app = App::new(cfg, tx, None, bg_tx);
    app.splash = false;
    let (store, gateway, docs) = (place("acme-store"), place("api-gateway"), place("docs-site"));
    let tree = |repo: &str, name: &str| place(&format!("{repo}-worktrees/{name}"));
    let (checkout_dir, cart_dir, orders_dir, rate_dir) =
        (tree("acme-store", "checkout-discounts"), tree("acme-store", "cart-tests"), tree("acme-store", "order-emails"), tree("api-gateway", "rate-limit"));

    let sessions = [
        in_repo(term(CHECKOUT, "checkout-discounts", "claude", Status::Blocked, &checkout_dir, now() - 40), &store, "checkout-discounts"),
        in_repo(term(CART, "cart-tests", "codex", Status::Working, &cart_dir, now() - 3 * MIN), &store, "cart-tests"),
        in_repo(term(ORDERS, "order-emails", "codex", Status::Working, &orders_dir, now() - MIN), &store, "order-emails"),
        in_repo(term(RATE, "rate-limit", "gemini", Status::Done, &rate_dir, now() - 4 * MIN), &gateway, "rate-limit"),
        in_repo(term(AUTH, "auth-refresh", "opencode", Status::Idle, &gateway, now() - 2 * HOUR), &gateway, "main"),
        in_repo(term(QUICKSTART, "quickstart", "claude", Status::Done, &docs, now() - 9 * MIN), &docs, "main"),
    ];
    let store_trees = [("main", &store), ("checkout-discounts", &checkout_dir), ("cart-tests", &cart_dir), ("order-emails", &orders_dir)]
        .map(|(branch, dir)| entry(dir, branch, dir == &store, "acme-store"))
        .to_vec();
    let gateway_trees = vec![entry(&gateway, "main", true, "api-gateway"), entry(&rate_dir, "rate-limit", false, "api-gateway")];
    let mut store_ws = workspace(STORE_WS, "acme-store", &store, &[CHECKOUT, ORDERS], Some(git(&store, "main", store_trees)), 0);
    store_ws.tabs[0].layout = split(&[CHECKOUT, CART], &[]);
    app.snap.workspaces = vec![
        store_ws,
        workspace(20, "api-gateway", &gateway, &[RATE, AUTH], Some(git(&gateway, "main", gateway_trees)), 1),
        workspace(DOCS_WS, "docs-site", &docs, &[QUICKSTART], Some(git(&docs, "main", vec![entry(&docs, "main", true, "docs-site")])), 3),
    ];
    app.snap.active_ws = Some(STORE_WS);
    for t in sessions {
        add(&mut app, t);
    }
    app.screen_sync();
    pin_fold_colors(&mut app);
    for (dir, lines) in [(&checkout_dir, (38, 6)), (&cart_dir, (54, 2)), (&orders_dir, (21, 0)), (&rate_dir, (42, 7))] {
        app.screen.lines_changed.insert(dir.clone(), lines);
    }
    app
}

/// Each project's −/+ button in the next colour of the palette, in place of the random one the
/// app picks the first time it draws it.
fn pin_fold_colors(app: &mut App) {
    let palette: Vec<_> = app.cfg.ui.workspace_colors.iter().filter_map(|c| crate::theme::parse_color(c)).collect();
    for (p, color) in app.screen_model().iter().zip(palette.iter().cycle()) {
        app.screen.fold_colors.insert(p.key.clone(), *color);
    }
}

/// A made-up folder under `~/code`, written with `/` on every platform so the frames read the
/// same wherever they are made. Nothing is there: nothing is read from it.
fn place(rel: &str) -> PathBuf {
    PathBuf::from(format!("~/code/{rel}"))
}

fn under(dir: &Path, rel: &str) -> PathBuf {
    PathBuf::from(format!("{}/{rel}", dir.display()))
}

fn add(app: &mut App, t: TermInfo) {
    app.parsers.insert(t.id, vt100::Parser::new(20, 80, 0));
    app.snap.terms.insert(t.id, t);
}

/// `top` side by side, and `bottom` side by side under them when there is a bottom row.
fn split(top: &[TermId], bottom: &[TermId]) -> Node {
    let mut layout = Node::Leaf(top[0]);
    for pair in top.windows(2) {
        layout.split(pair[0], Dir::Right, pair[1]);
    }
    for (above, below) in top.iter().zip(bottom) {
        layout.split(*above, Dir::Down, *below);
    }
    layout
}

/// acme-store's tabs become `layouts`, the first one open on its first pane.
fn store_tabs(app: &mut App, layouts: Vec<Node>) {
    let ws = app.snap.workspaces.iter_mut().find(|w| w.id == STORE_WS).expect("acme-store");
    ws.tabs = layouts.into_iter().zip(STORE_WS + 1..).map(|(layout, id)| TabInfo { id, name: String::new(), focus: layout.first_leaf(), layout }).collect();
    ws.active_tab = STORE_WS + 1;
    (app.screen.tabs, app.screen.last_focus) = (Vec::new(), None);
    app.screen_sync();
}

fn hero(_: &mut App) {}

fn floating(app: &mut App) {
    app.cfg.ui.layout = "floating".into();
}

fn tiled(app: &mut App) {
    app.cfg.ui.layout = "tiled".into();
}

fn compact(app: &mut App) {
    app.cfg.ui.layout = "compact".into();
}

fn grid_look(app: &mut App) {
    app.cfg.ui.layout = "grid".into();
}

/// Grid on a tab of four: Claude asking, Codex at work, Gemini and Codex done.
fn grid(app: &mut App) {
    grid_look(app);
    let store = app.snap.workspaces[0].cwd.clone();
    add(app, in_repo(term(PAYMENTS, "payment-webhooks", "gemini", Status::Done, &store, now() - 7 * MIN), &store, "main"));
    app.snap.terms.get_mut(&CART).expect("cart-tests").status = Status::Done;
    store_tabs(app, vec![split(&[CHECKOUT, ORDERS], &[PAYMENTS, CART])]);
}

fn shortcut(app: &mut App) {
    app.cfg.ui.which_key = false;
    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
}

fn splash(app: &mut App) {
    app.splash = true;
}

/// Changes open on checkout-discounts: three files, the total's diff showing.
fn changes(app: &mut App) {
    store_tabs(app, [CHECKOUT, CART, ORDERS].map(Node::Leaf).to_vec());
    app.sidebar = false;
    app.screen.saved.sheet_w.insert("changes".into(), 100);
    let info = &app.snap.terms[&CHECKOUT];
    let (dir, root) = (info.cwd.clone(), info.root.clone().unwrap_or_default());
    let changed = |path: &str, added, removed| tasks::Changed { path: path.into(), added, removed, untracked: false };
    let review = tasks::Review {
        task: tasks::TaskRow {
            ws: STORE_WS,
            name: "checkout-discounts".into(),
            branch: "checkout-discounts".into(),
            base: "main".into(),
            stage: tasks::Stage::Ready,
            summary: String::new(),
            dirty: 3,
            ahead: 0,
            agent: Some(CHECKOUT),
            dir: dir.clone(),
            root,
        },
        merge_base: String::new(),
        files: vec![changed("src/checkout/total.ts", 6, 2), changed("src/checkout/codes.ts", 14, 3), changed("src/checkout/total.test.ts", 18, 1)],
        sel: 0,
        diff: TOTAL_DIFF.lines().map(String::from).collect(),
        diff_sel: Some(0),
        scroll: 0,
        confirm: None,
    };
    app.show_view(View::Changes(Box::new(views::ChangesView {
        dir,
        review: Some(review),
        error: None,
        agent: "claude".into(),
        said: "Discount codes now come off the subtotal before shipping. Two new tests cover it.".into(),
        term: Some(CHECKOUT),
        ws: Some(STORE_WS),
        reviewed: Default::default(),
        linked: true,
        confirm: None,
    })));
}

const TOTAL_DIFF: &str = "\
@@ -24,9 +24,13 @@ export function total(cart, code) {
   const subtotal = sum(cart.items);
   const ship = subtotal < FREE_FROM ? SHIPPING : 0;
-  return subtotal + ship;
+  const off = code ? discount(subtotal, code) : 0;
+  if (off > subtotal) {
+    throw new RangeError(`${code} exceeds the total`);
+  }
+  return subtotal - off + ship;
 }

-export const SHIPPING = 499;
+export const SHIPPING = 495;
 export const FREE_FROM = 5000;
";

/// Files open on docs-site's README, its `docs` folder open. Nothing is read from disk: the
/// tree and the file are given.
fn files(app: &mut App) {
    app.snap.active_ws = Some(DOCS_WS);
    app.screen_sync();
    let root = app.snap.terms[&QUICKSTART].cwd.clone();
    let mut tree = views::FilesTree::new(root.clone(), Some("main".into()));
    tree.all = DOCS_TREE.iter().map(|(rel, is_dir)| file_node(&root, rel, *is_dir)).collect();
    tree.expanded.insert(under(&root, "docs"));
    tree.loading = false;
    tree.git = [("docs/quickstart.md", 'M'), ("docs/deploy.md", 'A')].map(|(rel, mark)| (rel.to_string(), mark)).into();
    tree.editing = [("docs/quickstart.md".to_string(), "claude".to_string())].into();
    let readme = under(&root, "README.md");
    tree.sel = tree.visible().iter().position(|(_, n)| n.path == readme).unwrap_or(0);
    tree.preview = Some((readme, README.lines().map(String::from).collect()));
    app.show_view(View::Files(Box::new(tree)));
}

fn file_node(root: &Path, rel: &str, is_dir: bool) -> views::FileNode {
    let path = under(root, rel);
    let label = rel.rsplit('/').next().unwrap_or(rel).to_string();
    views::FileNode { path, rel: rel.into(), label, depth: rel.matches('/').count(), is_dir }
}

/// docs-site's files, in the tree's order: folders first, then by name.
const DOCS_TREE: [(&str, bool); 10] = [
    ("docs", true),
    ("docs/api", true),
    ("docs/api/orders.md", false),
    ("docs/api/payments.md", false),
    ("docs/deploy.md", false),
    ("docs/index.md", false),
    ("docs/quickstart.md", false),
    ("package.json", false),
    ("README.md", false),
    ("site.config.ts", false),
];

const README: &str = "\
# docs-site

The documentation for **Acme Store**, built as a static site.

## Getting started

1. Install the dependencies with `pnpm install`.
2. Run `pnpm dev` and open the local preview.
3. Edit any page in `docs/`; the preview reloads as you save.

## Writing pages

- Keep one topic per page.
- Start with what the reader can do, then how.
- Link to the API reference instead of copying it.

> Pages under `docs/drafts/` are not published.

```sh
pnpm build && pnpm preview
```
";

/// The Inbox: auth-refresh asks a question, order-emails' check failed, two sessions finished.
fn inbox(app: &mut App) {
    let auth = app.snap.terms.get_mut(&AUTH).expect("auth-refresh");
    (auth.status, auth.since) = (Status::Blocked, now() - 2 * MIN);
    app.snap.questions = vec![HumanQuestion {
        id: 1,
        term: AUTH,
        text: "Keep refresh tokens in Redis or in Postgres?".into(),
        options: vec!["Redis".into(), "Postgres".into()],
    }];
    let orders = app.snap.terms.get_mut(&ORDERS).expect("order-emails");
    (orders.status, orders.since) = (Status::Done, now() - 6 * MIN);
    orders.checks = vec![CheckInfo {
        name: "test".into(),
        command: "pnpm test".into(),
        cwd: orders.cwd.clone(),
        turn: 2,
        started_at: now() - 5 * MIN,
        duration_ms: 9_400,
        after_turn_ended_at: now() - 6 * MIN,
        state: CheckState::Failed,
        exit_code: Some(1),
        line_count: 18,
        first_failing_line: "✗ orders/events.test.ts › mails the buyer once".into(),
        ..Default::default()
    }];
    app.screen_sync();
    app.act(Action::Jump);
}

mod tests {
    use super::*;

    #[test]
    fn every_frame_draws_in_both_themes_without_nerd_font_glyphs() {
        for shot in SHOTS {
            for theme in THEMES {
                let (page, _) = render(shot, theme);
                assert_eq!(page.matches('\n').count(), shot.rows as usize, "{} {theme}: one line a row", shot.name);
                assert!(!page.chars().any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)), "{} {theme}: no private-use glyphs", shot.name);            }
        }
    }
}
