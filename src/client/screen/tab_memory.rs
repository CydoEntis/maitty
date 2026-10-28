//! A session's tabs outlive the window. The daemon keeps every pane and the splits between
//! them but has no tabs, so which shell is whose tab is saved with the other choices the
//! window remembers (`Saved::session_tabs`) and put back when the window starts.

use super::*;
use crate::layout::Node;

/// The tabs that are not their session's own, as `(session, the tab's first pane)` in tab
/// order: all it takes to put them back.
pub(super) fn remember(tabs: &[ScreenTab]) -> Vec<(TermId, TermId)> {
    tabs.iter().filter(|t| !t.layout.contains(t.owner)).map(|t| (t.owner, t.layout.first_leaf())).collect()
}

/// `tabs` (the splits the daemon restored) with the remembered tabs back in their sessions,
/// each session's own tab first. A remembered pane that has since ended is skipped.
pub(super) fn restore(mut tabs: Vec<ScreenTab>, remembered: &[(TermId, TermId)], alive: impl Fn(TermId) -> bool) -> Vec<ScreenTab> {
    for &(owner, first) in remembered.iter().filter(|(owner, first)| owner != first && alive(*owner) && alive(*first)) {
        let tab = match tabs.iter().position(|t| t.layout.first_leaf() == first) {
            Some(i) => tabs.remove(i),
            None => ScreenTab { layout: Node::Leaf(first), focus: first, owner, used: 0, name: String::new() },
        };
        tabs.push(ScreenTab { owner, ..tab });
    }
    let mut owners: Vec<TermId> = Vec::new();
    for tab in &tabs {
        if !owners.contains(&tab.owner) {
            owners.push(tab.owner);
        }
    }
    for owner in owners {
        if alive(owner) && !tabs.iter().any(|t| t.layout.contains(owner)) {
            let at = tabs.iter().position(|t| t.owner == owner).unwrap_or(tabs.len());
            tabs.insert(at, ScreenTab { layout: Node::Leaf(owner), focus: owner, owner, used: 0, name: String::new() });
        }
    }
    tabs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(owner: TermId, leaves: &[TermId]) -> ScreenTab {
        let layout = leaves[1..].iter().fold(Node::Leaf(leaves[0]), |mut node, id| {
            node.split(leaves[0], crate::layout::Dir::Right, *id);
            node
        });
        ScreenTab { layout, focus: leaves[0], owner, used: 0, name: String::new() }
    }

    #[test]
    fn a_sessions_shell_tabs_come_back_behind_its_own() {
        // Session 1 had its own tab and two shells (5, then 6 split with 7); 9 is another session.
        let before = [tab(1, &[1]), tab(1, &[5]), tab(1, &[6, 7]), tab(9, &[9])];
        let kept = remember(&before);
        assert_eq!(kept, [(1, 5), (1, 6)], "only the tabs that aren't a session's own");
        // After a restart the daemon gives back only the split, owned by its first pane.
        let after = restore(vec![tab(6, &[6, 7])], &kept, |_| true);
        let shape: Vec<(TermId, Vec<TermId>)> = after.iter().map(|t| (t.owner, t.layout.leaves())).collect();
        assert_eq!(shape, [(1, vec![1]), (1, vec![5]), (1, vec![6, 7])], "its own tab first, the others in the order they were");
    }

    #[test]
    fn a_tab_whose_pane_ended_is_not_brought_back() {
        let after = restore(vec![tab(1, &[1])], &[(1, 5)], |id| id != 5);
        assert_eq!(after.len(), 1, "no tab for a pane that is gone: {after:?}");
        let after = restore(Vec::new(), &[(1, 5)], |id| id != 1);
        assert!(after.iter().all(|t| t.owner != 1 || t.layout.first_leaf() != 1), "and no tab is made for a session that ended");
    }
}
