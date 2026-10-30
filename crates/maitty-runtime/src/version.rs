/// Parse a release version, accepting a leading `v` and prerelease/build suffixes.
pub(crate) fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version
        .trim()
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u64>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

pub(crate) fn newer_than(tag: &str, current: &str) -> bool {
    matches!((parse(tag), parse(current)), (Some(tag), Some(current)) if tag > current)
}

#[cfg(test)]
mod tests {
    use super::{newer_than, parse};

    #[test]
    fn compares_release_numbers_not_text() {
        assert_eq!(parse("v0.10.2"), Some((0, 10, 2)));
        assert_eq!(parse("1.2.3-beta"), Some((1, 2, 3)));
        assert_eq!(parse("nonsense"), None);
        assert!(newer_than("0.10.0", "0.9.9"));
        assert!(!newer_than("0.9.9", "0.10.0"));
    }
}
