//! What the daemon's usage store reports to a client: provider-reported numbers per session
//! per day, added up over a period. Every number is what Claude Code or Codex said; nothing is
//! estimated, and a figure nobody reported is `None`, never zero.

use crate::protocol::TermId;
use serde::{Deserialize, Serialize};

/// How far back a report reaches. Windows are rolling and end today (UTC), so "week" is the
/// last 7 days including today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsagePeriod {
    Day,
    Week,
    Month,
}

impl UsagePeriod {
    /// The days the period covers, today included.
    pub fn days(self) -> u64 {
        match self {
            UsagePeriod::Day => 1,
            UsagePeriod::Week => 7,
            UsagePeriod::Month => 30,
        }
    }
}

/// The figures of one session, one day, or a whole period.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageFigures {
    /// Input tokens the provider reported; `None` when it reported none in the period.
    pub tokens_in: Option<u64>,
    pub tokens_out: Option<u64>,
    /// US dollars the provider reported; `None` for a subscription session or an agent that
    /// reports no cost (Codex).
    pub cost: Option<f64>,
    /// Turns the agent finished.
    pub turns: u64,
    /// Seconds the agent spent working across those turns.
    pub working_secs: u64,
}

/// One session's usage over the period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsage {
    /// The agent's own session id (a pane-local id when the agent never reported one).
    pub session: String,
    /// The pane showing the session right now, when it is still open.
    pub term: Option<TermId>,
    /// "claude" or "codex".
    pub agent: String,
    /// The folder the session worked in, by name.
    pub project: String,
    #[serde(flatten)]
    pub figures: UsageFigures,
    /// First and last day (`YYYY-MM-DD`, UTC) with usage in the period.
    pub first_day: String,
    pub last_day: String,
}

/// One day's usage across all sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayUsage {
    /// `YYYY-MM-DD`, UTC.
    pub day: String,
    #[serde(flatten)]
    pub figures: UsageFigures,
}

/// The answer to `Query::UsageReport`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReport {
    pub period: UsagePeriod,
    /// First and last day of the window, `YYYY-MM-DD` UTC, both included.
    pub from: String,
    pub to: String,
    /// How many days the store keeps; older usage is gone.
    pub retention_days: u32,
    pub totals: UsageFigures,
    pub session_count: u32,
    /// Sessions in the window that reported no cost (the page shows one note, not one per row).
    pub sessions_without_cost: u32,
    /// Most cost first, then most working time.
    pub sessions: Vec<SessionUsage>,
    /// Oldest first, only days with usage.
    pub days: Vec<DayUsage>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ClientMsg, Query, Reply, ServerMsg, Usage, decode, encode};

    fn report() -> UsageReport {
        UsageReport {
            period: UsagePeriod::Week,
            from: "2026-10-29".into(),
            to: "2026-11-04".into(),
            retention_days: 90,
            totals: UsageFigures {
                tokens_in: Some(1200),
                tokens_out: Some(300),
                cost: None,
                turns: 4,
                working_secs: 600,
            },
            session_count: 1,
            sessions_without_cost: 1,
            sessions: vec![SessionUsage {
                session: "abc".into(),
                term: Some(3),
                agent: "codex".into(),
                project: "shop".into(),
                figures: UsageFigures::default(),
                first_day: "2026-11-03".into(),
                last_day: "2026-11-04".into(),
            }],
            days: vec![DayUsage {
                day: "2026-11-04".into(),
                figures: UsageFigures::default(),
            }],
        }
    }

    #[test]
    fn a_report_round_trips_on_the_wire_and_names_its_fields_for_the_window() {
        let bytes = encode(&ServerMsg::Reply(Reply::UsageReport(report()))).unwrap();
        let ServerMsg::Reply(Reply::UsageReport(back)) = decode(&bytes).unwrap() else {
            panic!("expected a usage report")
        };
        assert_eq!(back, report());
        let json = serde_json::to_value(report()).unwrap();
        assert_eq!(json["totals"]["tokensIn"], 1200);
        assert!(json["totals"]["cost"].is_null(), "unreported cost is null");
        assert_eq!(json["sessions"][0]["workingSecs"], 0);
        assert_eq!(json["sessions"][0]["firstDay"], "2026-11-03");
    }

    /// A new query is a new enum variant: a peer built before it can't read it, which is why
    /// adding one bumps `PROTOCOL_VERSION` (and the two refuse to talk).
    #[test]
    fn a_peer_from_before_the_usage_report_cannot_read_it() {
        #[derive(Debug, serde::Deserialize)]
        #[allow(dead_code)]
        enum OldQuery {
            List,
        }
        #[derive(Debug, serde::Deserialize)]
        #[allow(dead_code)]
        enum OldClientMsg {
            Query(OldQuery),
        }
        let bytes = encode(&ClientMsg::Query(Query::UsageReport {
            period: UsagePeriod::Month,
        }))
        .unwrap();
        assert!(decode::<OldClientMsg>(&bytes).is_err());
        let ClientMsg::Query(Query::UsageReport { period }) = decode(&bytes).unwrap() else {
            panic!("expected a usage report query")
        };
        assert_eq!(period, UsagePeriod::Month);
    }

    #[test]
    fn a_usage_from_an_older_peer_has_no_token_split() {
        let bytes =
            rmp_serde::to_vec_named(&std::collections::BTreeMap::from([("tokens", 9u64)])).unwrap();
        let usage: Usage = decode(&bytes).expect("only the total");
        assert_eq!(
            (usage.tokens, usage.tokens_in, usage.tokens_out),
            (Some(9), None, None)
        );
    }
}
