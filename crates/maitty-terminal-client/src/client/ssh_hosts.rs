//! The named hosts in `~/.ssh/config`, for choosing a machine to add (ADR 0047).
//!
//! v1 lists names only. Settings before the first `Host` line or under a pattern such as
//! `Host *` are not merged into the hosts listed, and `Match` blocks are ignored. An
//! `Include` pulls in the other file's `Host` lines as if they stood at the top level; the
//! including file's current block carries on after it. Nothing here interprets a value:
//! aliases, host names and users are display data.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::{App, Bg};

/// ssh's own limit on nested `Include`s: the file you start from is level 0.
const MAX_INCLUDE_DEPTH: usize = 16;
const MAX_HOSTS: usize = 1000;
/// Bounds the fan-out of globs inside includes.
const MAX_FILES: usize = 256;
const MAX_LINE_BYTES: usize = 4096;
const MAX_FILE_BYTES: u64 = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshHost {
    pub alias: String,
    pub host_name: Option<String>,
    pub user: Option<String>,
}

/// How the parser reaches files, so tests need no real `~/.ssh`.
pub trait ConfigFiles {
    /// The file's text; `None` when it is missing or unreadable.
    fn read(&self, path: &Path) -> Option<String>;
    /// Every file matching a pattern with `*` or `?` in it, in any order.
    fn glob(&self, pattern: &Path) -> Vec<PathBuf>;
}

/// The folders `~` and a relative `Include` mean. ssh resolves relative includes in the user
/// config against `~/.ssh`.
pub struct Dirs<'a> {
    pub home: &'a Path,
    pub ssh: &'a Path,
}

pub fn parse(root: &Path, dirs: &Dirs, files: &dyn ConfigFiles) -> Vec<SshHost> {
    let mut walk = Walk {
        home: norm(dirs.home).trim_end_matches('/').to_string(),
        base: norm(dirs.ssh).trim_end_matches('/').to_string(),
        files,
        hosts: Vec::new(),
        seen: HashSet::new(),
        active: Vec::new(),
        reads: 0,
    };
    walk.file(&norm(root), 0);
    walk.hosts
}

/// Reads `~/.ssh/config` off the UI thread and hands the hosts to `done` on it.
pub fn load_in_background(app: &App, done: impl FnOnce(&mut App, Vec<SshHost>) + Send + 'static) {
    app.spawn_bg(move || {
        let hosts = user_hosts();
        Bg::Then(Box::new(move |app: &mut App| done(app, hosts)))
    });
}

fn user_hosts() -> Vec<SshHost> {
    let Some(dirs) = directories::BaseDirs::new() else {
        return Vec::new();
    };
    let home = dirs.home_dir();
    let ssh = home.join(".ssh");
    parse(&ssh.join("config"), &Dirs { home, ssh: &ssh }, &FsFiles)
}

struct Walk<'a> {
    home: String,
    base: String,
    files: &'a dyn ConfigFiles,
    hosts: Vec<SshHost>,
    seen: HashSet<String>,
    /// The files being read right now, so an include cycle ends.
    active: Vec<String>,
    reads: usize,
}

impl Walk<'_> {
    fn file(&mut self, path: &str, depth: usize) {
        let spent = depth > MAX_INCLUDE_DEPTH || self.reads >= MAX_FILES;
        if spent || self.active.iter().any(|p| p == path) {
            return;
        }
        self.reads += 1;
        let Some(text) = self.files.read(Path::new(path)) else {
            return;
        };
        self.active.push(path.to_string());
        let mut block: Vec<usize> = Vec::new();
        for line in text.lines() {
            if self.hosts.len() >= MAX_HOSTS {
                break;
            }
            let Some((key, args)) = split_line(line) else {
                continue;
            };
            match key.as_str() {
                "host" => block = self.add_hosts(&args),
                "match" => block.clear(),
                "hostname" => self.set(&block, args.first(), |h| &mut h.host_name),
                "user" => self.set(&block, args.first(), |h| &mut h.user),
                "include" => args.iter().for_each(|a| self.include(a, depth)),
                _ => {}
            }
        }
        self.active.pop();
    }

    /// The hosts this line added: patterns, unsafe names and repeats are not hosts.
    fn add_hosts(&mut self, aliases: &[String]) -> Vec<usize> {
        let mut added = Vec::new();
        for alias in aliases {
            let named = !is_pattern(alias) && clean(alias).is_some();
            if !named || self.hosts.len() >= MAX_HOSTS || !self.seen.insert(alias.clone()) {
                continue;
            }
            added.push(self.hosts.len());
            self.hosts.push(SshHost {
                alias: alias.clone(),
                host_name: None,
                user: None,
            });
        }
        added
    }

    /// The first value in a block wins, as in ssh.
    fn set(
        &mut self,
        block: &[usize],
        value: Option<&String>,
        field: fn(&mut SshHost) -> &mut Option<String>,
    ) {
        let Some(value) = value.and_then(|v| clean(v)) else {
            return;
        };
        for &i in block {
            let slot = field(&mut self.hosts[i]);
            if slot.is_none() {
                *slot = Some(value.clone());
            }
        }
    }

    fn include(&mut self, arg: &str, depth: usize) {
        let resolved = self.resolve(arg);
        let mut paths = if has_wildcard(&resolved) {
            self.files
                .glob(Path::new(&resolved))
                .iter()
                .map(|p| norm(p))
                .collect()
        } else {
            vec![resolved]
        };
        paths.sort();
        for path in paths {
            self.file(&path, depth + 1);
        }
    }

    /// `~` is home and a relative path is under the ssh folder. Environment variables are
    /// left alone: ssh does not expand them in an `Include` either.
    fn resolve(&self, arg: &str) -> String {
        let arg = arg.replace('\\', "/");
        if arg == "~" || arg.starts_with("~/") {
            format!("{}{}", self.home, &arg[1..])
        } else if is_absolute(&arg) {
            arg
        } else {
            format!("{}/{}", self.base, arg)
        }
    }
}

fn norm(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    path.starts_with('/') || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
}

fn is_pattern(alias: &str) -> bool {
    alias.starts_with('!') || has_wildcard(alias)
}

fn has_wildcard(s: &str) -> bool {
    s.contains(['*', '?'])
}

/// A value fit to show: not empty, no control characters.
fn clean(value: &str) -> Option<String> {
    let ok = !value.is_empty() && !value.chars().any(char::is_control);
    ok.then(|| value.to_string())
}

/// `Keyword value`, `Keyword=value` or `Keyword = value` as a lowercase keyword and its
/// words. `None` for a line to skip: a comment, a blank, an oversize or NUL-bearing line, or
/// an unterminated quote.
fn split_line(line: &str) -> Option<(String, Vec<String>)> {
    if line.len() > MAX_LINE_BYTES || line.contains('\0') {
        return None;
    }
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let key_end = line
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(line.len());
    let (key, rest) = line.split_at(key_end);
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();
    Some((key.to_ascii_lowercase(), words(rest)?))
}

/// Splits on whitespace, honouring double quotes. A `#` that starts a word begins a comment.
fn words(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut in_word, mut quoted) = (false, false);
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                in_word = true;
            }
            c if quoted => cur.push(c),
            c if c.is_whitespace() => {
                if in_word {
                    out.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            '#' if !in_word => return Some(out),
            c => {
                cur.push(c);
                in_word = true;
            }
        }
    }
    if quoted {
        return None;
    }
    if in_word {
        out.push(cur);
    }
    Some(out)
}

/// `*` and `?` over one path segment.
fn wildcard_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let (mut pi, mut ni) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// The real files.
struct FsFiles;

impl ConfigFiles for FsFiles {
    fn read(&self, path: &Path) -> Option<String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .ok()?
            .take(MAX_FILE_BYTES)
            .read_to_end(&mut bytes)
            .ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn glob(&self, pattern: &Path) -> Vec<PathBuf> {
        let pattern = norm(pattern);
        let mut found = vec![String::new()];
        for (i, seg) in pattern.split('/').enumerate() {
            if i > 0 && seg.is_empty() {
                continue;
            }
            found = found
                .iter()
                .flat_map(|dir| extend_segment(dir, seg, i == 0))
                .take(MAX_FILES)
                .collect();
        }
        found.into_iter().map(PathBuf::from).collect()
    }
}

/// What a path becomes with one more segment, which may match many names.
fn extend_segment(dir: &str, seg: &str, first: bool) -> Vec<String> {
    let join = |name: &str| {
        if first {
            name.to_string()
        } else {
            format!("{dir}/{name}")
        }
    };
    if !has_wildcard(seg) {
        return vec![join(seg)];
    }
    let listing = if dir.is_empty() { "/" } else { dir };
    let Ok(entries) = std::fs::read_dir(listing) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.') || seg.starts_with('.'))
        .filter(|name| wildcard_match(seg, name))
        .map(|name| join(&name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct Mem(BTreeMap<String, String>);

    impl ConfigFiles for Mem {
        fn read(&self, path: &Path) -> Option<String> {
            self.0.get(&norm(path)).cloned()
        }

        fn glob(&self, pattern: &Path) -> Vec<PathBuf> {
            let pattern = norm(pattern);
            let (dir, seg) = pattern.rsplit_once('/').unwrap_or(("", &pattern));
            self.0
                .keys()
                .filter_map(|k| k.rsplit_once('/').map(|(d, name)| (k, d, name)))
                .filter(|(_, d, name)| *d == dir && wildcard_match(seg, name))
                .map(|(k, _, _)| PathBuf::from(k))
                .collect()
        }
    }

    fn mem(files: &[(&str, &str)]) -> Mem {
        Mem(files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect())
    }

    fn unix(files: &[(&str, &str)]) -> Vec<SshHost> {
        let (home, ssh) = (Path::new("/home/me"), Path::new("/home/me/.ssh"));
        parse(Path::new(ROOT), &Dirs { home, ssh }, &mem(files))
    }

    fn aliases(hosts: &[SshHost]) -> Vec<&str> {
        hosts.iter().map(|h| h.alias.as_str()).collect()
    }

    const ROOT: &str = "/home/me/.ssh/config";

    #[test]
    fn reads_host_name_and_user() {
        let hosts = unix(&[(ROOT, "Host box\n  HostName 10.0.0.5\n  User ana\n")]);
        assert_eq!(
            hosts,
            vec![SshHost {
                alias: "box".into(),
                host_name: Some("10.0.0.5".into()),
                user: Some("ana".into()),
            }]
        );
    }

    #[test]
    fn follows_an_include_chain() {
        let hosts = unix(&[
            (ROOT, "Host a\nInclude /x/b\n"),
            ("/x/b", "Host b\nInclude /x/c\n"),
            ("/x/c", "Host c\n"),
        ]);
        assert_eq!(aliases(&hosts), ["a", "b", "c"]);
    }

    #[test]
    fn glob_include_is_read_in_lexical_order() {
        let hosts = unix(&[
            (ROOT, "Include config.d/*\nHost last\n"),
            ("/home/me/.ssh/config.d/20-b", "Host b\n"),
            ("/home/me/.ssh/config.d/10-a", "Host a\n"),
            ("/home/me/.ssh/config.d/sub/z", "Host z\n"),
        ]);
        assert_eq!(aliases(&hosts), ["a", "b", "last"]);
    }

    #[test]
    fn patterns_are_skipped_but_named_aliases_beside_them_stay() {
        let hosts = unix(&[(
            ROOT,
            "Host *\nHost *.corp\nHost !x\nHost a b*\nHost c?\nHost d\n",
        )]);
        assert_eq!(aliases(&hosts), ["a", "d"]);
    }

    #[test]
    fn settings_under_a_pattern_or_before_any_host_are_not_applied() {
        let hosts = unix(&[(ROOT, "User root\nHost *\n  User wild\nHost box\n")]);
        assert_eq!(hosts[0].user, None);
    }

    #[test]
    fn quoted_include_path_may_hold_spaces() {
        let hosts = unix(&[
            (ROOT, "Include \"/my configs/extra\" /x/plain\n"),
            ("/my configs/extra", "Host spaced\n"),
            ("/x/plain", "Host plain\n"),
        ]);
        assert_eq!(aliases(&hosts), ["spaced", "plain"]);
    }

    #[test]
    fn equals_syntax_works_with_or_without_spaces() {
        let hosts = unix(&[(ROOT, "Host=foo\nHostName = 1.2.3.4\nUser=bob\n")]);
        assert_eq!(hosts[0].alias, "foo");
        assert_eq!(hosts[0].host_name.as_deref(), Some("1.2.3.4"));
        assert_eq!(hosts[0].user.as_deref(), Some("bob"));
    }

    #[test]
    fn keywords_ignore_case() {
        let hosts = unix(&[(ROOT, "HOST box\nhostname h\nUSER u\n")]);
        assert_eq!(hosts[0].host_name.as_deref(), Some("h"));
        assert_eq!(hosts[0].user.as_deref(), Some("u"));
    }

    #[test]
    fn comments_are_dropped_but_a_hash_inside_a_word_is_kept() {
        let hosts = unix(&[(
            ROOT,
            "# Host hidden\n   # indented\nHost box # trailing\nUser a#b\nHostName \"h #1\"\n",
        )]);
        assert_eq!(aliases(&hosts), ["box"]);
        assert_eq!(hosts[0].user.as_deref(), Some("a#b"));
        assert_eq!(hosts[0].host_name.as_deref(), Some("h #1"));
    }

    #[test]
    fn malformed_lines_are_skipped_and_the_rest_still_reads() {
        let huge = format!("Host {}", "x".repeat(MAX_LINE_BYTES + 1));
        let text = format!(
            "Host\nUser\nHost \"open\nHost nul\0\n{huge}\n=\n\"\nHost ok\n  HostName 1.1.1.1\n"
        );
        let hosts = unix(&[(ROOT, &text)]);
        assert_eq!(aliases(&hosts), ["ok"]);
        assert_eq!(hosts[0].host_name.as_deref(), Some("1.1.1.1"));
    }

    #[test]
    fn an_include_chain_of_twenty_stops_at_sixteen_levels() {
        let names: Vec<String> = (0..20).map(|i| format!("/chain/f{i}")).collect();
        let texts: Vec<String> = (0..20)
            .map(|i| format!("Host h{i}\nInclude /chain/f{}\n", i + 1))
            .collect();
        let mut files: Vec<(&str, &str)> = vec![(ROOT, "Include /chain/f0\n")];
        files.extend(
            names
                .iter()
                .zip(&texts)
                .map(|(n, t)| (n.as_str(), t.as_str())),
        );
        let hosts = unix(&files);
        assert_eq!(hosts.len(), MAX_INCLUDE_DEPTH);
        assert_eq!(hosts.last().map(|h| h.alias.as_str()), Some("h15"));
    }

    #[test]
    fn a_file_including_itself_ends() {
        let hosts = unix(&[(ROOT, "Host a\nInclude /home/me/.ssh/config\nHost b\n")]);
        assert_eq!(aliases(&hosts), ["a", "b"]);
    }

    #[test]
    fn a_repeated_alias_keeps_the_first() {
        let hosts = unix(&[(
            ROOT,
            "Host box\nHostName first\nHost box\nHostName second\nUser late\n",
        )]);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host_name.as_deref(), Some("first"));
        assert_eq!(hosts[0].user, None);
    }

    #[test]
    fn the_first_value_in_a_block_wins() {
        let hosts = unix(&[(ROOT, "Host box\nUser a\nUser b\nHostName x\nHostName y\n")]);
        assert_eq!(hosts[0].user.as_deref(), Some("a"));
        assert_eq!(hosts[0].host_name.as_deref(), Some("x"));
    }

    #[test]
    fn a_host_line_lists_several_aliases_that_share_settings() {
        let hosts = unix(&[(ROOT, "Host a b\n  User shared\n")]);
        assert_eq!(aliases(&hosts), ["a", "b"]);
        assert!(hosts.iter().all(|h| h.user.as_deref() == Some("shared")));
    }

    #[test]
    fn a_match_block_ends_the_host_and_its_settings_are_ignored() {
        let hosts = unix(&[(
            ROOT,
            "Host a\nUser one\nMatch host b\nUser two\nHostName nope\nHost c\n",
        )]);
        assert_eq!(aliases(&hosts), ["a", "c"]);
        assert_eq!(hosts[0].user.as_deref(), Some("one"));
        assert_eq!(hosts[0].host_name, None);
    }

    #[test]
    fn a_relative_include_is_under_the_ssh_folder() {
        let hosts = unix(&[
            (ROOT, "Include extra\n"),
            ("/home/me/.ssh/extra", "Host rel\n"),
            ("/home/me/extra", "Host wrong\n"),
        ]);
        assert_eq!(aliases(&hosts), ["rel"]);
    }

    #[test]
    fn tilde_means_home() {
        let hosts = unix(&[
            (ROOT, "Include ~/other/cfg\n"),
            ("/home/me/other/cfg", "Host tilde\n"),
        ]);
        assert_eq!(aliases(&hosts), ["tilde"]);
    }

    #[test]
    fn environment_variables_are_not_expanded() {
        let hosts = unix(&[
            (ROOT, "Include %USERPROFILE%/x $HOME/y\n"),
            ("/home/me/.ssh/%USERPROFILE%/x", "Host a\n"),
            ("/home/me/.ssh/$HOME/y", "Host b\n"),
            ("/home/me/y", "Host c\n"),
        ]);
        assert_eq!(aliases(&hosts), ["a", "b"]);
    }

    #[test]
    fn a_missing_include_is_skipped() {
        let hosts = unix(&[(ROOT, "Include /nope\nInclude gone/*\nHost a\n")]);
        assert_eq!(aliases(&hosts), ["a"]);
    }

    #[test]
    fn a_missing_root_gives_no_hosts() {
        assert!(unix(&[]).is_empty());
    }

    #[test]
    fn windows_paths_work_with_either_separator() {
        let (home, ssh) = (Path::new(r"C:\Users\me"), Path::new(r"C:\Users\me\.ssh"));
        let root = "Include C:/Users/me/.ssh/config.d/*\n\
                    Include C:\\Users\\me\\.ssh\\extra\n\
                    Include ~\\home.cfg\n\
                    Include rel\\one\n";
        let files = mem(&[
            ("C:/Users/me/.ssh/config", root),
            ("C:/Users/me/.ssh/config.d/a", "Host glob\n"),
            ("C:/Users/me/.ssh/extra", "Host back\n"),
            ("C:/Users/me/home.cfg", "Host tilde\n"),
            ("C:/Users/me/.ssh/rel/one", "Host rel\n"),
        ]);
        let hosts = parse(
            Path::new(r"C:\Users\me\.ssh\config"),
            &Dirs { home, ssh },
            &files,
        );
        assert_eq!(aliases(&hosts), ["glob", "back", "tilde", "rel"]);
    }

    #[test]
    fn an_alias_with_a_control_character_is_dropped() {
        let hosts = unix(&[(ROOT, "Host bad\u{1b}[31m ok\nHostName x\u{7}y\nUser fine\n")]);
        assert_eq!(aliases(&hosts), ["ok"]);
        assert_eq!(hosts[0].host_name, None);
        assert_eq!(hosts[0].user.as_deref(), Some("fine"));
    }

    #[test]
    fn the_host_list_is_capped() {
        let text: String = (0..MAX_HOSTS + 50)
            .map(|i| format!("Host h{i}\n"))
            .collect();
        assert_eq!(unix(&[(ROOT, &text)]).len(), MAX_HOSTS);
    }

    #[test]
    fn wildcards_match_within_a_segment() {
        assert!(wildcard_match("*", "anything"));
        assert!(wildcard_match("a*c", "abbbc"));
        assert!(wildcard_match("a?c", "abc"));
        assert!(!wildcard_match("a?c", "ac"));
        assert!(!wildcard_match("a*c", "abd"));
    }

    #[test]
    fn the_real_filesystem_reads_and_globs() {
        let dir = std::env::temp_dir().join(format!("ssh_hosts_{}", std::process::id()));
        let sub = dir.join("config.d");
        std::fs::create_dir_all(&sub).expect("make temp dir");
        std::fs::write(sub.join("b"), "Host b\n").expect("write");
        std::fs::write(sub.join("a"), "Host a\n").expect("write");
        std::fs::write(sub.join(".hidden"), "Host hidden\n").expect("write");
        std::fs::write(dir.join("config"), "Include config.d/*\n").expect("write");
        let dirs = Dirs {
            home: &dir,
            ssh: &dir,
        };
        let hosts = parse(&dir.join("config"), &dirs, &FsFiles);
        std::fs::remove_dir_all(&dir).expect("clean up");
        assert_eq!(aliases(&hosts), ["a", "b"]);
    }
}
