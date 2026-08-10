//! Invariant tests for [`Agreements`](super::Agreements) and its knob areas

use super::Agreements;

/// The literal defaults equal what a virgin thread's cells hold
///
/// The safety net for deleting the cells: `Agreements::default()`
/// transcribes 218 `Cell::new` initialisers into one value, and a
/// transcription error would silently ship a different system.  libtest
/// gives every test its own thread, so `current()` here reads cells nothing
/// has armed.
#[test]
fn build_defaults_match_the_cells() {
    let (d, c) = (Agreements::default(), Agreements::current());
    assert_eq!(d.competition, c.competition);
    assert_eq!(d.defense, c.defense);
    assert_eq!(d.notrump, c.notrump);
    assert_eq!(d.opening, c.opening);
    assert_eq!(d.response, c.response);
    assert_eq!(d.rebid, c.rebid);
    assert_eq!(d.game_force, c.game_force);
    assert_eq!(d.instinct, c.instinct);
    // The catch-all: a field added later is covered here even if nobody adds a
    // line for it above.
    assert!(d == c);
}

/// The classify half's literal defaults equal a virgin thread's cells
///
/// The half [`build_defaults_match_the_cells`] cannot name field by field:
/// `DecisionProfile` and the two profiles it nests carry 85 cells between
/// them, and none of the three derives [`Debug`], so this is one value
/// comparison rather than a list.
#[test]
fn decision_defaults_match_the_cells() {
    let (d, c) = (Agreements::default(), Agreements::current());
    assert!(d.decision == c.decision, "the classify half diverged");
}

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

/// One cell, one home: no knob is a field of both halves of `Agreements`
///
/// A cell read at build time *and* at classify time must live in the
/// classify-time profile alone, with the book reading it from there — see
/// `two_notrump_wide`, `longer_major_response`, `xyz`.  Duplicating it into
/// a `*Knobs` struct is invisible while the `thread_local!` cells still back
/// both captures, since both read the same cell microseconds apart; it turns
/// into a silent divergence the moment the cells go and the two fields
/// become independently settable.  Twelve cells were duplicated exactly that
/// way before this test existed.
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
            "{name} and the build-time areas share {} cell(s): {both:?} — a dual \
             cell belongs to {name} alone, and the book should read it from \
             the pinned profile",
            both.len(),
        );
    }
}
