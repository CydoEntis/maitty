//! Usage and cost (`u` in key mode, the sidebar's menu): what each agent session has used and
//! the plan limits, exactly as the providers reported them to the daemon (ADR 0009). Nothing is
//! estimated here; a figure the provider did not report is left out.

use super::*;
use crate::protocol::{Limit, Usage};

/// The modal's width and the rows it shows sessions in before `+ N more`.
const USAGE_W: u16 = 84;
const SESSIONS_SHOWN: usize = 12;

/// One agent session's line: its name and what it reported.
struct UsageRow {
    state: status_mark::State,
    name: String,
    usage: Usage,
}

fn rows(app: &App) -> Vec<UsageRow> {
    let model = app.screen_model();
    let mut rows: Vec<UsageRow> = app
        .all_terms()
        .into_iter()
        .filter(|(_, i)| i.agent.is_some())
        .map(|(term, info)| UsageRow {
            state: pane_state(app, info, term),
            name: session_name(app, &model, term),
            usage: info.usage.clone(),
        })
        .collect();
    rows.sort_by(|a, b| b.usage.cost.unwrap_or(0.0).total_cmp(&a.usage.cost.unwrap_or(0.0)));
    rows
}

/// Tokens as people read them: `950`, `12.3k`, `1.2M`.
fn tokens(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..1_000_000 => format!("{:.1}k", n as f64 / 1_000.0),
        _ => format!("{:.1}M", n as f64 / 1_000_000.0),
    }
}

/// What a session reported, in the order a reader compares them; empty when it reported none.
fn figures(u: &Usage) -> String {
    let mut parts = Vec::new();
    if let Some(c) = u.context {
        parts.push(format!("context {c:.0}%"));
    }
    if let Some(n) = u.tokens {
        parts.push(format!("{} tokens", tokens(n)));
    }
    if let Some(c) = u.cost {
        parts.push(format!("${c:.2}"));
    }
    parts.join(" · ")
}

/// A plan limit: `5h 40% · resets 14:00`.
fn limit_words(l: &Limit) -> String {
    format!("{} {:.0}% · resets {}", l.name, l.used, crate::clock::local_hm(l.resets_at))
}

pub(in crate::client) fn draw_usage(app: &mut App, f: &mut Frame, area: Rect, t: &Theme) {
    let rows = rows(app);
    let limits = app.snap.limits.clone();
    let shown = rows.len().min(SESSIONS_SHOWN);
    let more = rows.len() - shown;
    let limit_rows = limits.len();
    let body = shown.max(1) + usize::from(more > 0) + 2 + limit_rows.max(1);
    let h = body as u16 + 5;
    let buf = f.buffer_mut();
    dim_all(app, buf, area, t);
    let r = panel(app, buf, area, USAGE_W, h, "usage and cost", &[], t);
    let ground = Style::default().bg(t.card);
    let (x, end) = (r.x + 3, r.right().saturating_sub(3));
    let mut y = r.y + 2;
    if rows.is_empty() {
        put(buf, x, y, &[seg("No agent sessions yet.", ground.fg(t.muted))], end);
        y += 1;
    }
    for row in rows.iter().take(shown) {
        let mark = app.state_mark(row.state, t);
        let said = figures(&row.usage);
        let said = if said.is_empty() { "nothing reported".to_string() } else { said };
        let right = said.width() as u16;
        let name_end = end.saturating_sub(right + 2);
        put(buf, x, y, &[seg(mark.glyph, ground.fg(mark.color)), seg(" ", ground)], end);
        put(buf, x + 2, y, &[seg(truncate(&row.name, name_end.saturating_sub(x + 2) as usize), ground.fg(t.strong))], name_end);
        put(buf, end.saturating_sub(right), y, &[seg(said, ground.fg(t.text))], end);
        y += 1;
    }
    if more > 0 {
        put(buf, x, y, &[seg(format!("+ {more} more"), ground.fg(t.muted))], end);
        y += 1;
    }
    y += 1;
    put(buf, x, y, &[seg("plan limits", ground.fg(t.muted))], end);
    y += 1;
    if limits.is_empty() {
        put(buf, x, y, &[seg("none reported", ground.fg(t.muted))], end);
    }
    for (agent, ls) in &limits {
        let words: Vec<String> = ls.iter().map(limit_words).collect();
        put(
            buf,
            x,
            y,
            &[seg(format!("{agent:<8}"), ground.fg(t.strong)), seg(words.join("   "), ground.fg(t.text))],
            end,
        );
        y += 1;
    }
}

impl App {
    /// Any key closes it: there is nothing to choose.
    pub(in crate::client) fn on_usage_key(&mut self, k: &KeyEvent) {
        if matches!(k.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('u' | 'q')) {
            self.mode = Mode::Normal;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_say_only_what_the_provider_reported() {
        assert_eq!(figures(&Usage::default()), "");
        let u = Usage {
            context: Some(42.0),
            cost: Some(0.4213),
            tokens: Some(12_345),
            ..Usage::default()
        };
        assert_eq!(figures(&u), "context 42% · 12.3k tokens · $0.42");
        assert_eq!(tokens(950), "950");
        assert_eq!(tokens(1_200_000), "1.2M");
    }
}
