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

/// `{:#?}` flattened to `path.to.field -> value`, multi-line values joined
///
/// Debug pretty-print is one field per line, four spaces per level, a
/// multi-line value (`Some(`, a tuple) closed by `)` at the opener's indent.
fn flat(agreements: &super::Agreements) -> std::collections::BTreeMap<String, String> {
    let text = format!("{agreements:#?}");
    let mut out = std::collections::BTreeMap::new();
    let mut path: Vec<&str> = Vec::new();
    let mut open: Option<(String, usize, String)> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let body = line.trim();
        if let Some((key, at, value)) = &mut open {
            value.push_str(body);
            if indent == *at && body.starts_with(')') {
                out.insert(format!("{}.{key}", path.join(".")), value.clone());
                open = None;
            }
            continue;
        }
        if body.ends_with('{') {
            path.push(
                body.trim_end_matches(" {")
                    .split(':')
                    .next()
                    .expect("a name"),
            );
        } else if body.starts_with('}') {
            path.pop();
        } else if let Some((key, value)) = body.split_once(": ") {
            if value.ends_with('(') {
                open = Some((key.to_owned(), indent, value.to_owned()));
            } else {
                out.insert(format!("{}.{key}", path.join(".")), value.to_owned());
            }
        }
    }
    out
}

/// `basic()` moves exactly the knobs `docs/system-variants.md` §4 lists
///
/// A future default-on gadget cannot join the beginner card unnoticed, and a
/// knob renamed under the constructor shows up here, not in a web preset.
#[test]
fn basic_differs_from_default_only_on_the_listed_fields() {
    let default = flat(&super::Agreements::default());
    let basic = flat(&super::Agreements::basic());
    assert_eq!(default.len(), basic.len(), "same field set");
    let moved: Vec<&str> = default
        .iter()
        .filter(|(key, value)| basic[*key] != **value)
        .map(|(key, _)| key.as_str())
        .collect();
    assert_eq!(
        moved,
        [
            "Agreements.competition.competition_over_diamond_transfer",
            "Agreements.competition.competition_over_minor_transfer",
            "Agreements.competition.competition_over_stayman",
            "Agreements.competition.landy_minor_slam_answer",
            "Agreements.competition.lebensohl_style",
            "Agreements.competition.multi_kokish_kraft",
            "Agreements.competition.multi_minor_slam_try",
            "Agreements.decision.reading.crawling_stayman",
            "Agreements.decision.reading.garbage_stayman",
            "Agreements.decision.transfer_gf_hearts",
            "Agreements.decision.transfer_gf_majors",
            "Agreements.defense.advance_2nt_continuation_enabled",
            "Agreements.defense.advance_minor_jump_enabled",
            "Agreements.defense.advance_sohl_style",
            "Agreements.defense.leaping_michaels_enabled",
            "Agreements.defense.longest_first_advance_enabled",
            "Agreements.defense.responsive_takeout_enabled",
            "Agreements.defense.rich_advance_double_enabled",
            "Agreements.defense.xx_dont_runout",
            "Agreements.notrump.diamond_splinter",
            "Agreements.notrump.invitational_5card_majors",
            "Agreements.notrump.minor_transfer_slam_try",
            "Agreements.notrump.stayman_both_majors",
            "Agreements.notrump.stayman_cue_continuation",
            "Agreements.notrump.stayman_minor_slam_try",
            "Agreements.notrump.texas_slam_drive",
            "Agreements.notrump.transfer_longer_major",
            "Agreements.notrump.transfer_slam_try",
            "Agreements.rebid.forcing_nt_two_suiter",
            "Agreements.response.drury",
            "Agreements.response.drury_splinters",
            "Agreements.response.major_choice_of_games",
            "Agreements.response.two_over_one_fit",
        ]
    );
}

#[test]
fn advanced_is_the_default() {
    assert_eq!(super::Agreements::advanced(), super::Agreements::default());
}
