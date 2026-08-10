//! Invariant tests for [`Agreements`](super::Agreements) and its knob areas

/// The `pub`-ish field names of `struct name` in `src`
///
/// A crude line scanner, not a parser: every knob struct in this crate is
/// one field per line with a leading visibility, which is all this needs to
/// see.  Field names are invisible to the type system, so a source scan is
/// the only mechanism that can check the invariant below at all.
fn fields(src: &str, name: &str) -> Vec<String> {
    let body = src
        .split_once(&format!("struct {name} {{"))
        .unwrap_or_else(|| panic!("{name} is declared"))
        .1
        .split_once("\n}")
        .expect("the struct body is closed")
        .0;
    body.lines()
        .filter_map(|line| {
            let line = line.trim_start().strip_prefix("pub")?;
            let line = line.strip_prefix(')').map_or(line, |rest| rest);
            let line = line.trim_start_matches(|c| c != ' ').trim_start();
            let (ident, _) = line.split_once(':')?;
            ident
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                .then(|| ident.to_owned())
        })
        .collect()
}

/// One knob, one home: no setting is a field of both halves of `Agreements`
///
/// A setting read at build time *and* at classify time must live in the
/// classify-time profile alone, with the book reading it from there — see
/// `two_notrump_wide`, `longer_major_response`, `xyz`.  Duplicating it into
/// a `*Knobs` struct creates independently settable copies that can silently
/// diverge.  Twelve knobs were duplicated exactly that way before this test
/// existed.
#[test]
fn no_knob_lives_in_two_homes() {
    let agreements = include_str!("../agreements.rs");
    let build: Vec<String> = [
        "CompetitionKnobs",
        "DefenseKnobs",
        "NotrumpKnobs",
        "OpeningKnobs",
        "ResponseKnobs",
        "RebidKnobs",
        "GameForceKnobs",
        "InstinctKnobs",
    ]
    .iter()
    .flat_map(|name| fields(agreements, name))
    .collect();
    assert!(
        build.len() > 100,
        "the build-time areas were found: {build:?}"
    );

    for (src, name) in [
        (include_str!("../inference/knobs.rs"), "ReadingProfile"),
        (include_str!("../instinct.rs"), "InstinctProfile"),
        (include_str!("../context.rs"), "DecisionProfile"),
    ] {
        let classify = fields(src, name);
        assert!(!classify.is_empty(), "{name} was found");
        let both: Vec<&String> = build.iter().filter(|f| classify.contains(f)).collect();
        assert!(
            both.is_empty(),
            "{name} and the build-time areas share {} knob(s): {both:?} — a dual \
             knob belongs to {name} alone, and the book should read it from \
             the pinned profile",
            both.len(),
        );
    }
}
