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
