use super::parse_threads;
use core::num::NonZero;

#[test]
fn unset_empty_and_zero_mean_no_cap() {
    assert_eq!(parse_threads(""), Some(None));
    assert_eq!(parse_threads("  "), Some(None));
    assert_eq!(parse_threads("0"), Some(None));
}

#[test]
fn positive_count_caps_the_pool() {
    assert_eq!(parse_threads("96"), Some(NonZero::new(96)));
    assert_eq!(parse_threads(" 8\n"), Some(NonZero::new(8)));
}

#[test]
fn garbage_is_rejected() {
    assert_eq!(parse_threads("-1"), None);
    assert_eq!(parse_threads("all"), None);
    assert_eq!(parse_threads("9.5"), None);
}
