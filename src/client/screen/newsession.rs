//! New sessions: leader `s` starts one straight away with the defaults in Settings ›
//! Sessions (the task is typed in the session); `c` asks first, in a small dialog
//! with an Agent, a Where and a Project row. Neither takes a goal.

use super::*;

/// The dialog's rows, top to bottom.
const ROW_AGENT: u8 = 0;
const ROW_WHERE: u8 = 1;
const ROW_PROJECT: u8 = 2;
const NS_ROWS: u8 = 3;

/// The two places a session can open; the same words as `sessions.where`.
pub(in crate::client) const WHERE_WORKTREE: usize = 0;
const WHERE_FOLDER: usize = 1;

const AGENT_LAST_USED: &str = "last used";
const AGENT_FALLBACK: &str = "claude";
const DIALOG_WIDTH: u16 = 76;
const DIALOG_HEIGHT: u16 = 14;
const SUMMARY_LINES: usize = 2;
const BUTTON_ROW: u16 = 11;
const LABEL_COL: u16 = 4;
const OPTION_COL: u16 = 14;

/// The new-session dialog: which row is chosen and what each row shows.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct NewSess {
    pub row: u8,
    /// Index into `ns_agents`.
    pub agent: usize,
    /// `WHERE_WORKTREE` or `WHERE_FOLDER`.
    pub place: usize,
    /// Index into the project list.
    pub proj: usize,
}

/// The agents the dialog offers: the configured quick agents and the built-in three, with no
/// shell, recipe or preset (those start from Menu › Shell here and the search).
pub(in crate::client) fn ns_agents(app: &App) -> Vec<String> {
    np_agents(app, 0).into_iter().filter(|a| a != "shell" && !a.starts_with('⚙') && !a.starts_with('★')).collect()
}

impl App {
    /// The agent `s` starts: `sessions.agent`, where "last used" is the last one started
    /// from Seshi (kept in `seshi-ui.json`) and claude before there is one.
    pub(in crate::client) fn ns_default_agent(&self) -> String {
        let agents = ns_agents(self);
        let pick = match self.cfg.sessions.agent.as_str() {
            AGENT_LAST_USED => self.screen.saved.last_agent.as_str(),
            named => named,
        };
        if agents.iter().any(|a| a == pick) { pick.to_string() } else { AGENT_FALLBACK.to_string() }
    }

    /// Where `s` opens: the project the sidebar cursor is on, else the one you are in.
    pub(in crate::client) fn ns_default_project(&self) -> usize {
        let model = self.screen_model();
        let key = self.screen.cursor.and_then(|c| model.iter().find(|p| p.sessions().any(|s| s.term == c)).map(|p| p.key.clone())).or(self.screen.proj.clone());
        model.iter().position(|p| Some(&p.key) == key.as_ref()).unwrap_or(0)
    }

    pub(in crate::client) fn ns_default_place(&self) -> usize {
        if self.cfg.sessions.place == crate::config::NEW_WHERE[WHERE_FOLDER] { WHERE_FOLDER } else { WHERE_WORKTREE }
    }

    /// `s`: a session now, with the defaults.
    pub(in crate::client) fn screen_start_session(&mut self, proj: Option<usize>) {
        let proj = proj.unwrap_or_else(|| self.ns_default_project());
        let agent = self.ns_default_agent();
        self.screen_start_agent(proj, &agent, self.ns_default_place() == WHERE_WORKTREE);
    }

    /// `c`: the dialog, filled with the defaults.
    pub(in crate::client) fn screen_open_new_session(&mut self, proj: Option<usize>) {
        let proj = proj.unwrap_or_else(|| self.ns_default_project());
        let agent = self.ns_default_agent();
        let agent = ns_agents(self).iter().position(|a| *a == agent).unwrap_or(0);
        self.mode = Mode::NewSession(Box::new(NewSess { row: ROW_AGENT, agent, place: self.ns_default_place(), proj }));
    }

    /// Start `agent` in project number `proj`: in its own new worktree when asked for one and
    /// the project is a git repo, else in the project's folder. Remembers the agent as the
    /// last used.
    pub(in crate::client) fn screen_start_agent(&mut self, proj: usize, agent: &str, worktree: bool) {
        let Some(p) = self.screen_model().get(proj).cloned() else {
            self.screen_open_finder();
            return;
        };
        let main = p.wts.iter().find(|w| w.main).map(|w| w.path.clone()).unwrap_or_else(|| p.path.clone());
        let cmd = np_command(self, agent, 0, "");
        if self.screen.saved.last_agent != agent {
            self.screen.saved.last_agent = agent.to_string();
            self.screen.save();
        }
        self.screen.cursor = None;
        if worktree && p.git {
            self.screen_new_worktree(&p, cmd, false, None);
        } else {
            self.screen_new_session(main, cmd, false);
        }
    }

    fn ns_go(&mut self, ns: &NewSess) {
        let agents = ns_agents(self);
        let agent = agents.get(ns.agent).cloned().unwrap_or_else(|| AGENT_FALLBACK.into());
        self.screen_start_agent(ns.proj, &agent, ns.place == WHERE_WORKTREE);
    }

    /// Set one row's option (a click on it, or the arrows).
    pub(in crate::client) fn ns_pick(&mut self, row: u8, i: usize) {
        if let Mode::NewSession(ns) = &mut self.mode {
            ns.row = row;
            match row {
                ROW_AGENT => ns.agent = i,
                ROW_WHERE => ns.place = i,
                _ => ns.proj = i,
            }
        }
    }

    pub(in crate::client) fn ns_start(&mut self) {
        if let Mode::NewSession(ns) = &self.mode {
            let ns = (**ns).clone();
            self.ns_go(&ns);
        }
    }

    pub(in crate::client) fn on_new_session_key(&mut self, mut ns: NewSess, k: &KeyEvent) {
        let nproj = self.screen_model().len();
        let counts = [ns_agents(self).len(), crate::config::NEW_WHERE.len(), nproj];
        let cyc = |v: usize, d: i32, n: usize| (v as i32 + d).rem_euclid(n.max(1) as i32) as usize;
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Enter => return self.ns_go(&ns),
            KeyCode::Down | KeyCode::Tab => ns.row = (ns.row + 1).min(NS_ROWS - 1),
            KeyCode::Up | KeyCode::BackTab => ns.row = ns.row.saturating_sub(1),
            KeyCode::Left | KeyCode::Right => {
                let d = if k.code == KeyCode::Right { 1 } else { -1 };
                match ns.row {
                    ROW_AGENT => ns.agent = cyc(ns.agent, d, counts[0]),
                    ROW_WHERE => ns.place = cyc(ns.place, d, counts[1]),
                    _ => ns.proj = cyc(ns.proj, d, counts[2]),
                }
            }
            _ => {}
        }
        self.mode = Mode::NewSession(Box::new(ns));
    }
}

/// The one line saying what Start will do.
pub(in crate::client) fn ns_summary(agent: &str, worktree: bool, project: &str) -> String {
    format!("Opens {agent} {} {project}. You type the task there.", if worktree { "in a new worktree of" } else { "in the folder of" })
}

pub(in crate::client) fn draw_new_session(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, ns: &NewSess) {
    let model = app.screen_model();
    let agents = ns_agents(app);
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    let r = panel(app, buf, area, DIALOG_WIDTH, DIALOG_HEIGHT, "New session", &[], t);
    let card_bg = Style::default().bg(t.card);
    let projs: Vec<String> = model.iter().map(|p| p.name.clone()).collect();
    let rows: [(&str, Vec<String>, usize); 2] = [
        ("Agent", agents.clone(), ns.agent),
        ("Where", crate::config::NEW_WHERE.iter().map(|s| s.to_string()).collect(), ns.place),
    ];
    for (i, (label, opts, cur)) in rows.iter().enumerate() {
        let y = r.y + 2 + i as u16 * 2;
        let on = ns.row as usize == i;
        let bg = if on { t.sel_on(t.card) } else { t.card };
        let st = Style::default().bg(bg);
        fill(buf, Rect { x: r.x + 2, y, width: r.width.saturating_sub(4), height: 1 }, bg);
        if on {
            put(buf, r.x + 2, y, &[seg("›", st.fg(t.accent).add_modifier(Modifier::BOLD))], r.right());
        }
        let ls = st.fg(if on { t.strong } else { t.muted });
        put(buf, r.x + LABEL_COL, y, &[seg(*label, if on { ls.add_modifier(Modifier::BOLD) } else { ls })], r.right());
        let mut x = r.x + OPTION_COL;
        for (j, o) in opts.iter().enumerate() {
            let txt = format!(" {o} ");
            let cr = Rect { x, y, width: txt.width() as u16, height: 1 };
            let sel = j == *cur;
            let cs = if sel { st.bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD) } else if hovered(app, cr) { st.bg(t.hov).fg(t.strong) } else { st.bg(t.btn).fg(t.text) };
            x = put(buf, x, y, &[seg(txt, cs)], r.right() - 2) + 1;
            hit(app, cr, ScreenHit::NsOpt(i as u8, j));
        }
    }
    // The project is one value you step through: `‹ seshi ›   1 of 3`.
    let y = r.y + 2 + ROW_PROJECT as u16 * 2;
    let on = ns.row == ROW_PROJECT;
    let bg = if on { t.sel_on(t.card) } else { t.card };
    let st = Style::default().bg(bg);
    fill(buf, Rect { x: r.x + 2, y, width: r.width.saturating_sub(4), height: 1 }, bg);
    if on {
        put(buf, r.x + 2, y, &[seg("›", st.fg(t.accent).add_modifier(Modifier::BOLD))], r.right());
    }
    let ls = st.fg(if on { t.strong } else { t.muted });
    put(buf, r.x + LABEL_COL, y, &[seg("Project", if on { ls.add_modifier(Modifier::BOLD) } else { ls })], r.right());
    let proj_name = projs.get(ns.proj).cloned().unwrap_or_else(|| "none yet".into());
    let segs = [
        seg("‹ ", st.fg(t.muted)),
        seg(format!(" {proj_name} "), st.bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD)),
        seg(" ›", st.fg(t.muted)),
        seg(format!("   {} of {}", ns.proj.min(projs.len().saturating_sub(1)) + 1, projs.len().max(1)), st.fg(t.muted)),
    ];
    let end = put(buf, r.x + OPTION_COL, y, &segs, r.right() - 2);
    hit(app, Rect { x: r.x + OPTION_COL, y, width: end.saturating_sub(r.x + OPTION_COL), height: 1 }, ScreenHit::NsOpt(ROW_PROJECT, (ns.proj + 1) % projs.len().max(1)));

    let agent = agents.get(ns.agent).map(String::as_str).unwrap_or(AGENT_FALLBACK);
    let git = model.get(ns.proj).is_some_and(|p| p.git);
    let line = ns_summary(agent, ns.place == WHERE_WORKTREE && git, &proj_name);
    let room = r.width.saturating_sub(LABEL_COL + 2) as usize;
    for (i, l) in crate::client::views::wrap(&line, room).into_iter().take(SUMMARY_LINES).enumerate() {
        put(buf, r.x + LABEL_COL, r.y + 8 + i as u16, &[seg(l, card_bg.fg(t.text))], r.right() - 2);
    }
    let gx = btn(app, buf, r.x + LABEL_COL, r.y + BUTTON_ROW, "Start", "Enter", BtnKind::Primary, ScreenHit::NsStart, r.right());
    let cx = btn(app, buf, gx + 1, r.y + BUTTON_ROW, "Cancel", "Esc", BtnKind::Normal, ScreenHit::Close, r.right());
    let keys = hints(t, &[("↑↓", "rows"), ("←→", "pick")]);
    if cx + 2 + segs_width(&keys) <= r.right() {
        put(buf, r.right().saturating_sub(segs_width(&keys) + 2), r.y + BUTTON_ROW, &keys, r.right());
    }
}
