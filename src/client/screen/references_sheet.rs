//! The References view in the side panel: the project's screenshots and design files, kept in
//! its main checkout so agents in every worktree find the same set (`crate::references`). You
//! paste a clipboard image or add a file by its path, open one in the system's viewer, copy its
//! path, or remove it (asked first). Every file operation runs in a background task.

use super::*;
use crate::client::files::human_size;
use crate::client::menu::Act;
use crate::client::{Bg, Btn, PromptKind, View};
use crate::references::{self as refs, Listing, Reference};

/// Rows under the list: a gap and the hints.
const FOOT_ROWS: u16 = 2;
const HINTS: [(&str, &str); 6] = [("Enter", "open"), ("p", "paste image"), ("n", "add file"), ("d", "remove"), ("y", "copy path"), ("Esc", "close")];
/// With nothing listed, only the ways to add one.
const EMPTY_HINTS: [(&str, &str); 3] = [("p", "paste image"), ("n", "add file"), ("Esc", "close")];
/// Below this width a row shows its name and age only.
const FULL_ROW_FROM: u16 = 44;
const EMPTY: &str = "No references yet · paste an image or add a file";
/// A paste that is no file to add: the pasted text is never repeated back, it may be a secret.
const PASTE_NOTICE: &str = "References has the keys: drop a file or paste an image to add it; Esc closes it, or click a pane to paste there";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReferencesSheet {
    /// The project's root: its main checkout, or the folder outside git.
    pub root: PathBuf,
    /// Where the session runs: a typed relative path starts here.
    pub dir: PathBuf,
    /// None while the folder is read.
    pub list: Option<Result<Listing, String>>,
    pub sel: usize,
    scroll: usize,
}

impl ReferencesSheet {
    pub(crate) fn new(root: PathBuf, dir: PathBuf) -> ReferencesSheet {
        ReferencesSheet { root, dir, list: None, sel: 0, scroll: 0 }
    }

    fn items(&self) -> &[Reference] {
        match &self.list {
            Some(Ok(l)) => &l.items,
            _ => &[],
        }
    }

    fn chosen(&self) -> Option<&Reference> {
        self.items().get(self.sel)
    }
}

/// A typed or pasted path as a file to add: quotes a terminal adds on a drop are taken off, and
/// a relative path starts at `dir`.
pub(in crate::client) fn typed_path(raw: &str, dir: &Path) -> Option<PathBuf> {
    let t = raw.trim();
    let t = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')).or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\''))).unwrap_or(t);
    if t.is_empty() {
        return None;
    }
    let p = PathBuf::from(t);
    Some(if p.is_absolute() { p } else { dir.join(p) })
}

impl App {
    /// `a`: the project's references, or closed again when they're open.
    pub(in crate::client) fn toggle_references_sheet(&mut self) {
        if matches!(self.view, Some(View::References(_))) {
            // Set aside by a click into a pane: the key hands it the keys back rather than closing it.
            if !self.view_has_keys() {
                self.view_set_aside = None;
                return;
            }
            self.view = None;
            return;
        }
        let dir = self.screen_target_dir();
        let root = self.references_root_guess(&dir);
        self.screen.cursor = None;
        self.mode = Mode::Normal;
        self.show_view(View::References(Box::new(ReferencesSheet::new(root.clone(), dir))));
        self.read_references(root);
    }

    /// The project's root as the snapshot knows it (the session's main checkout), found without
    /// touching the disk; the background read settles it.
    fn references_root_guess(&self, dir: &Path) -> PathBuf {
        let term = self.screen.cursor.or(self.focused()).and_then(|t| self.snap.terms.get(&t));
        term.and_then(|t| t.root.clone()).unwrap_or_else(|| dir.to_path_buf())
    }

    fn read_references(&self, asked: PathBuf) {
        self.spawn_bg(move || {
            let root = refs::project_root(&asked);
            let list = refs::of_project(&root);
            Bg::Then(Box::new(move |app: &mut App| app.references_arrived(&asked, root, list)))
        });
    }

    /// The listing for the view that asked about `asked`; `root` is where the project really is.
    pub(in crate::client) fn references_arrived(&mut self, asked: &Path, root: PathBuf, list: Result<Listing, String>) {
        if let Some(View::References(v)) = &mut self.view
            && v.root == asked
        {
            v.root = root;
            v.list = Some(list);
            v.sel = v.sel.min(v.items().len().saturating_sub(1));
        }
    }

    /// After an add or a remove, the open References view reads the folder again.
    fn references_reload(&self) {
        if let Some(View::References(v)) = &self.view {
            self.read_references(v.root.clone());
        }
    }

    /// Keys in the References view; returns false when it closes.
    pub(in crate::client) fn on_references_key(&mut self, v: &mut ReferencesSheet, k: &KeyEvent) -> bool {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc | KeyCode::Char('a') => return false,
            KeyCode::Char('j') => v.sel = (v.sel + 1).min(v.items().len().saturating_sub(1)),
            KeyCode::Char('k') => v.sel = v.sel.saturating_sub(1),
            KeyCode::Enter | KeyCode::Char('o') => self.open_reference(v),
            KeyCode::Char('p') => self.paste_reference(v.root.clone()),
            KeyCode::Char('v') if ctrl => self.paste_reference(v.root.clone()),
            KeyCode::Char('n') => self.mode = Mode::Prompt { kind: PromptKind::AddReference(v.root.clone(), v.dir.clone()), input: String::new() },
            KeyCode::Char('d') | KeyCode::Delete => self.ask_remove_reference(v),
            KeyCode::Char('y') => self.copy_reference_path(v),
            _ => {
                let n = v.items().len();
                Self::list_move(&mut v.sel, n, k);
            }
        }
        true
    }

    /// A paste while References has the keys stays here: an empty one (a terminal's paste of an
    /// image) adds the clipboard image, one line naming a file is a drop to add, and anything
    /// else only says what the keys do. Returns whether it took the paste.
    pub(in crate::client) fn paste_into_references(&mut self, text: &str) -> bool {
        if !self.view_has_keys() || !matches!(self.mode, Mode::Normal) {
            return false;
        }
        let Some(View::References(v)) = &self.view else { return false };
        let (root, dir) = (v.root.clone(), v.dir.clone());
        let text = text.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            self.paste_reference(root);
        } else if text.contains(['\r', '\n']) {
            self.notify(PASTE_NOTICE.into(), false);
        } else if let Some(src) = typed_path(text, &dir) {
            self.drop_reference(root, src);
        } else {
            self.notify(PASTE_NOTICE.into(), false);
        }
        true
    }

    /// A dropped path: added when it names a file (checked off the UI thread), else only the
    /// notice, so a pasted secret is neither copied nor shown.
    fn drop_reference(&mut self, root: PathBuf, src: PathBuf) {
        self.spawn_bg(move || {
            let added = if src.is_file() { Some(refs::add_file(&refs::project_root(&root), &src)) } else { None };
            Bg::Then(Box::new(move |app: &mut App| match added {
                Some(added) => app.reference_added(added),
                None => app.notify(PASTE_NOTICE.into(), false),
            }))
        });
    }

    fn reference_added(&mut self, added: Result<PathBuf, String>) {
        match added {
            Ok(to) => self.notify(format!("added {} to the references", folder_name(&to)), false),
            Err(e) => self.notify(e, true),
        }
        self.references_reload();
    }

    fn open_reference(&mut self, v: &ReferencesSheet) {
        if let Some(r) = v.chosen() {
            self.open_outside(r.path.clone(), r.name.clone());
        }
    }

    fn copy_reference_path(&mut self, v: &ReferencesSheet) {
        let Some(r) = v.chosen() else { return };
        let path = r.path.display().to_string();
        crate::client::copy::to_clipboard(&path);
        self.notify(format!("copied {path}"), false);
    }

    fn ask_remove_reference(&mut self, v: &ReferencesSheet) {
        if let Some(r) = v.chosen() {
            self.menu_act(Act::RemoveReference(v.root.clone(), r.path.clone(), r.name.clone()));
        }
    }

    /// The clipboard's image saved into the project's references as a dated screenshot.
    fn paste_reference(&mut self, root: PathBuf) {
        self.spawn_bg(move || {
            let saved = save_clipboard_image(&root);
            Bg::Then(Box::new(move |app: &mut App| {
                match saved {
                    Ok((name, w, h)) => app.notify(format!("added {name} ({w}×{h}) to the references"), false),
                    Err(e) => app.notify(e, true),
                }
                app.references_reload();
            }))
        });
    }

    /// Copy the file at the path typed into the add prompt.
    pub(in crate::client) fn add_reference(&mut self, root: PathBuf, dir: PathBuf, input: &str) {
        let Some(src) = typed_path(input, &dir) else { return };
        self.spawn_bg(move || {
            let added = refs::add_file(&refs::project_root(&root), &src);
            Bg::Then(Box::new(move |app: &mut App| app.reference_added(added)))
        });
    }

    /// The confirmed remove.
    pub(in crate::client) fn remove_reference(&mut self, root: PathBuf, path: PathBuf, name: String) {
        self.spawn_bg(move || {
            let removed = refs::remove(&root, &path);
            Bg::Then(Box::new(move |app: &mut App| {
                match removed {
                    Ok(()) => app.notify(format!("removed {name}"), false),
                    Err(e) => app.notify(e, true),
                }
                app.references_reload();
            }))
        });
    }
}

/// Read the clipboard's image, then make the folder and write it: no image, no folder.
fn save_clipboard_image(root: &Path) -> Result<(String, usize, usize), String> {
    let img = crate::client::clipboard_image()?;
    let dir = refs::ensure(&refs::project_root(root))?;
    let (path, file) = refs::create_screenshot(&dir, now_secs())?;
    if let Err(why) = crate::client::encode_png(file, img.width as u32, img.height as u32, &img.bytes) {
        return Err(match std::fs::remove_file(&path) {
            Ok(()) => format!("couldn't save the screenshot: {why}"),
            Err(e) => format!("couldn't save the screenshot: {why}; the partial file {} is left ({e})", path.display()),
        });
    }
    Ok((folder_name(&path), img.width, img.height))
}

/// One row's right side: kind, size and age, or the age alone when narrow.
fn row_meta(r: &Reference, full: bool) -> String {
    if full { format!("{:<6} {:>7} {:>4}", r.kind.label(), human_size(r.size), age(r.modified)) } else { age(r.modified) }
}

/// The References view inside the sheet's card; `bg` is the card's ground.
pub(in crate::client) fn draw_references(app: &mut App, buf: &mut Buffer, body: Rect, bg: Color, t: &Theme, v: &mut ReferencesSheet) {
    let c = Style::default().bg(bg);
    let list = Rect { x: body.x + 1, width: body.width.saturating_sub(2), height: body.height.saturating_sub(FOOT_ROWS), ..body };
    let note = |buf: &mut Buffer, segs: Vec<Seg>| put(buf, list.x + 1, list.y, &segs, list.right());
    match &v.list {
        None => {
            note(buf, vec![seg("reading…", c.fg(t.muted).add_modifier(Modifier::ITALIC))]);
        }
        Some(Err(e)) => {
            note(buf, vec![seg(format!("couldn't read: {e}"), c.fg(t.err))]);
        }
        Some(Ok(l)) if l.items.is_empty() && l.skipped == 0 => {
            note(buf, vec![seg(EMPTY, c.fg(t.muted))]);
            put(buf, list.x + 1, list.y + 2, &[seg(format!("Kept in {}/", refs::DIR), c.fg(t.muted))], list.right());
            put(buf, list.x + 1, list.y + 3, &[seg("Agents find them through MCP.", c.fg(t.muted))], list.right());
            return status_bar(buf, body, t, &EMPTY_HINTS, &[]);
        }
        Some(Ok(l)) => {
            let skipped = skipped_note(l);
            let rows = Rect { height: list.height.saturating_sub(u16::from(skipped.is_some())), ..list };
            if let Some(words) = skipped {
                put(buf, list.x + 1, rows.bottom(), &[seg(words, c.fg(t.err))], list.right());
            }
            draw_rows(app, buf, rows, bg, t, v);
        }
    }
    status_bar(buf, body, t, &HINTS, &[]);
}

/// What the listing left out, for the line under the rows.
fn skipped_note(l: &Listing) -> Option<String> {
    let first = l.first_skip.as_deref()?;
    let more = l.skipped.saturating_sub(1);
    Some(if more == 0 { format!("left out: {first}") } else { format!("left out: {first} (and {more} more)") })
}

fn draw_rows(app: &mut App, buf: &mut Buffer, list: Rect, bg: Color, t: &Theme, v: &mut ReferencesSheet) {
    let h = list.height as usize;
    v.scroll = v.scroll.min(v.sel).max((v.sel + 1).saturating_sub(h));
    let full = list.width >= FULL_ROW_FROM;
    let look = Look::of(&app.cfg.ui);
    for (i, r) in v.items().iter().enumerate().skip(v.scroll).take(h) {
        let rr = Rect { y: list.y + (i - v.scroll) as u16, height: 1, ..list };
        let sel = i == v.sel;
        let row_bg = if sel {
            let sb = t.sel_on(bg);
            row_pill(look, buf, rr.x, rr.y, rr.width, sb, bg);
            put(buf, rr.x + 1, rr.y, &[seg("▎", Style::default().fg(t.accent).bg(sb).add_modifier(Modifier::BOLD))], rr.right());
            sb
        } else if hovered(app, rr) {
            fill(buf, rr, t.hov);
            t.hov
        } else {
            bg
        };
        app.hits.push((rr, Hit::Button(Btn::Row(i))));
        let st = Style::default().bg(row_bg);
        let meta = row_meta(r, full);
        let mw = meta.width() as u16;
        let name_room = rr.width.saturating_sub(mw + 5) as usize;
        let name_st = if sel { st.fg(t.strong).add_modifier(Modifier::BOLD) } else { st.fg(t.text) };
        put(buf, rr.x + 2, rr.y, &[seg(truncate(&r.name, name_room), name_st)], rr.right().saturating_sub(mw + 2));
        put(buf, rr.right().saturating_sub(mw + 1), rr.y, &[seg(meta, st.fg(t.muted))], rr.right());
    }
}

/// What follows the sheet's title: the project's folder.
pub(in crate::client) fn references_title(app: &App) -> String {
    let Some(View::References(v)) = &app.view else { return String::new() };
    format!("· {}", folder_name(&v.root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_the_listing_left_out_is_said_not_hidden() {
        assert_eq!(skipped_note(&Listing::default()), None);
        let one = Listing { skipped: 1, first_skip: Some("x.png is a link out of the folder".into()), ..Listing::default() };
        assert_eq!(skipped_note(&one).as_deref(), Some("left out: x.png is a link out of the folder"));
        let three = Listing { skipped: 3, ..one };
        assert_eq!(skipped_note(&three).as_deref(), Some("left out: x.png is a link out of the folder (and 2 more)"));
    }

    #[test]
    fn a_dropped_path_loses_its_quotes_and_a_relative_one_starts_where_you_are() {
        let here = std::env::temp_dir();
        assert_eq!(typed_path("  ", &here), None);
        assert_eq!(typed_path("shot.png", &here), Some(here.join("shot.png")));
        let abs = here.join("a b.png");
        assert_eq!(typed_path(&format!("\"{}\"", abs.display()), &here), Some(abs.clone()));
        assert_eq!(typed_path(&format!("'{}'", abs.display()), &here), Some(abs));
    }
}
