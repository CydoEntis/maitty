//! Whether an agent's tool calls in a turn are one call over and over, or two taking turns.
//!
//! The daemon sees each call only as a fingerprint (`ClientMsg::Hook::call`), so this can say that
//! a call repeats and never what the call was.

/// One call this many times in a row is repeating.
pub(super) const SAME_CALLS_IN_A_ROW: u32 = 5;
/// Two calls taking turns, this many calls long, is repeating.
pub(super) const TURN_TAKING_CALLS: u32 = 8;
/// The length of a fingerprint, in hex characters.
const FINGERPRINT_HEX: usize = 16;

/// A fingerprint a hook reported, checked: exactly 16 hex characters, else `None`.
pub(super) fn parse_call(call: &str) -> Option<u64> {
    if call.len() != FINGERPRINT_HEX || !call.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(call, 16).ok()
}

/// The tail of a turn's calls, enough to tell a repeat as it grows.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct CallRun {
    last: Option<u64>,
    before: Option<u64>,
    same: u32,
    taking_turns: u32,
}

impl CallRun {
    /// The agent started this call.
    pub(super) fn push(&mut self, call: u64) {
        self.same = if self.last == Some(call) { self.same + 1 } else { 1 };
        self.taking_turns = match (self.last, self.before) {
            (Some(last), Some(before)) if last != call && before == call => self.taking_turns + 1,
            (Some(last), _) if last != call => 2,
            _ => 1,
        };
        self.before = self.last;
        self.last = Some(call);
    }

    /// How many calls the repeat has reached, while one is going on.
    pub(super) fn repeats(&self) -> Option<u32> {
        if self.same >= SAME_CALLS_IN_A_ROW {
            Some(self.same)
        } else if self.taking_turns >= TURN_TAKING_CALLS {
            Some(self.taking_turns)
        } else {
            None
        }
    }

    /// A new prompt, the turn ending, or the agent leaving: nothing carries over.
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(calls: &[u64]) -> CallRun {
        let mut run = CallRun::default();
        for call in calls {
            run.push(*call);
        }
        run
    }

    #[test]
    fn four_of_the_same_call_is_nothing_and_five_is_five() {
        assert_eq!(run(&[7, 7, 7, 7]).repeats(), None);
        assert_eq!(run(&[7, 7, 7, 7, 7]).repeats(), Some(5));
    }

    #[test]
    fn it_grows_while_the_same_call_goes_on() {
        assert_eq!(run(&[7; 6]).repeats(), Some(6));
        assert_eq!(run(&[7; 12]).repeats(), Some(12));
    }

    #[test]
    fn a_different_call_clears_it() {
        assert_eq!(run(&[7, 7, 7, 7, 7, 8]).repeats(), None);
    }

    #[test]
    fn five_of_one_call_after_others_counts_from_where_it_began() {
        assert_eq!(run(&[1, 2, 3, 7, 7, 7, 7, 7]).repeats(), Some(5));
    }

    #[test]
    fn two_calls_taking_turns_for_eight_calls_is_eight() {
        let turns = |n: usize| (0..n).map(|i| if i % 2 == 0 { 1 } else { 2 }).collect::<Vec<u64>>();
        assert_eq!(run(&turns(7)).repeats(), None);
        assert_eq!(run(&turns(8)).repeats(), Some(8));
        assert_eq!(run(&turns(9)).repeats(), Some(9));
    }

    #[test]
    fn taking_turns_breaks_when_a_third_call_comes() {
        let mut calls: Vec<u64> = (0..8).map(|i| if i % 2 == 0 { 1 } else { 2 }).collect();
        calls.push(3);
        assert_eq!(run(&calls).repeats(), None);
    }

    #[test]
    fn many_different_calls_are_never_a_repeat() {
        assert_eq!(run(&(0..40).collect::<Vec<u64>>()).repeats(), None);
    }

    #[test]
    fn clearing_forgets_everything() {
        let mut calls = run(&[7; 6]);
        calls.clear();
        assert_eq!(calls.repeats(), None);
        calls.push(7);
        assert_eq!(calls.repeats(), None, "one call after a clear is the first");
    }

    #[test]
    fn a_fingerprint_is_sixteen_hex_characters_and_nothing_else() {
        assert_eq!(parse_call("00112233445566ff"), Some(0x0011_2233_4455_66ff));
        assert_eq!(parse_call("ffffffffffffffff"), Some(u64::MAX));
        for bad in ["", "0011", "00112233445566f", "00112233445566fff", "0011223344556zzz", "+0112233445566ff", "00112233 45566ff", "０011223344556677"] {
            assert_eq!(parse_call(bad), None, "{bad:?}");
        }
    }
}
