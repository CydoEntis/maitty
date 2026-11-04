#!/usr/bin/python3
"""Loads the built GUI with the mocked Tauri host in a real WebKitGTK 4.1 view and measures the
WebKitWebProcess CPU. Uses a GtkOffscreenWindow so nothing is shown on screen (WebKit may treat
the page as hidden; the script reports document.hidden so that is visible in the result).

Usage: /usr/bin/python3 webkit-harness.py --scenario a|b|e [--measure 20] [--warm 5] [--exp noanim|nocursorblink|none]
"""
import argparse
import http.server
import json
import os
import socketserver
import sys
import threading
import time

import gi
gi.require_version('Gtk', '3.0')
gi.require_version('WebKit2', '4.1')
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
DIST = os.path.join(HERE, 'dist')
ap = argparse.ArgumentParser()
ap.add_argument('--scenario', default='a')
ap.add_argument('--measure', type=float, default=20)
ap.add_argument('--warm', type=float, default=5)
ap.add_argument('--exp', default='none')
ap.add_argument('--visible', action='store_true', help='use a real window (only when allowed)')
ap.add_argument('--tick', type=int, default=50, help='ms between output frames; the host flushes every 12 under a steady stream')
args = ap.parse_args()

SESSIONS = {
    'a': [{'id': 1, 'status': 'Idle', 'stream': False, 'agent': None}],
    'b': [{'id': 1, 'status': 'Working', 'stream': True}],
    'e': [{'id': 1, 'status': 'Working', 'stream': True}, {'id': 2, 'status': 'Working', 'stream': False}, {'id': 3, 'status': 'Idle', 'stream': False, 'agent': None}, {'id': 4, 'status': 'Blocked', 'stream': False}],
}
layout = {'tabs': [{'id': 'tab-1', 'panes': [{'id': 'pane-1', 'sessionId': 't1', 'col': 0}], 'focus': 'pane-1', 'max': None}], 'activeTabId': 'tab-1'}


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


socketserver.TCPServer.allow_reuse_address = True
httpd = socketserver.TCPServer(('127.0.0.1', 0), lambda *a, **k: Quiet(*a, directory=DIST, **k))
port = httpd.server_address[1]
threading.Thread(target=httpd.serve_forever, daemon=True).start()

ucm = WebKit2.UserContentManager()
pre = f"""
window.__SCENARIO = {json.dumps({'sessions': SESSIONS[args.scenario], 'bytesPerSec': 20000, 'tickMs': args.tick})};
try {{ localStorage.setItem('maitty.gui.welcomed.v1','1'); localStorage.setItem('maitty.gui.settings.v1','{{}}'); localStorage.setItem('maitty.gui.layout.v1', {json.dumps(json.dumps(layout))}); }} catch (e) {{}}
"""
with open(os.path.join(HERE, 'mock-tauri.js')) as f:
    mock = f.read()
for src in (pre, mock):
    ucm.add_script(WebKit2.UserScript.new(src, WebKit2.UserContentInjectedFrames.TOP_FRAME, WebKit2.UserScriptInjectionTime.START, None, None))
EXPS = {
    'none': '',
    'noanim': "const s=document.createElement('style');s.textContent='*,*::before,*::after{animation:none!important;transition:none!important}';document.head.appendChild(s)",
    'nocursorblink': "const s=document.createElement('style');s.textContent='.xterm .xterm-cursor-blink{animation:none!important}';document.head.appendChild(s)",
    'hidechrome': "const s=document.createElement('style');s.textContent='.mx-sidebar,.mx-tabbar,.mx-statusbar,.mx-pane-head,.title-bar{display:none!important}';document.head.appendChild(s)",
    'nostream': "window.__mock.dropStream()",
    'ns_noanim': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='*,*::before,*::after{animation:none!important;transition:none!important}';document.head.appendChild(s); ",
    'ns_nosweep': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='.mx-fx-working::after,.mx-fx-needs::after,.mx-fx-done::after,.mx-effect-chip::after{animation:none!important}';document.head.appendChild(s); ",
    'ns_nopulse': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='.mx-badge.is-needs,.mx-inbox-count.is-needs{animation:none!important}';document.head.appendChild(s); ",
    'ns_hidechrome': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='.mx-sidebar,.mx-tabbar,.mx-statusbar,.mx-pane-head,.title-bar{display:none!important}';document.head.appendChild(s); ",
    'ns_hidesidebar': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='.mx-sidebar{display:none!important}';document.head.appendChild(s); ",
    'ns_hidetabbar': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='.mx-tabbar{display:none!important}';document.head.appendChild(s); ",
    'ns_hidepanehead': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='.mx-pane-head{display:none!important}';document.head.appendChild(s); ",
    'ns_hidestatusbar': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='.mx-statusbar{display:none!important}';document.head.appendChild(s); ",
    'ns_stoptimers': "window.__mock.dropStream(); for (const ms of [80,1000,1500,8000,30000]) window.__perf.blockPeriod(ms); ",
    'genonly': "window.__mock.sinkOutput(); ",
    'ns_stop160': "window.__mock.dropStream(); window.__perf.blockPeriod(160); ",
    'ns_flat': "window.__mock.dropStream(); document.documentElement.dataset.finish='flat'; document.documentElement.dataset.style='flat'; ",
    'ns_stop80': "window.__mock.dropStream(); window.__perf.blockPeriod(80); ",
    'ns_stop1s': "window.__mock.dropStream(); window.__perf.blockPeriod(1000); ",
    'ns_stop1500': "window.__mock.dropStream(); window.__perf.blockPeriod(1500); ",
    'ns_noanim_stoptimers': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='*,*::before,*::after{animation:none!important;transition:none!important}';document.head.appendChild(s); for (const ms of [80,1000,1500,8000,30000]) window.__perf.blockPeriod(ms); ",
    'ns_noshadow': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='*{box-shadow:none!important;text-shadow:none!important;filter:none!important}';document.head.appendChild(s); ",
    'ns_nogradient': "window.__mock.dropStream(); const s=document.createElement('style');s.textContent='*,*::before,*::after{background-image:none!important}';document.head.appendChild(s); ",
    'nosweep': "const s=document.createElement('style');s.textContent='.mx-fx-working::after,.mx-fx-needs::after,.mx-fx-done::after,.mx-effect-chip::after{animation:none!important}';document.head.appendChild(s); ",
    'focus': "document.querySelector('.xterm-helper-textarea')?.focus()",
    'focusnoblink': "document.querySelector('.xterm-helper-textarea')?.focus(); const s=document.createElement('style');s.textContent='.xterm .xterm-cursor-blink{animation:none!important}';document.head.appendChild(s)",
}

settings = WebKit2.Settings()
settings.set_enable_developer_extras(True)
settings.set_enable_write_console_messages_to_stdout(False)
view = WebKit2.WebView(user_content_manager=ucm, settings=settings)
win = Gtk.Window() if args.visible else Gtk.OffscreenWindow()
win.set_default_size(1440, 900)
win.add(view)
win.show_all()

out = {'scenario': args.scenario, 'exp': args.exp, 'hidden': None}


def parent_of(pid):
    with open(f'/proc/{pid}/stat') as f:
        return int(f.read().rsplit(')', 1)[1].split()[1])


def web_pid():
    # /proc comm is truncated to 15 chars ("WebKitWebProces"); the process may sit under bwrap.
    me = os.getpid()
    for pid in os.listdir('/proc'):
        if not pid.isdigit():
            continue
        try:
            with open(f'/proc/{pid}/comm') as f:
                comm = f.read().strip()
            if not comm.startswith('WebKitWebProc'):
                continue
            p = int(pid)
            for _ in range(8):
                p = parent_of(p)
                if p == me:
                    return int(pid)
                if p <= 1:
                    break
        except (OSError, ValueError):
            continue
    return None


def cpu_times(pid):
    with open(f'/proc/{pid}/stat') as f:
        parts = f.read().rsplit(')', 1)[1].split()
    utime, stime = int(parts[11]), int(parts[12])
    # Main thread is the thread with tid == pid.
    with open(f'/proc/{pid}/task/{pid}/stat') as f:
        tp = f.read().rsplit(')', 1)[1].split()
    return (utime + stime) / os.sysconf('SC_CLK_TCK'), (int(tp[11]) + int(tp[12])) / os.sysconf('SC_CLK_TCK')


def js(script, cb):
    def done(v, res):
        try:
            r = v.evaluate_javascript_finish(res)
            cb(r.to_string() if r is not None and not r.is_null() else None)
        except Exception as e:  # noqa: BLE001
            cb(f'ERR {e}')
    view.evaluate_javascript(script, -1, None, None, None, done)


state = {}


def step_loaded():
    js(EXPS[args.exp] + "; 'ok'", lambda r: None)
    GLib.timeout_add(int(args.warm * 1000), step_start)
    return False


def step_start():
    pid = web_pid()
    if pid is None:
        print('could not find WebKitWebProcess', file=sys.stderr)
        Gtk.main_quit()
        return False
    state['pid'] = pid
    state['t0'] = time.monotonic()
    state['cpu0'] = cpu_times(pid)
    js("window.__perf.reset(); window.__rafN=0; (function tick(){window.__rafN++; requestAnimationFrame(tick)})(); document.hidden", lambda r: out.__setitem__('hidden', r))
    GLib.timeout_add(int(args.measure * 1000), step_end)
    return False


def step_end():
    secs = time.monotonic() - state['t0']
    cpu1 = cpu_times(state['pid'])
    out['seconds'] = round(secs, 1)
    out['webProcessCpuPct'] = round((cpu1[0] - state['cpu0'][0]) / secs * 100, 1)
    out['webProcessMainThreadCpuPct'] = round((cpu1[1] - state['cpu0'][1]) / secs * 100, 1)

    def got(r):
        try:
            out['page'] = json.loads(r)
        except Exception:  # noqa: BLE001
            out['page'] = r
        print(json.dumps(out, indent=1))
        with open(os.path.join(HERE, 'results', f'webkit-{args.scenario}-{args.exp}.json'), 'w') as f:
            json.dump(out, f, indent=1)
        Gtk.main_quit()
    js("JSON.stringify({ snap: window.__perf.snapshot(), rafPerSec: window.__rafN / window.__perf.snapshot().seconds, hidden: document.hidden, panes: document.querySelectorAll('.mx-pane').length, rows: document.querySelectorAll('.xterm-rows > div').length, anims: document.getAnimations().map(a => a.animationName + ':' + a.playState), blinkEls: document.querySelectorAll('.xterm-cursor-blink').length, hasFocus: document.hasFocus(), active: document.activeElement && document.activeElement.className })", got)
    return False


def on_load(v, event):
    if event == WebKit2.LoadEvent.FINISHED:
        GLib.timeout_add(4000, step_loaded)


view.connect('load-changed', on_load)
view.connect('close', lambda *a: Gtk.main_quit())
view.load_uri(f'http://127.0.0.1:{port}/')
GLib.timeout_add(int((args.warm + args.measure + 40) * 1000), Gtk.main_quit)
Gtk.main()
