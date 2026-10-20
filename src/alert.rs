//! Alerts: seshi's needs-you sound, and a desktop notification for when no seshi window is
//! open to say it. Each system's own tools show and play them, so nothing extra is needed: a
//! Windows toast through PowerShell, `osascript` on macOS, `notify-send` on Linux; sounds
//! through PowerShell's media player, `afplay`, or `paplay` / `pw-play`.
//!
//! The sounds are made here rather than borrowed from the system: a system sound means
//! something else already (on Windows, "you can't do that").
//!
//! Everything runs on a short-lived thread and failures are ignored: an alert that can't be
//! shown must never get in the way. A notification can carry a `seshi://` link (see
//! `reveal`): clicking it takes you to the session.

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// What happened, for picking the sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Needs,
    Done,
}

impl Kind {
    /// Its sound as notes (pitch in Hz, length in ms): two rising for "needs you".
    fn notes(self) -> &'static [(f32, u32)] {
        match self {
            Kind::Needs => &[(659.25, 130), (987.77, 240)],
            Kind::Done => &[(523.25, 320)],
        }
    }

    fn file_name(self) -> &'static str {
        // The number changes with the sound, so an older file isn't played in its place.
        match self {
            Kind::Needs => "seshi-needs-you-1.wav",
            Kind::Done => "seshi-finished-1.wav",
        }
    }

    /// The sound for a user-facing status change, if that change needs the user to act.
    pub fn for_status(status: crate::protocol::Status) -> Option<Self> {
        match status {
            crate::protocol::Status::Blocked => Some(Self::Needs),
            _ => None,
        }
    }
}

const SAMPLE_RATE: u32 = 22_050;
/// How loud the sounds are, of full scale: heard, not startling.
const LEVEL: f32 = 0.3;
/// Each note fades in over this long, so it doesn't click.
const ATTACK_MS: f32 = 6.0;

/// `kind`'s sound as a WAV file's bytes (16-bit mono).
pub fn tone(kind: Kind) -> Vec<u8> {
    let mut pcm: Vec<i16> = Vec::new();
    for (hz, ms) in kind.notes() {
        let n = SAMPLE_RATE * ms / 1000;
        for i in 0..n {
            let t = i as f32 / SAMPLE_RATE as f32;
            // Rings like a struck note: quick in, dying away; an octave above for colour.
            let fade_in = (t * 1000.0 / ATTACK_MS).min(1.0);
            let fade_out = (-4.0 * i as f32 / n as f32).exp();
            let wave = (std::f32::consts::TAU * hz * t).sin() + 0.25 * (std::f32::consts::TAU * hz * 2.0 * t).sin();
            pcm.push((wave / 1.25 * fade_in * fade_out * LEVEL * f32::from(i16::MAX)) as i16);
        }
    }
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    // PCM, one channel.
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// `kind`'s sound as a file the system's player can open: written to seshi's data folder the
/// first time it's needed.
pub fn tone_file(kind: Kind) -> Option<PathBuf> {
    let path = crate::config::data_dir().join(kind.file_name());
    if !path.is_file()
        && let Err(e) = crate::config::write_atomic(&path, tone(kind))
    {
        tracing::warn!("writing {}: {e}", path.display());
        return None;
    }
    Some(path)
}

/// An agent that says the same thing again this soon (it finished, worked a moment longer for
/// a subagent, finished again) isn't news: no second alert.
pub const SAME_AGAIN_AFTER: std::time::Duration = std::time::Duration::from_secs(120);
/// "Finished" waits this long before it's said. An agent running helpers of its own
/// finishes a turn and starts the next by itself, over and over; one still finished after
/// this is really waiting for you.
pub const FINISHED_SETTLES: std::time::Duration = std::time::Duration::from_secs(1);
/// Auto-approved prompts can disappear before a person could respond to them.
const NEEDS_SETTLES: std::time::Duration = std::time::Duration::from_millis(300);

pub fn settle_delay(status: crate::protocol::Status) -> Option<std::time::Duration> {
    match status {
        crate::protocol::Status::Blocked => Some(NEEDS_SETTLES),
        crate::protocol::Status::Done => Some(FINISHED_SETTLES),
        _ => None,
    }
}
/// Several agents at once ring once.
pub const ANY_AGAIN_AFTER: std::time::Duration = std::time::Duration::from_secs(4);

/// Whether an alert for `status` goes now, given the last one for this agent (its status,
/// when) and the last for any agent. Needs-you after a finish is new, so it goes.
pub fn should_alert<S: PartialEq>(status: &S, last_here: Option<(&S, std::time::Instant)>, last_any: Option<std::time::Instant>, now: std::time::Instant) -> bool {
    let same_again = last_here.is_some_and(|(s, at)| s == status && now.saturating_duration_since(at) < SAME_AGAIN_AFTER);
    let crowded = last_any.is_some_and(|at| now.saturating_duration_since(at) < ANY_AGAIN_AFTER);
    !same_again && !crowded
}

/// Play seshi's sound for `kind`, unless sounds are off. Returns at once.
pub fn ring(cfg: &crate::config::Notify, kind: Kind) {
    // Tests drive real status changes: they must not ring the machine they run on.
    if cfg.sound && !cfg!(test) {
        std::thread::spawn(move || play(kind));
    }
}

/// A desktop notification, for when no seshi window is open to say it. `link`: where
/// clicking it goes. Returns at once.
pub fn notify_soon(title: &str, body: &str, link: Option<String>) {
    if cfg!(test) {
        return;
    }
    let (title, body) = (title.to_string(), body.to_string());
    // Its own thread: on Linux it waits for the click.
    std::thread::spawn(move || notify(&title, &body, link.as_deref()));
}

/// An alert on your phone: an ntfy message to your topic. Blocks until sent (run it off the
/// UI thread); what went wrong when it isn't.
pub fn phone(cfg: &crate::config::Notify, title: &str, body: &str) -> Result<(), String> {
    let url = phone_url(&cfg.phone_server, &cfg.phone_topic).ok_or("no phone topic set (notify.phone_topic)")?;
    let one_line = |s: &str| s.replace(['\r', '\n'], " ");
    crate::proc::run(
        crate::proc::system_tool("curl")
            .args(["-fsS", "--max-time", "10", "-H"])
            .arg(format!("Title: {}", one_line(title)))
            .args(["-H", "Tags: seshi", "--data-raw", body])
            .arg(&url),
    )
    .map(|_| ())
}

/// Where a topic's messages go: `<server>/<topic>`. None without a topic.
pub fn phone_url(server: &str, topic: &str) -> Option<String> {
    let topic = topic.trim().trim_matches('/');
    if topic.is_empty() {
        return None;
    }
    let server = if server.trim().is_empty() { "https://ntfy.sh" } else { server.trim().trim_end_matches('/') };
    Some(format!("{server}/{topic}"))
}

/// The phone alert for an agent: who and where always; what it's doing only with `text`.
pub fn phone_message(agent: &str, place: &str, needs_you: bool, doing: &str, text: bool) -> (String, String) {
    let title = format!("{agent} {}", if needs_you { "needs you" } else { "finished" });
    let doing = doing.trim();
    let body = if text && !doing.is_empty() { format!("{place} · {doing}") } else { place.to_string() };
    (title, body)
}

fn quiet(cmd: &mut Command) -> &mut Command {
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    crate::proc::quiet(cmd)
}

/// A desktop notification. Blocks until the tool returns (on Linux, with a link, until the
/// notification is clicked or goes away).
pub fn notify(title: &str, body: &str, link: Option<&str>) {
    if cfg!(windows) {
        // Windows PowerShell (not pwsh) can load the WinRT toast types. Text goes in through
        // the environment, so nothing in it is ever run.
        const TOAST: &str = "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
$x = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
$t = $x.GetElementsByTagName('text'); \
$t.Item(0).AppendChild($x.CreateTextNode($env:SESHI_TITLE)) > $null; \
$t.Item(1).AppendChild($x.CreateTextNode($env:SESHI_BODY)) > $null; \
$a = $x.CreateElement('audio'); $a.SetAttribute('silent', 'true'); $x.DocumentElement.AppendChild($a) > $null; \
if ($env:SESHI_LINK) { $x.DocumentElement.SetAttribute('activationType', 'protocol'); $x.DocumentElement.SetAttribute('launch', $env:SESHI_LINK) }; \
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe').Show([Windows.UI.Notifications.ToastNotification]::new($x))";
        let _ = quiet(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", TOAST]))
            .env("SESHI_TITLE", title)
            .env("SESHI_BODY", body)
            .env("SESHI_LINK", link.unwrap_or_default())
            .status();
    } else if cfg!(target_os = "macos") {
        // macOS's own notifications can't run anything when clicked; terminal-notifier can.
        if let (Some(link), Ok(exe)) = (link, std::env::current_exe())
            && on_path("terminal-notifier")
        {
            let sh = |s: &str| format!("'{}'", s.replace('\'', r"'\''"));
            let run = format!("{} reveal {}", sh(&exe.to_string_lossy()), sh(link));
            if quiet(Command::new("terminal-notifier").args(["-title", title, "-message", body, "-execute", &run])).status().is_ok_and(|s| s.success()) {
                return;
            }
        }
        // AppleScript strings: escape backslashes and quotes.
        let q = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!("display notification \"{}\" with title \"{}\"", q(body), q(title));
        let _ = quiet(Command::new("osascript").args(["-e", &script])).status();
    } else {
        // With a link: an action the notification runs when clicked, and wait to hear it.
        // notify-send before 0.7.9 doesn't know --action; then a plain one.
        if let (Some(link), Ok(exe)) = (link, std::env::current_exe()) {
            let out = crate::proc::quiet(Command::new("notify-send").args(["--app-name=seshi", "--action=default=Open", "--wait", "--", title, body]))
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output();
            if let Ok(out) = out
                && out.status.success()
            {
                if String::from_utf8_lossy(&out.stdout).trim() == "default" {
                    let _ = quiet(Command::new(exe).args(["reveal", link])).status();
                }
                return;
            }
        }
        let _ = quiet(Command::new("notify-send").args(["--app-name=seshi", "--", title, body])).status();
    }
}

/// Whether a program is on PATH.
fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()))
}

/// Play `kind`'s sound. Blocks until it has played.
pub fn play(kind: Kind) {
    let Some(file) = tone_file(kind) else { return };
    if cfg!(windows) {
        // MediaPlayer handles wav, mp3 and wma; it plays asynchronously, so wait a moment.
        const PLAY: &str = "Add-Type -AssemblyName PresentationCore; $p = New-Object System.Windows.Media.MediaPlayer; \
$p.Open([Uri]$env:SESHI_SOUND); $p.Volume = 1; $p.Play(); Start-Sleep -Milliseconds 2500";
        let _ = quiet(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", PLAY]))
            .env("SESHI_SOUND", &file)
            .status();
    } else if cfg!(target_os = "macos") {
        let _ = quiet(Command::new("afplay").arg(&file)).status();
    } else {
        for player in ["paplay", "pw-play"] {
            if quiet(Command::new(player).arg(&file)).status().is_ok_and(|s| s.success()) {
                return;
            }
        }
        let _ = quiet(Command::new("canberra-gtk-play").arg("-f").arg(&file)).status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_needs_you_statuses_make_a_notification_sound() {
        assert_eq!(Kind::for_status(crate::protocol::Status::Blocked), Some(Kind::Needs));
        assert_eq!(Kind::for_status(crate::protocol::Status::Done), None);
        assert_eq!(Kind::for_status(crate::protocol::Status::Working), None);
        assert_eq!(Kind::for_status(crate::protocol::Status::Idle), None);
    }

    #[test]
    fn an_agent_rings_once_for_the_same_news_and_a_crowd_rings_once() {
        use std::time::{Duration, Instant};
        let now = Instant::now();
        assert!(should_alert(&"done", None, None, now), "first time: it goes");
        assert!(!should_alert(&"done", Some((&"done", now - Duration::from_secs(30))), None, now), "done again 30s later: quiet");
        assert!(should_alert(&"needs", Some((&"done", now - Duration::from_secs(30))), None, now), "needs you after done: it goes");
        assert!(should_alert(&"done", Some((&"done", now - Duration::from_secs(300))), None, now), "done again much later: it goes");
        assert!(!should_alert(&"done", None, Some(now - Duration::from_secs(1)), now), "another agent just rang: one ding for both");
    }

    #[test]
    fn the_two_sounds_are_wav_files_you_can_tell_apart() {
        let (needs, done) = (tone(Kind::Needs), tone(Kind::Done));
        for wav in [&needs, &done] {
            assert_eq!((&wav[..4], &wav[8..16]), (&b"RIFF"[..], &b"WAVEfmt "[..]));
            let riff = u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize;
            let data = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
            assert_eq!((riff, data), (wav.len() - 8, wav.len() - 44), "the sizes it declares are its own");
            let samples: Vec<i16> = wav[44..].as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b)).collect();
            let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
            assert!(peak > 2000 && f32::from(peak) <= LEVEL * f32::from(i16::MAX) + 1.0, "audible, and no louder than its level: {peak}");
            assert!(samples[0].unsigned_abs() < 200 && samples[samples.len() - 1].unsigned_abs() < 800, "it starts and ends near silence (no click)");
            let secs = samples.len() as f32 / SAMPLE_RATE as f32;
            assert!((0.2..0.6).contains(&secs), "short: {secs}s");
        }
        assert!(needs != done && Kind::Needs.notes().len() == 2 && Kind::Done.notes().len() == 1, "two notes for needs you, one for finished");
        assert!(Kind::Needs.notes()[0].0 < Kind::Needs.notes()[1].0, "needs you rises");
        assert!(Kind::Needs.file_name() != Kind::Done.file_name());
    }

    #[test]
    fn a_phone_alert_says_who_and_where_and_the_task_only_when_asked() {
        assert_eq!(phone_url("", "seshi-x7k2").as_deref(), Some("https://ntfy.sh/seshi-x7k2"));
        assert_eq!(phone_url("https://ntfy.example.com/", " /seshi-x7k2 ").as_deref(), Some("https://ntfy.example.com/seshi-x7k2"));
        assert_eq!(phone_url("https://ntfy.sh", "  "), None, "no topic: off");
        let m = |text| phone_message("claude", "AeVox", true, "Run npm test?", text);
        assert_eq!(m(false), ("claude needs you".to_string(), "AeVox".to_string()));
        assert_eq!(m(true).1, "AeVox · Run npm test?");
        assert_eq!(phone_message("codex", "shop-api", false, "", true).0, "codex finished");
    }
}
