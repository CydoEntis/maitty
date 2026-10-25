//! A project's references: screenshots and design files in `.<app>/references/` at the repo's
//! main checkout, so every agent in every worktree of the repo sees the same set. The client's
//! References view adds, lists and removes them; `<app> mcp` lists them for agents. Nothing
//! here reads or returns a path outside that folder.

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The folder, relative to the project's root.
pub const DIR: &str = concat!(".", env!("APP_COMMAND"), "/references");
/// The line in the repo's `info/exclude` that keeps the folder out of git.
const EXCLUDE_LINE: &str = concat!(".", env!("APP_COMMAND"), "/references/");
/// How deep the listing walks into folders you made inside it.
const MAX_DEPTH: usize = 3;
/// The most it lists; a folder holding more is not a set of references.
const MAX_LISTED: usize = 500;
/// The biggest file it copies in: a design export, not a video.
pub const MAX_ADD_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image,
    Pdf,
    Design,
    Text,
    Other,
}

impl Kind {
    pub fn of(name: &str) -> Kind {
        let ext = Path::new(name).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "avif" | "ico" | "tif" | "tiff" | "heic" => Kind::Image,
            "pdf" => Kind::Pdf,
            "fig" | "sketch" | "xd" | "psd" | "ai" | "penpot" | "afdesign" => Kind::Design,
            "md" | "txt" | "html" | "css" | "json" | "yaml" | "yml" => Kind::Text,
            _ => Kind::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Image => "image",
            Kind::Pdf => "pdf",
            Kind::Design => "design",
            Kind::Text => "text",
            Kind::Other => "file",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub path: PathBuf,
    /// Its path inside the folder, with `/` between parts.
    pub name: String,
    pub kind: Kind,
    pub size: u64,
    /// Last changed (unix seconds).
    pub modified: u64,
}

/// The project `dir` belongs to: the repo's main checkout (never a linked worktree, so every
/// agent shares one set), else the folder itself.
pub fn project_root(dir: &Path) -> PathBuf {
    crate::gitfs::head(dir).map(|h| h.main_root).unwrap_or_else(|| dir.to_path_buf())
}

/// The references folder of the project at `root`.
pub fn folder(root: &Path) -> PathBuf {
    root.join(DIR)
}

/// The references of the project at `root`, newest first.
pub fn of_project(root: &Path) -> Result<Vec<Reference>, String> {
    list(&checked_folder(root)?)
}

/// `folder(root)`, refused when it leads out of the project through a link (its own, or
/// `.<app>`'s).
fn checked_folder(root: &Path) -> Result<PathBuf, String> {
    let dir = folder(root);
    match (root.canonicalize(), dir.canonicalize()) {
        (Ok(r), Ok(d)) if !d.starts_with(&r) => Err(format!("{DIR} leads out of {}", root.display())),
        _ => Ok(dir),
    }
}

/// Every reference in `folder`, newest first. No folder yet is an empty list. Links are
/// listed only when they point inside the folder.
pub fn list(folder: &Path) -> Result<Vec<Reference>, String> {
    let meta = match std::fs::symlink_metadata(folder) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("can't read {}: {e}", folder.display())),
    };
    if !meta.is_dir() {
        return Err(format!("{} is not a folder", folder.display()));
    }
    let real = folder.canonicalize().map_err(|e| format!("can't read {}: {e}", folder.display()))?;
    let mut out = Vec::new();
    walk(folder, &real, "", 0, &mut out);
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

fn walk(dir: &Path, real_root: &Path, prefix: &str, depth: usize, out: &mut Vec<Reference>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if out.len() >= MAX_LISTED {
            return;
        }
        let file = e.file_name().to_string_lossy().into_owned();
        if file.starts_with('.') {
            continue;
        }
        let name = if prefix.is_empty() { file } else { format!("{prefix}/{file}") };
        let path = e.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if meta.is_dir() {
            if depth + 1 < MAX_DEPTH {
                walk(&path, real_root, &name, depth + 1, out);
            }
            continue;
        }
        let meta = if meta.file_type().is_symlink() {
            match linked_inside(&path, real_root) {
                Some(m) => m,
                None => continue,
            }
        } else {
            meta
        };
        if !meta.is_file() {
            continue;
        }
        let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
        out.push(Reference { kind: Kind::of(&name), path, name, size: meta.len(), modified });
    }
}

/// What a link points at, when that is a file inside the folder.
fn linked_inside(link: &Path, real_root: &Path) -> Option<std::fs::Metadata> {
    let target = link.canonicalize().ok()?;
    if !target.starts_with(real_root) {
        return None;
    }
    std::fs::metadata(&target).ok()
}

/// Whether `path` is a file inside `folder` (no `..`, no link leading out of it).
pub fn inside(folder: &Path, path: &Path) -> bool {
    let Ok(rest) = path.strip_prefix(folder) else { return false };
    if rest.as_os_str().is_empty() || rest.components().any(|c| !matches!(c, Component::Normal(_))) {
        return false;
    }
    match (folder.canonicalize(), path.canonicalize()) {
        (Ok(f), Ok(p)) => p.starts_with(&f) && p != f,
        _ => false,
    }
}

/// The folder for the project at `root`, made if missing and kept out of git.
pub fn ensure(root: &Path) -> Result<PathBuf, String> {
    let dir = folder(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("can't make {}: {e}", dir.display()))?;
    let dir = checked_folder(root)?;
    if crate::gitfs::head(root).is_some() {
        exclude_from_git(root)?;
    }
    Ok(dir)
}

/// Add the folder to the repo's `info/exclude` (not `.gitignore`, which is the user's tracked
/// file), once. `git add -f` still commits a reference.
fn exclude_from_git(root: &Path) -> Result<(), String> {
    let rel = crate::proc::git(root, &["rev-parse", "--git-path", "info/exclude"])?;
    let rel = PathBuf::from(rel.trim());
    let path = if rel.is_absolute() { rel } else { root.join(rel) };
    let now = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("can't read {}: {e}", path.display())),
    };
    let Some(next) = with_exclude_line(&now) else { return Ok(()) };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("can't make {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, next).map_err(|e| format!("can't write {}: {e}", path.display()))
}

/// The exclude file with the folder's line added, or None when it is already there.
fn with_exclude_line(text: &str) -> Option<String> {
    let anchored = format!("/{EXCLUDE_LINE}");
    if text.lines().map(str::trim).any(|l| l == EXCLUDE_LINE || l == anchored) {
        return None;
    }
    let mut out = text.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(EXCLUDE_LINE);
    out.push('\n');
    Some(out)
}

/// `stem.ext` in `dir`, or `stem-2.ext`, `stem-3.ext`, … when that is taken.
pub fn free_name(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let dot = if ext.is_empty() { String::new() } else { format!(".{ext}") };
    let first = dir.join(format!("{stem}{dot}"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{stem}-{n}{dot}"))).find(|p| !p.exists()).unwrap_or(first)
}

/// Where a screenshot pasted at `secs` (unix) goes: `screenshot-2026-10-25-143012.png`.
pub fn screenshot_path(dir: &Path, secs: u64) -> PathBuf {
    free_name(dir, &format!("screenshot-{}", utc_stamp(secs)), "png")
}

/// Copy the file at `src` into the project's references, under its own name (numbered when
/// taken). Returns where it went.
pub fn add_file(root: &Path, src: &Path) -> Result<PathBuf, String> {
    let meta = std::fs::metadata(src).map_err(|e| format!("can't read {}: {e}", src.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a file", src.display()));
    }
    if meta.len() > MAX_ADD_BYTES {
        return Err(format!("{} is over {} MB", src.display(), MAX_ADD_BYTES >> 20));
    }
    let dir = ensure(root)?;
    let p = Path::new(src.file_name().ok_or_else(|| format!("{} has no file name", src.display()))?);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let to = free_name(&dir, &stem, &ext);
    std::fs::copy(src, &to).map_err(|e| format!("can't copy {}: {e}", src.display()))?;
    Ok(to)
}

/// Delete a reference: only a file inside the project's references folder.
pub fn remove(root: &Path, path: &Path) -> Result<(), String> {
    let folder = checked_folder(root)?;
    if !inside(&folder, path) {
        return Err(format!("{} is not in {}", path.display(), folder.display()));
    }
    std::fs::remove_file(path).map_err(|e| format!("can't remove {}: {e}", path.display()))
}

/// `2026-10-25-143012` in UTC.
pub fn utc_stamp(secs: u64) -> String {
    let (y, m, d) = civil_date(secs / 86_400);
    let s = secs % 86_400;
    format!("{y:04}-{m:02}-{d:02}-{:02}{:02}{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// `2026-10-25T14:30:12Z`.
pub fn utc_iso(secs: u64) -> String {
    let (y, m, d) = civil_date(secs / 86_400);
    let s = secs % 86_400;
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s / 60 % 60, s % 60)
}

/// Days since 1970-01-01 as (year, month, day): Howard Hinnant's `civil_from_days`.
fn civil_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + u64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brand::APP_COMMAND;

    fn temp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("{APP_COMMAND}-refs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn git(dir: &Path, args: &[&str]) -> String {
        crate::proc::git(dir, args).unwrap()
    }

    /// A repo with one commit and a linked worktree beside it.
    fn repo_with_worktree(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = temp(tag);
        let main = base.join("repo");
        std::fs::create_dir_all(&main).unwrap();
        git(&main, &["init", "-q", "-b", "main"]);
        git(&main, &["config", "user.email", "t@t"]);
        git(&main, &["config", "user.name", "t"]);
        std::fs::write(main.join("a.txt"), "a").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-qm", "first"]);
        let wt = base.join("repo-feat");
        git(&main, &["worktree", "add", "-q", "-b", "feat", wt.to_str().unwrap()]);
        (base, main, wt)
    }

    fn touch(path: &Path, secs_ago: u64) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
        let at = std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago);
        std::fs::File::options().write(true).open(path).unwrap().set_modified(at).unwrap();
    }

    #[test]
    fn a_worktree_shares_the_main_checkouts_references() {
        let (base, main, wt) = repo_with_worktree("wt");
        let main = main.canonicalize().unwrap();
        assert_eq!(project_root(&wt).canonicalize().unwrap(), main, "a linked worktree maps to the main checkout");
        assert_eq!(project_root(&wt.join("sub")).canonicalize().unwrap(), main, "a folder inside it too");
        assert_eq!(project_root(&main).canonicalize().unwrap(), main);
        let added = add_file(&project_root(&wt), &main.join("a.txt")).unwrap();
        assert!(added.starts_with(folder(&project_root(&wt))));
        assert_eq!(list(&folder(&main)).unwrap().len(), 1, "seen from the main checkout");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_plain_folder_keeps_its_own() {
        let dir = temp("plain");
        if crate::gitfs::head(&dir).is_none() {
            assert_eq!(project_root(&dir), dir);
            assert_eq!(ensure(&dir).unwrap(), dir.join(DIR));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_folder_is_kept_out_of_git_once() {
        let (base, main, wt) = repo_with_worktree("exclude");
        ensure(&project_root(&wt)).unwrap();
        ensure(&main).unwrap();
        let exclude = std::fs::read_to_string(main.join(".git").join("info").join("exclude")).unwrap();
        assert_eq!(exclude.lines().filter(|l| *l == EXCLUDE_LINE).count(), 1, "{exclude}");
        std::fs::write(folder(&main).join("shot.png"), "x").unwrap();
        assert_eq!(git(&main, &["status", "--porcelain"]).trim(), "", "the reference is not an untracked file");
        assert!(!main.join(".gitignore").exists(), "the user's tracked files are not touched");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_exclude_line_is_added_after_whats_there() {
        assert_eq!(with_exclude_line("").unwrap(), format!("{EXCLUDE_LINE}\n"));
        assert_eq!(with_exclude_line("# comment").unwrap(), format!("# comment\n{EXCLUDE_LINE}\n"), "on a line of its own");
        assert_eq!(with_exclude_line(&format!("*.log\n{EXCLUDE_LINE}\n")), None);
        assert_eq!(with_exclude_line(&format!("/{EXCLUDE_LINE}  \n")), None, "anchored counts too");
    }

    #[test]
    fn references_are_listed_newest_first_without_hidden_files() {
        let root = temp("list");
        let dir = folder(&root);
        touch(&dir.join("old.png"), 3600);
        touch(&dir.join("new.pdf"), 10);
        touch(&dir.join("mockups").join("home.fig"), 60);
        touch(&dir.join(".DS_Store"), 0);
        let got = list(&dir).unwrap();
        let names: Vec<&str> = got.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["new.pdf", "mockups/home.fig", "old.png"]);
        assert_eq!(got.iter().map(|r| r.kind).collect::<Vec<_>>(), [Kind::Pdf, Kind::Design, Kind::Image]);
        assert_eq!(got[0].size, 1);
        assert_eq!(list(&root.join("missing")).unwrap(), Vec::new(), "no folder yet is no references");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_folder_is_not_listed() {
        let root = temp("links");
        let dir = folder(&root);
        touch(&dir.join("in.png"), 0);
        std::fs::write(root.join("secret.txt"), "s").unwrap();
        std::os::unix::fs::symlink(root.join("secret.txt"), dir.join("out.txt")).unwrap();
        std::os::unix::fs::symlink(dir.join("in.png"), dir.join("again.png")).unwrap();
        let names: Vec<String> = list(&dir).unwrap().into_iter().map(|r| r.name).collect();
        assert!(names.contains(&"again.png".to_string()) && !names.contains(&"out.txt".to_string()), "{names:?}");
        assert!(!inside(&dir, &dir.join("out.txt")), "nor removable through");
        // The app's folder itself a link to somewhere else: nothing is listed or removed there.
        let elsewhere = temp("links-elsewhere");
        touch(&folder(&elsewhere).join("theirs.png"), 0);
        let project = temp("links-project");
        std::os::unix::fs::symlink(elsewhere.join(DIR).parent().unwrap(), project.join(DIR).parent().unwrap()).unwrap();
        assert!(of_project(&project).is_err(), "a link out of the project");
        assert!(remove(&project, &folder(&project).join("theirs.png")).is_err());
        assert!(folder(&elsewhere).join("theirs.png").exists());
        for p in [&root, &elsewhere, &project] {
            let _ = std::fs::remove_dir_all(p);
        }
    }

    #[test]
    fn only_a_file_inside_the_folder_is_removed() {
        let root = temp("remove");
        let dir = folder(&root);
        touch(&dir.join("a.png"), 0);
        std::fs::write(root.join("keep.txt"), "k").unwrap();
        assert!(remove(&root, &dir.join("..").join("..").join("keep.txt")).is_err(), "no way out with ..");
        assert!(remove(&root, &root.join("keep.txt")).is_err());
        assert!(remove(&root, &dir).is_err(), "not the folder itself");
        assert!(root.join("keep.txt").exists());
        remove(&root, &dir.join("a.png")).unwrap();
        assert!(!dir.join("a.png").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_added_file_keeps_its_name_and_never_overwrites() {
        let root = temp("add");
        let src = root.join("hero.png");
        std::fs::write(&src, "1").unwrap();
        let first = add_file(&root, &src).unwrap();
        let second = add_file(&root, &src).unwrap();
        assert_eq!(first.file_name().unwrap(), "hero.png");
        assert_eq!(second.file_name().unwrap(), "hero-2.png");
        assert!(add_file(&root, &root).is_err(), "a folder is not added");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_screenshot_is_named_by_its_time() {
        assert_eq!(utc_stamp(0), "1970-01-01-000000");
        assert_eq!(utc_stamp(1_793_017_812), "2026-10-26-123012");
        assert_eq!(utc_iso(951_782_400), "2000-02-29T00:00:00Z", "a leap day");
        let dir = temp("shot");
        assert_eq!(screenshot_path(&dir, 0).file_name().unwrap(), "screenshot-1970-01-01-000000.png");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
