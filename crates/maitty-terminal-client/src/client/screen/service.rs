//! When the window loses its own daemon: a reconnecting bar that counts its five tries, then a
//! box that says the service is not answering and offers to start it. The daemon owns every
//! session, so nothing here stops one; the window only keeps asking. Docs: UI.md, "Service and
//! update states".

use super::closing::Banner;
use super::*;
use crate::client::connections::Connections;
use crate::ipc;
use std::time::Duration;

/// The tries before the window gives up and says so.
pub(in crate::client) const TRIES: u8 = 5;
/// The wait before each try, in order.
const BACKOFF: [Duration; TRIES as usize] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(15),
];
/// How often a window that gave up counting still asks, quietly: another window or an update can
/// bring the service back long after the five tries, and this one must not need a key to notice.
const DOWN_RETRY: Duration = Duration::from_secs(15);
/// Rows and columns of the box that says the service is down.
const DOWN_W: u16 = 64;
const DOWN_H: u16 = 13;

/// What the service is called on screen: the command with a `d`, as `maittyd`.
fn daemon_name() -> String {
    format!("{}d", crate::brand::APP_COMMAND)
}

/// Whether the window has its daemon.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(in crate::client) enum Service {
    #[default]
    Up,
    /// Lost it just now: try `try_n` of `TRIES` is at `next_at`.
    Reconnecting { try_n: u8, next_at: Instant },
    /// Every counted try failed: the box is up, and a quiet try is at `next_at`.
    Down { next_at: Instant },
}

impl Service {
    pub(in crate::client) fn is_down(&self) -> bool {
        matches!(self, Service::Down { .. })
    }

    fn down_from(now: Instant) -> Self {
        Service::Down { next_at: now + DOWN_RETRY }
    }

    /// The try this is on and whether it is due at `now`. None while the window has its daemon.
    fn try_at(&self, now: Instant) -> Option<(u8, bool)> {
        match self {
            Service::Up => None,
            Service::Reconnecting { try_n, next_at } => Some((*try_n, now >= *next_at)),
            Service::Down { next_at } => Some((TRIES, now >= *next_at)),
        }
    }
}

/// What the service box does with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum ServiceKey {
    Retry,
    Start,
    Leave,
    Nothing,
}

pub(in crate::client) fn service_key(k: &KeyEvent) -> ServiceKey {
    match k.code {
        KeyCode::Char('r') => ServiceKey::Retry,
        KeyCode::Char('s') => ServiceKey::Start,
        KeyCode::Esc => ServiceKey::Leave,
        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => ServiceKey::Leave,
        _ => ServiceKey::Nothing,
    }
}

/// The bar's words for try `try_n`, due in `wait`.
pub(in crate::client) fn reconnect_line(try_n: u8, wait: Duration) -> String {
    format!(
        "┆ Reconnecting to {} · sessions still running · try {try_n} of {TRIES} · next in {}s · r retry",
        daemon_name(),
        wait.as_secs_f64().ceil() as u64
    )
}

impl App {
    /// The daemon went away without being asked to: keep the window and start counting.
    pub(in crate::client) fn service_lost(&mut self) {
        self.service = Service::Reconnecting { try_n: 1, next_at: Instant::now() + BACKOFF[0] };
        self.dirty = true;
    }

    /// The reconnecting bar, while it is up.
    pub(in crate::client) fn service_banner(&self) -> Option<Banner> {
        match &self.service {
            Service::Reconnecting { try_n, next_at } => Some(Banner {
                text: reconnect_line(*try_n, next_at.saturating_duration_since(Instant::now())),
                warn: true,
            }),
            _ => None,
        }
    }

    /// Keys while the daemon is away: only the box's own, and the way out.
    pub(in crate::client) fn on_service_key(&mut self, k: &KeyEvent) {
        match service_key(k) {
            ServiceKey::Retry => {
                self.service_retry_now = true;
            }
            ServiceKey::Start if self.service.is_down() => {
                self.service_start_now = true;
            }
            ServiceKey::Leave => self.quit = Some("left while maittyd was away".into()),
            _ => {}
        }
        self.dirty = true;
    }

    /// The service is back as a version this window cannot talk to (another copy of the app
    /// updated and restarted it): no try will ever work, so stop asking. A newer app installed
    /// here opens in this window's place; otherwise the start-up screen for a version mismatch
    /// takes over and offers the update.
    pub(in crate::client) fn service_changed_version(&mut self, daemon: u32) {
        if self.window_stale() {
            return self.relaunch_window();
        }
        self.service_version = Some(daemon);
        self.quit = Some(format!("{} was updated", daemon_name()));
        self.dirty = true;
    }

    /// Once a tick: make the try that is due, and start the service when it was asked for.
    pub(in crate::client) async fn service_tick(&mut self, connections: &mut Connections) {
        let Some((try_n, timed)) = self.service.try_at(Instant::now()) else {
            return;
        };
        let asked = std::mem::take(&mut self.service_retry_now);
        let start = std::mem::take(&mut self.service_start_now);
        if !(timed || asked || start) {
            // The countdown on the bar changes every second.
            self.dirty = true;
            return;
        }
        let attempt = if start {
            ipc::open_or_spawn_installed(true).await
        } else {
            ipc::open(true).await
        };
        match attempt {
            Ok(connection) => {
                connections.add(MachineId::LOCAL, connection);
                self.note_local_protocol();
                self.service = Service::Up;
                self.reattached = Some(Instant::now());
                self.notify(format!("Reconnected to {}", daemon_name()), false);
            }
            Err(e) if ipc::other_version(&e).is_some() => {
                self.service_changed_version(ipc::other_version(&e).unwrap_or_default());
            }
            Err(_) if try_n >= TRIES => self.service = Service::down_from(Instant::now()),
            Err(_) => {
                let next = usize::from(try_n).min(BACKOFF.len() - 1);
                self.service = Service::Reconnecting { try_n: try_n + 1, next_at: Instant::now() + BACKOFF[next] };
            }
        }
        self.dirty = true;
    }
}

/// How long the note about agents a restart stopped stays up.
const STOPPED_NOTE_FOR: Duration = Duration::from_secs(30);

impl App {
    /// The agents the daemon's last restart stopped, with the command that brings each back.
    pub(in crate::client) fn stopped_agents(&self) -> Vec<(ClientTerm, String, String)> {
        self.all_terms()
            .into_iter()
            .filter_map(|(term, info)| {
                let i = info.interrupted.as_ref()?;
                Some((term, i.agent.clone(), i.resume.clone()))
            })
            .collect()
    }

    /// The note under the header after a restart: what came back and what did not. Resuming is
    /// never automatic: Enter on the agent's row types its resume command.
    pub(in crate::client) fn restart_banner(&mut self) -> Option<Banner> {
        let stopped = self.stopped_agents().len();
        if stopped == 0 {
            self.stopped_since = None;
            return None;
        }
        let since = *self.stopped_since.get_or_insert_with(Instant::now);
        (since.elapsed() < STOPPED_NOTE_FOR).then(|| Banner {
            text: format!(
                "{} restarted · {stopped} agent{} stopped · Enter on its row resumes",
                daemon_name(),
                if stopped == 1 { "" } else { "s" }
            ),
            warn: false,
        })
    }

    /// Type an interrupted agent's resume command into its session. True when it had one.
    pub(in crate::client) fn resume_stopped_agent(&mut self, term: ClientTerm) -> bool {
        let Some((_, agent, resume)) = self.stopped_agents().into_iter().find(|(t, ..)| *t == term) else {
            return false;
        };
        self.send_input(term, format!("{resume}\r").into_bytes());
        self.notify(format!("Resuming {agent}"), false);
        true
    }
}

/// The box that says the service is not answering: red border, a red title, three things to try.
pub(in crate::client) fn draw_service_down(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) {
    let buf = f.buffer_mut();
    super::popups::dim_all(app, buf, area, t);
    let w = DOWN_W.min(area.width.saturating_sub(2));
    let h = DOWN_H.min(area.height.saturating_sub(2));
    let r = Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h };
    let title = format!("{} ISN'T RESPONDING", daemon_name().to_uppercase());
    let mut c = Card::new(t, &title).lit(t.err);
    c.bg = t.bg;
    c.close = Some(ScreenHit::Noop);
    app.screen.modal_rects.push(r);
    let inside = card(app, buf, r, &c, t);
    let s = Style::default().bg(t.bg);
    let lines: [(&str, Style); 7] = [
        ("Your sessions are still running: nothing was closed.", s.fg(t.text)),
        ("", s),
        ("Try, in this order:", s.fg(t.muted)),
        ("1  r   connect again", s.fg(t.strong)),
        ("2  s   start the service", s.fg(t.strong)),
        ("3  run `maitty doctor` in another terminal", s.fg(t.strong)),
        ("It also tries again by itself every 15 seconds.", s.fg(t.muted)),
    ];
    for (i, (text, style)) in lines.iter().enumerate() {
        put(buf, inside.x + 2, inside.y + 1 + i as u16, &[seg(*text, *style)], inside.right());
    }
    let bar = super::popups::hints(t, &[("s", "start"), ("r", "retry"), ("Esc", "close")]);
    put(buf, inside.x + 1, inside.bottom(), &bar, inside.right());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::reference_fixture::{Stage, world};
    use crate::protocol::{ClientMsg, Interrupted};

    #[test]
    fn the_bar_counts_tries_and_seconds() {
        assert_eq!(
            reconnect_line(2, Duration::from_millis(2300)),
            format!(
                "┆ Reconnecting to {} · sessions still running · try 2 of 5 · next in 3s · r retry",
                daemon_name()
            )
        );
    }

    #[test]
    fn r_s_and_esc_are_the_keys() {
        let k = |c| KeyEvent::new(c, KeyModifiers::NONE);
        assert_eq!(service_key(&k(KeyCode::Char('r'))), ServiceKey::Retry);
        assert_eq!(service_key(&k(KeyCode::Char('s'))), ServiceKey::Start);
        assert_eq!(service_key(&k(KeyCode::Esc)), ServiceKey::Leave);
        assert_eq!(service_key(&k(KeyCode::Char('x'))), ServiceKey::Nothing);
    }

    #[test]
    fn while_the_daemon_is_away_the_bar_shows_and_typing_goes_nowhere() {
        let mut app = world(Stage::None);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        app.service_lost();
        let bar = app.closing_banner().expect("the reconnecting bar").text;
        assert!(bar.starts_with("┆ Reconnecting to ") && bar.contains("try 1 of 5"), "{bar}");
        app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert!(app.mode == Mode::Normal, "no shortcut mode either");
        assert!(rx.try_recv().is_err(), "nothing was sent to a pane");
        app.on_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert!(app.service_retry_now, "r asks for a try now");
        app.service = Service::down_from(Instant::now());
        app.on_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert!(app.service_start_now, "s asks for the service to start");
    }

    #[test]
    fn a_window_that_gave_up_counting_still_tries_by_itself() {
        let now = Instant::now();
        let down = Service::down_from(now);
        assert!(down.is_down());
        assert_eq!(down.try_at(now), Some((TRIES, false)), "not before its wait is over");
        assert_eq!(down.try_at(now + DOWN_RETRY), Some((TRIES, true)));
        assert_eq!(Service::Up.try_at(now), None);
    }

    #[test]
    fn a_service_that_came_back_as_another_version_ends_the_tries() {
        let mut app = world(Stage::None);
        app.service_lost();
        app.installed_version = Some(Ok(env!("APP_VERSION").into()));
        app.service_changed_version(crate::protocol::PROTOCOL_VERSION + 1);
        assert_eq!(app.service_version, Some(crate::protocol::PROTOCOL_VERSION + 1));
        assert!(app.quit.is_some() && !app.relaunch, "the mismatch screen takes over, in this app");
    }

    #[test]
    fn a_newer_app_installed_here_takes_the_window_when_the_service_changed_version() {
        let mut app = world(Stage::None);
        app.service_lost();
        app.installed_version = Some(Ok("999.0.0".into()));
        app.service_changed_version(crate::protocol::PROTOCOL_VERSION + 1);
        assert!(app.relaunch && app.quit.is_some());
        assert_eq!(app.service_version, None, "the installed app talks to that service");
    }

    #[test]
    fn agents_a_restart_stopped_are_named_and_enter_on_one_types_its_resume_command() {
        let mut app = world(Stage::None);
        let id = *app.snap.terms.keys().next().unwrap();
        app.snap.terms.get_mut(&id).unwrap().interrupted =
            Some(Interrupted { agent: "codex".into(), resume: "codex resume --last".into() });
        let note = app.restart_banner().expect("the note").text;
        assert!(note.contains("1 agent stopped") && note.contains("Enter"), "{note}");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        assert!(app.resume_stopped_agent(ClientTerm::local(id)));
        let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(
            sent.iter().any(|o| matches!(&o.msg, ClientMsg::Input { term, data } if *term == id && data == b"codex resume --last\r")),
            "{sent:?}"
        );
        assert!(!app.resume_stopped_agent(ClientTerm::local(id + 100)), "only an interrupted one");
    }
}
