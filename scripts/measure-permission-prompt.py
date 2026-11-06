#!/usr/bin/env python3
"""Measure how long a Claude Code permission prompt lasts (ticket 31, NEEDS_SETTLES).

Runs interactive `claude` in a fresh temp dir inside a PTY, sends tiny "create a file"
turns, and timestamps two sources:
  * the agent's own hooks (PermissionRequest, PreToolUse, PostToolUse), logged by this
    script when called as `--hook EVENT`;
  * the PTY output (the first time the prompt text "Do you want to" appears).

Modes:
  hook   default permission mode, a PermissionRequest hook answers "allow" at once
  auto   `--permission-mode auto` (the agent decides on its own)
  human  default permission mode, nobody answers for HUMAN_WAIT seconds, then Enter

Usage: scripts/measure-permission-prompt.py MODE TURNS     (prints one JSON line per turn)
It uses only temp dirs and a throwaway --settings file; it never edits any real settings.
"""
import fcntl
import json
import os
import pty
import select
import struct
import sys
import tempfile
import termios
import time

HUMAN_WAIT = 3.0
TURN_TIMEOUT = 90.0
PROMPT_TEXT = b"Do you want to"
LOG_ENV = "MEASURE_HOOK_LOG"
DECISION_ENV = "MEASURE_HOOK_ALLOW"


def hook_main(event):
    """Called by the agent as a hook: log the time, optionally approve."""
    sys.stdin.read()
    with open(os.environ[LOG_ENV], "a") as f:
        f.write(f"{time.time()} {event}\n")
    if event == "PermissionRequest" and os.environ.get(DECISION_ENV) == "1":
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PermissionRequest",
                                                  "decision": {"behavior": "allow"}}}))


def write_settings(path):
    cmd = lambda e: {"hooks": [{"type": "command", "command": f"python3 {os.path.abspath(__file__)} --hook {e}"}]}
    settings = {"hooks": {e: [cmd(e)] for e in ("PermissionRequest", "PreToolUse", "PostToolUse")}}
    with open(path, "w") as f:
        json.dump(settings, f)


def read_log(path):
    out = {}
    if os.path.exists(path):
        for line in open(path):
            t, e = line.split()
            out.setdefault(e, float(t))
    return out


def pump(fd, seconds, buf):
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try:
                buf.append((time.time(), os.read(fd, 65536)))
            except OSError:
                return False
    return True


def main(mode, turns):
    work = tempfile.mkdtemp(prefix="maitty-measure-")
    cfg = os.path.join(work, "settings.json")
    log = os.path.join(work, "hooks.log")
    proj = os.path.join(work, "proj")
    os.mkdir(proj)
    write_settings(cfg)
    os.environ[LOG_ENV] = log
    os.environ[DECISION_ENV] = "1" if mode == "hook" else "0"
    args = ["claude", "--settings", cfg, "--permission-mode", "auto" if mode == "auto" else "default"]
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir(proj)
        os.execvp("claude", args)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    buf = []
    pump(fd, 5, buf)
    os.write(fd, b"\x1b[A")  # trust-folder dialog: move to "Yes"
    time.sleep(0.3)
    os.write(fd, b"\r")
    pump(fd, 6, buf)
    for n in range(turns):
        if os.path.exists(log):
            os.remove(log)
        buf.clear()
        sent = time.time()
        os.write(fd, f"create a file named x{n}.txt containing ok".encode())
        time.sleep(0.4)
        os.write(fd, b"\r")
        answered = None
        while time.time() - sent < TURN_TIMEOUT:
            pump(fd, 0.05, buf)
            if mode == "human" and answered is None:
                # The prompt's text is drawn in pieces, so the hook is the cue that it is up.
                seen = read_log(log).get("PermissionRequest")
                if seen and time.time() - seen >= HUMAN_WAIT:
                    os.write(fd, b"\r")
                    answered = time.time()
            if os.path.exists(os.path.join(proj, f"x{n}.txt")):
                pump(fd, 1.0, buf)
                break
        hooks = read_log(log)
        screen = next((t for t, c in buf if PROMPT_TEXT in c), None)
        req, post = hooks.get("PermissionRequest"), hooks.get("PostToolUse")
        print(json.dumps({
            "mode": mode, "turn": n,
            "screen_prompt_s": None if screen is None else round(screen - sent, 3),
            "hook_request_s": None if req is None else round(req - sent, 3),
            "request_to_post_ms": None if req is None or post is None else round((post - req) * 1000),
            "request_to_pretool_ms": None if req is None or "PreToolUse" not in hooks else round((hooks["PreToolUse"] - req) * 1000),
            "answered_after_request_s": None if answered is None or req is None else round(answered - req, 3),
            "file": os.path.exists(os.path.join(proj, f"x{n}.txt")),
        }), flush=True)
        pump(fd, 2, buf)
    os.write(fd, b"\x03")
    time.sleep(0.2)
    os.write(fd, b"\x03")
    time.sleep(0.5)
    os.kill(pid, 9)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--hook":
        hook_main(sys.argv[2])
    else:
        main(sys.argv[1], int(sys.argv[2]))
