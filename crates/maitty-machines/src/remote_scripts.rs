//! The fixed scripts the ssh lines run on another machine. They are constants: the destination
//! and the byte count reach the install script as positional arguments, never as part of its text.

/// Run on the far side as `sh -c <this> sh`.
pub const PLATFORM_SCRIPT: &str = "printf '%s\\n' maitty-platform; uname -sm";

/// Where the far side's login shell finds the app, for `doctor`: a fixed script, so nothing a
/// person or a machine supplies can reach the remote shell.
pub const WHERE_SCRIPT: &str = concat!("command -v ", env!("APP_COMMAND"));

/// `$1` is the destination (a leading `~/` is the far side's home), `$2` the byte count
/// expected on stdin. The new file is written beside the destination so the final `mv` is a
/// rename, and is made executable by its owner alone until it has run `--version`.
pub const INSTALL_SCRIPT: &str = concat!(
    "umask 077\n",
    "dest=$1\n",
    "want=$2\n",
    "case $dest in\n",
    "  \"~/\"*) dest=$HOME/$(printf '%s' \"$dest\" | cut -c3-) ;;\n",
    "esac\n",
    "[ ! -d \"$dest\" ] || exit 72\n",
    "dir=$(dirname -- \"$dest\")\n",
    "mkdir -p -- \"$dir\" || exit 72\n",
    "tmp=$(mktemp \"$dir/.",
    env!("APP_COMMAND"),
    "-new.XXXXXX\") || exit 72\n",
    "trap 'rm -f -- \"$tmp\"' EXIT\n",
    "trap 'exit 72' HUP INT TERM\n",
    "cat > \"$tmp\" || exit 72\n",
    "got=$(wc -c < \"$tmp\" | tr -d ' ')\n",
    "[ \"$got\" = \"$want\" ] || exit 70\n",
    "chmod 700 -- \"$tmp\" || exit 72\n",
    "case $(\"$tmp\" --version 2>/dev/null) in\n",
    "  \"",
    env!("APP_COMMAND"),
    " \"*) ;;\n",
    "  *) exit 71 ;;\n",
    "esac\n",
    "chmod 755 -- \"$tmp\" || exit 72\n",
    "mv -f -- \"$tmp\" \"$dest\" || exit 72\n",
    "echo OK\n",
);

/// The line the paste script prints before the path, so text a login shell prints first (a
/// banner, a message of the day) is never taken for it.
pub const PASTE_MARKER: &str = concat!(env!("APP_COMMAND"), "-paste");

/// Keeps a pasted image or a sent file (ADR 0049, 0055) on the far side. `$1` is the file's name (letters, digits, `.`, `_`
/// and `-`, not starting with a dot), `$2` the byte count expected on stdin. The file goes in
/// the app's folder in the far side's cache, which must belong to the user running the script
/// (a folder someone else owns is refused), readable by its owner alone, and is written beside
/// its place and renamed into it so a half-sent file is never there. Pastes older than a week
/// are cleared out, and so are half-sent files a killed upload left behind (older than a day). It
/// prints `PASTE_MARKER` and then the file's absolute path.
pub const PASTE_SCRIPT: &str = concat!(
    "umask 077\n",
    "name=$1\n",
    "want=$2\n",
    "case $name in\n",
    "  ''|.*|*[!A-Za-z0-9._-]*) exit 72 ;;\n",
    "esac\n",
    "case ${XDG_CACHE_HOME:-} in\n",
    "  /*) base=$XDG_CACHE_HOME ;;\n",
    "  *) base=${HOME:-}/.cache ;;\n",
    "esac\n",
    "case $base in\n",
    "  /*) ;;\n",
    "  *) exit 72 ;;\n",
    "esac\n",
    "dir=$base/",
    env!("APP_COMMAND"),
    "/pastes\n",
    "mkdir -p -- \"$dir\" || exit 72\n",
    "[ -O \"$dir\" ] || exit 72\n",
    "find \"$dir\" -type f -name 'paste-*' -mtime +7 -exec rm -f {} + 2>/dev/null\n",
    "find \"$dir\" -type f -name '.new.*' -mtime +1 -exec rm -f {} + 2>/dev/null\n",
    "tmp=$(mktemp \"$dir/.new.XXXXXX\") || exit 72\n",
    "trap 'rm -f -- \"$tmp\"' EXIT\n",
    "trap 'exit 72' HUP INT TERM\n",
    "cat > \"$tmp\" || exit 72\n",
    "got=$(wc -c < \"$tmp\" | tr -d ' ')\n",
    "[ \"$got\" = \"$want\" ] || exit 70\n",
    "mv -f -- \"$tmp\" \"$dir/$name\" || exit 72\n",
    "printf '%s\\n%s\\n' '",
    env!("APP_COMMAND"),
    "-paste' \"$dir/$name\"\n",
);
