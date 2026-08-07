use super::*;

fn parse(snapshot: &str) -> serde_json::Value {
    serde_json::from_str(snapshot).expect("snapshot is valid JSON")
}

/// A par contract from a parseable string, its declarer, and its overtricks
fn par_contract(text: &str, declarer: Seat, overtricks: i8) -> pons_dds::ParContract {
    pons_dds::ParContract {
        contract: text.parse().expect("valid contract"),
        declarer,
        overtricks,
    }
}

/// `verdict_lines` renders Result / Par / IMP for the cases that matter:
/// an exact-par tie, a sub-IMP edge, a down result, and both flavors of
/// pass-out (makeable par vs genuinely flat).
#[test]
fn verdict_lines_cases() {
    let n3 = || "3NT".parse::<Contract>().expect("valid contract");
    let none = AbsoluteVulnerability::NONE;
    let par_3nt = |score| Par {
        score,
        contracts: vec![
            par_contract("3NT", Seat::North, 0),
            par_contract("3NT", Seat::South, 0),
        ],
    };

    // Exact par: 3NT-N makes, par is 3NT-NS/400 → 0 IMP, no side.
    assert_eq!(
        verdict_lines(Some((n3(), Seat::North)), Some(9), &par_3nt(400), none),
        [
            "Result: 400 to NS — 3NTN=",
            "Par: 400 to NS — 3NTNS=",
            "0 IMP",
        ],
    );

    // Sub-IMP edge: reached 400 vs a par of 410 (−10) still scores 0 IMP.
    assert_eq!(
        verdict_lines(Some((n3(), Seat::North)), Some(9), &par_3nt(410), none).last(),
        Some(&"0 IMP".to_string()),
    );

    // A down result: 3NT-N down two, nonvul, is −100 (100 to EW).
    assert_eq!(
        verdict_lines(Some((n3(), Seat::North)), Some(7), &par_3nt(400), none)[0],
        "Result: 100 to EW — 3NTN-2",
    );

    // Passed out but 3NT-NS was cold: par shows, EW gains the swing.
    assert_eq!(
        verdict_lines(None, None, &par_3nt(400), none),
        [
            "Result: Passed out",
            "Par: 400 to NS — 3NTNS=",
            "9 IMP to EW",
        ],
    );

    // Genuinely flat: no par contract, no IMP.
    let flat = Par {
        score: 0,
        contracts: Vec::new(),
    };
    assert_eq!(
        verdict_lines(None, None, &flat, none),
        ["Result: Passed out", "Par: Passed out", "0 IMP"],
    );
}

/// The hint prices all five strains while the auction is live, and goes
/// quiet once it ends.  Watching `sd` narrow call by call is the point of
/// the feature, but that is a property of the *reading*, not of this
/// wiring — it belongs in the UI and in an A/B, not in an assertion here.
#[test]
fn hint_prices_every_strain_on_a_live_auction() {
    let mut table = WebTable::new("12345");
    let mut snap = parse(&table.deal_practice("S", "N", "none", 0));

    let rows = parse(&table.hint());
    let rows = rows.as_array().expect("a live auction has a hint");
    assert_eq!(rows.len(), 5, "one row per strain");
    for row in rows {
        let mean = row["mean"].as_f64().expect("mean is a number");
        let sd = row["sd"].as_f64().expect("sd is a number");
        assert!((0.0..=13.0).contains(&mean), "{mean} is a trick count");
        assert!(sd > 0.0 && sd < 6.0, "{sd} is a usable spread");
    }

    while snap["your_turn"] == true {
        let legal = snap["legal"].as_array().expect("legal is an array");
        let call = legal[0].as_str().expect("a legal call").to_string();
        snap = parse(&table.bid(&call));
    }
    assert_eq!(table.hint(), "null", "no hint once the auction has ended");
}

#[test]
fn practice_board_runs_to_completion() {
    let mut table = WebTable::new("12345");
    let mut snap = parse(&table.deal_practice("S", "N", "none", 0));
    assert_eq!(snap["mode"], "practice");
    assert_eq!(snap["seat"], "S");

    let mut human_calls = 0;
    while snap["your_turn"] == true {
        let legal = snap["legal"].as_array().expect("legal is an array");
        assert!(!legal.is_empty(), "legal calls before the auction ends");
        for code in legal {
            let code = code.as_str().expect("legal codes are strings");
            assert!(code.parse::<Call>().is_ok(), "code {code} must re-parse");
        }
        assert_eq!(snap["hands"].as_object().expect("hands").len(), 1);
        snap = parse(&table.bid("P"));
        human_calls += 1;
        assert!(human_calls < 100, "auction must terminate");
    }

    assert_eq!(snap["ended"], true);
    assert!(snap["contract"].is_string());
    assert_eq!(snap["hands"].as_object().expect("hands").len(), 4);
    assert_eq!(
        snap["feedback"].as_array().expect("feedback").len(),
        human_calls,
    );
}

#[test]
fn illegal_and_out_of_turn_bids_are_ignored() {
    let mut table = WebTable::new("7");
    let before = table.deal_practice("S", "S", "ns", 0);
    assert_eq!(table.bid("8♣"), before, "unparseable call is a no-op");
    assert_eq!(table.bid("XX"), before, "illegal call is a no-op");
}

#[test]
fn set_option_reroutes_the_bidding() {
    // North is a balanced 15 — opens 1NT by default, a suit with 1NT off.
    const PBN: &str = "N:AK72.K65.K43.Q82 QJT.AQJ.AQJ.AKJT 986.T987.T98.976 543.432.7652.543";
    let mut table = WebTable::new("1");

    let on = parse(&table.deal_pbn(PBN, "N", "none"));
    assert!(
        on["auction"][0]
            .as_str()
            .expect("opening call")
            .contains('N'),
        "default opens 1NT",
    );

    set_option("open_one_notrump", false);
    let off = parse(&table.deal_pbn(PBN, "N", "none"));
    assert_ne!(
        on["auction"][0], off["auction"][0],
        "toggling the knob changes North's opening",
    );

    set_option("open_one_notrump", true); // restore for a reused test thread
}

#[test]
fn registry_is_well_formed() {
    use std::collections::HashSet;
    // Unique keys — a dup would shadow in the linear find and confuse the UI.
    let mut keys = HashSet::new();
    for setting in SETTINGS {
        assert!(
            keys.insert(setting.key()),
            "duplicate registry key: {}",
            setting.key()
        );
    }
    // describe_options round-trips and matches the table shape one-for-one.
    let json = parse(&describe_options());
    let entries = json.as_array().expect("registry is a JSON array");
    assert_eq!(
        entries.len(),
        SETTINGS.len(),
        "one JSON entry per registry row"
    );
    for entry in entries {
        assert!(entry["key"].is_string() && entry["section"].is_string());
        match entry["kind"].as_str().expect("kind is a string") {
            "toggle" => assert!(entry["default"].is_boolean(), "toggle default is a bool"),
            "choice" => {
                let default = entry["default"]
                    .as_str()
                    .expect("choice default is a string");
                let values: Vec<&str> = entry["variants"]
                    .as_array()
                    .expect("choice has variants")
                    .iter()
                    .map(|v| v["value"].as_str().expect("variant value"))
                    .collect();
                assert!(
                    values.contains(&default),
                    "choice default {default} is a variant"
                );
            }
            other => panic!("unknown kind {other}"),
        }
    }
    // Every `requires` must name a row that exists, in the form that row can
    // satisfy: a bare key must be a toggle, `key=value` a choice with that
    // variant. A typo here renders a permanently-disabled control, which is
    // worse than the ungated lie it replaced.
    for setting in SETTINGS {
        let Some(spec) = setting.requires() else {
            continue;
        };
        let (key, want) = match spec.split_once('=') {
            Some((key, value)) => (key, Some(value)),
            None => (spec, None),
        };
        assert_ne!(key, setting.key(), "{} requires itself", setting.key());
        let master = SETTINGS
            .iter()
            .find(|s| s.key() == key)
            .unwrap_or_else(|| panic!("{} requires missing row {key}", setting.key()));
        match (master, want) {
            (Setting::Toggle { .. }, None) => {}
            (Setting::Choice { variants, .. }, Some(value)) => assert!(
                variants.iter().any(|v| v.value == value),
                "{} requires {key}={value}, not a variant",
                setting.key()
            ),
            _ => panic!("{} requires {spec}, wrong form for that row", setting.key()),
        }
    }
}

/// Every registry `default` must mirror the engine's `Cell::new(...)`.
///
/// Not cosmetic: `app.js` stores only *deltas* against these values, and the
/// Settings reset button pushes them into the engine — so a drifted row rebids
/// the board for anyone who touches the panel. `rich_advance_double`,
/// `rubens_advances` and `fuzzy_fifths` all contradicted the engine for weeks
/// (see `docs/bidding-options.md`); nothing but CI's absence hid them.
///
/// Runs on its own thread because the knobs are thread-locals: that reads a
/// virgin engine whatever order the suite ran in, and dirties nothing for
/// whichever test the runner schedules next on this one.
///
/// Reads each cell back rather than bidding boards and diffing the auctions.
/// The sampling form was built first and measured: flipping each row in turn,
/// 32 of 69 never moved an auction in 600 boards — the slam tries, the runouts,
/// the Stayman follow-ups and the whole inference block need a specific auction
/// that random deals rarely deal. It missed `rubens_advances`, one of the three
/// bugs above, and cost 210 s of CI to miss it. This is exact and instant; the
/// price was widening those getters to `pub`, which is arguably a fix in its own
/// right, since the paired `set_*` were public already.
#[test]
fn registry_defaults_match_the_engine() {
    std::thread::spawn(|| {
        for setting in SETTINGS {
            match setting {
                Setting::Toggle {
                    key, default, get, ..
                } => assert_eq!(get(), *default, "toggle {key} contradicts the engine"),
                Setting::Choice {
                    key, default, get, ..
                } => assert_eq!(get(), *default, "choice {key} contradicts the engine"),
            }
        }
    })
    .join()
    .expect("the registry-drift thread reads a pristine engine");
}

/// Each row's `get` must observe its own `set` — not a neighbour's cell.
///
/// `registry_defaults_match_the_engine` cannot see this: a getter wired to the
/// wrong knob still agrees with `default` whenever the two knobs share one. The
/// getters were generated mechanically against a cell name, so that is exactly
/// the mistake to expect, and `penalize_escape_stack` / `penalize_escape_values`
/// are the type case — adjacent, identically defaulted, one letter apart.
#[test]
fn every_registry_getter_observes_its_own_setter() {
    std::thread::spawn(|| {
        for setting in SETTINGS {
            match setting {
                Setting::Toggle {
                    key, default, get, ..
                } => {
                    set_option(key, !*default);
                    assert_eq!(get(), !*default, "toggle {key} does not read its own cell");
                    set_option(key, *default);
                }
                Setting::Choice {
                    key,
                    default,
                    variants,
                    get,
                    ..
                } => {
                    let other = variants
                        .iter()
                        .map(|v| v.value)
                        .find(|v| v != default)
                        .expect("a choice has a second variant");
                    set_choice(key, other);
                    assert_eq!(get(), other, "choice {key} does not read its own cell");
                    set_choice(key, default);
                }
            }
        }
    })
    .join()
    .expect("the getter-wiring thread runs clean");
}

#[test]
fn set_choice_reroutes_the_defense() {
    // North opens 1NT; East (19 HCP) acts over it — doubles under the natural
    // defense, passes under always-pass.  Selecting the family through set_choice
    // must change East's action.
    const PBN: &str = "N:AK72.K65.K43.Q82 QJT.AQJ.AQJ.AKJT 986.T987.T98.976 543.432.7652.543";
    let mut table = WebTable::new("1");

    set_choice("notrump_defense", "natural");
    let natural = parse(&table.deal_pbn(PBN, "N", "none"));

    set_choice("notrump_defense", "always_pass");
    let always_pass = parse(&table.deal_pbn(PBN, "N", "none"));

    assert_ne!(
        natural["auction"], always_pass["auction"],
        "always-pass defense changes East's action over North's 1NT",
    );

    set_choice("notrump_defense", "natural"); // restore for a reused test thread
}

#[test]
fn set_choice_reroutes_the_notrump_shape() {
    // North is a 16-HCP 6322 with six clubs: opens 1NT under the default
    // wide6322 shape, its minor under balanced-only.  Selecting the family
    // through set_choice must change North's opening.
    const PBN: &str = "N:Q2.K3.AQ4.KQ8765 AKJT9.AQJ.KJT.A9 876.T987.987.JT4 543.6542.6532.32";
    let mut table = WebTable::new("1");

    set_choice("notrump_shape", "wide6322");
    let wide = parse(&table.deal_pbn(PBN, "N", "none"));
    assert!(
        wide["auction"][0]
            .as_str()
            .expect("opening call")
            .contains('N'),
        "wide6322 opens 1NT on a 6322 with a six-card minor",
    );

    set_choice("notrump_shape", "balanced");
    let balanced = parse(&table.deal_pbn(PBN, "N", "none"));
    assert_ne!(
        wide["auction"][0], balanced["auction"][0],
        "balanced-only shape opens a minor, not 1NT",
    );

    set_choice("notrump_shape", "wide6322"); // restore for a reused test thread
}

#[test]
fn demo_board_bids_out() {
    let mut table = WebTable::new("42");
    let snap = parse(&table.deal_demo("W", "both"));
    assert_eq!(snap["mode"], "demo");
    assert_eq!(snap["vul"], "Both");
    assert_eq!(snap["ended"], true);
    assert_eq!(snap["your_turn"], false);
    assert_eq!(snap["hands"].as_object().expect("hands").len(), 4);
    assert!(snap["auction"].as_array().expect("auction").len() >= 4);
    assert!(snap["contract"].is_string());
}

#[test]
fn deal_pbn_bids_out_a_specified_deal() {
    let mut table = WebTable::new("1");
    // A full deal round-trips through the editor's canonical "N:…" form.
    let pbn = "N:AKT86.4.AJ962.K3 Q9432.KQJ8..AQT8 7.AT3.QT753.J764 J5.97652.K84.952";
    let snap = parse(&table.deal_pbn(pbn, "N", "none"));
    assert_eq!(snap["mode"], "demo");
    assert_eq!(snap["ended"], true, "bots bid the specified deal out");
    assert_eq!(snap["hands"].as_object().expect("hands").len(), 4);
    // The North hand is the one we asked for, not a random deal.
    assert_eq!(snap["hands"]["N"]["spades"], "AKT86");
    assert_eq!(snap["hands"]["E"]["diamonds"], "", "East's diamond void");

    assert_eq!(table.deal_pbn("garbage", "N", "none"), "null");
    assert_eq!(
        table.deal_pbn(
            "N:AK.4.AJ962.K3 Q9432.KQJ8..AQT8 7.AT3.QT753.J764 J5.97652.K84.952",
            "N",
            "none"
        ),
        "null",
        "a non-full deal is rejected",
    );
}

#[test]
fn dd_table_solves_revealed_demo_board() {
    let mut table = WebTable::new("42");
    assert_eq!(table.dd_table(), "null", "no board yet");
    let _ = table.deal_demo("N", "none");

    let start = std::time::Instant::now();
    let dd: serde_json::Value = serde_json::from_str(&table.dd_table()).expect("dd JSON");
    eprintln!("dd_table (full 5x4, cold): {:?}", start.elapsed());

    assert_eq!(dd["seats"], serde_json::json!(["W", "N", "E", "S"]));
    let rows = dd["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 5);
    for row in rows {
        let tricks = row["tricks"].as_array().expect("tricks");
        assert_eq!(tricks.len(), 4);
        assert!(tricks.iter().all(|t| t.as_u64().expect("u8") <= 13));
    }
    // The verdict the JS renders line-by-line: Result / Par / IMP.
    let verdict = dd["verdict"].as_array().expect("verdict is an array");
    assert_eq!(verdict.len(), 3);
    assert!(verdict[0].as_str().expect("line").starts_with("Result:"));
    assert!(verdict[1].as_str().expect("line").starts_with("Par:"));
    // Cached: the second call is the same JSON, instantly
    let again: serde_json::Value = serde_json::from_str(&table.dd_table()).expect("cached dd JSON");
    assert_eq!(dd, again);
}

/// The Evaluate tab's ground truth: fix N-S, reshuffle E-W, and read the
/// conditional distribution off the histogram. Two disjoint thirteen-card
/// hands in, a distribution over 0..=13 tricks out, accumulating across the
/// chunks the UI calls it in.
#[test]
fn binky_verdict_accumulates_and_is_conditional() {
    // A flat 25-count facing itself: it should land near nine notrump tricks
    // and, being flat, should not spread far.
    let mut binky = Binky::create("AK32.QJ4.T98.762", "Q54.AK5.QJ42.A83", true, "7")
        .expect("two disjoint thirteen-card hands");

    let first: serde_json::Value = serde_json::from_str(&binky.run(6)).expect("verdict JSON");
    assert_eq!(first["n"], 6);
    let again: serde_json::Value = serde_json::from_str(&binky.run(6)).expect("verdict JSON");
    assert_eq!(again["n"], 12, "the histogram accumulates across chunks");

    let counts: Vec<u32> = serde_json::from_value(again["histogram"].clone()).expect("counts");
    assert_eq!(counts.len(), 14);
    assert_eq!(
        counts.iter().sum::<u32>(),
        12,
        "every layout lands in a bin"
    );

    // The whole point is that this is *conditional*: N-S hold 25 HCP, so the
    // unconditional mean of ~6.5 tricks must not be what comes back.
    let mean = again["mean"].as_f64().expect("mean");
    assert!(
        (7.0..=11.0).contains(&mean),
        "25 HCP should take ~9 tricks, got {mean}"
    );
    assert!(
        again["sd"].as_f64().expect("sd") < 3.0,
        "a flat pair is not wild"
    );
}

/// Overlapping or short hands must be refused rather than solved as nonsense.
#[test]
fn binky_rejects_impossible_hands() {
    assert!(
        Binky::create("AK32.QJ4.T98.762", "AK32.QJ4.T98.762", true, "1").is_none(),
        "the same hand twice is not a deal"
    );
    assert!(
        Binky::create("AK32.QJ4.T98.76", "Q54.AK5.QJ42.A83", true, "1").is_none(),
        "twelve cards is not a hand"
    );
    assert!(Binky::create("nonsense", "Q54.AK5.QJ42.A83", true, "1").is_none());
}

#[test]
fn oracle_accumulates_over_reshuffles() {
    // Seeded so the practice board (human passing throughout) ends in a
    // bot contract: seed 12345 ends in 2NT by N (see the test above).
    let mut table = WebTable::new("12345");
    let mut snap = parse(&table.deal_practice("S", "N", "none", 0));
    for _ in 0..100 {
        if snap["your_turn"] != true {
            break;
        }
        snap = parse(&table.bid("P"));
    }
    assert_eq!(snap["ended"], true);
    if !snap["contract"].is_string() || snap["contract"] == "Passed out" {
        panic!("seed no longer yields a contract; pick a new seed");
    }

    let start = std::time::Instant::now();
    let o: serde_json::Value = serde_json::from_str(&table.oracle(5)).expect("oracle JSON");
    eprintln!("oracle (5 shuffles, 1 strain): {:?}", start.elapsed());

    assert_eq!(o["n"], 5);
    let o2: serde_json::Value = serde_json::from_str(&table.oracle(5)).expect("oracle JSON");
    assert_eq!(o2["n"], 10, "stats accumulate across chunks");
    let pct = o2["makes_pct"].as_f64().expect("pct");
    assert!((0.0..=100.0).contains(&pct));
    assert!(o2["tricks_min"].as_u64() <= o2["tricks_max"].as_u64());
}

#[test]
fn oracle_is_practice_only() {
    let mut table = WebTable::new("42");
    let _ = table.deal_demo("N", "none");
    assert_eq!(
        table.oracle(1),
        "null",
        "demo has no bidding side to be fair to"
    );
}

#[test]
fn book_is_json_with_described_nodes() {
    let nodes: serde_json::Value = serde_json::from_str(&book()).expect("book is valid JSON");
    let nodes = nodes.as_array().expect("book is an array");
    assert!(
        nodes.len() > 100,
        "expected >100 nodes, got {}",
        nodes.len()
    );
    for node in nodes {
        assert!(
            !node["rules"].as_array().expect("rules").is_empty() || node["note"].is_string(),
            "every node has rules or a note: {node}",
        );
    }
}

/// The 1NT-overcall systems-on graft renders under **every** opening — not
/// just the one that wins the pointer dedup.  Each `(1x) 1NT` re-roots the
/// same grafted `Arc`s, so a book display keyed on the pointer alone showed
/// only spades; the seat-invariant-auction key restores all four.
#[test]
fn book_renders_1nt_overcall_advances_per_opening() {
    let nodes: serde_json::Value = serde_json::from_str(&book()).expect("book is valid JSON");
    let nodes = nodes.as_array().expect("book is an array");
    for opening in ["1♣", "1♦", "1♥", "1♠"] {
        // The advancer's response menu after their opening, our 1NT overcall,
        // RHO pass: "1x 1NT -" (Pass renders as "-").
        let heading = format!("{opening} 1NT -");
        assert!(
            nodes
                .iter()
                .any(|node| { node["book"] == "defensive" && node["auction"] == heading }),
            "systems-on advance node {heading:?} must render",
        );
    }
}

/// The competitive book renders: guarded fallbacks surface as entries with
/// the guard's condition folded into the auction heading.
#[test]
fn book_renders_the_competitive_fallbacks() {
    let nodes: serde_json::Value = serde_json::from_str(&book()).expect("book is valid JSON");
    let competitive: Vec<&serde_json::Value> = nodes
        .as_array()
        .expect("book is an array")
        .iter()
        .filter(|node| node["book"] == "competitive")
        .collect();
    assert!(
        competitive.len() > 30,
        "expected >30 competitive entries, got {}",
        competitive.len()
    );
    // A `FirstIs` guard's condition, e.g. "1♣ X …".  This used to test the
    // direct-seat overcall package's "(overcall ≤2♠)" ceiling, which is gone:
    // that guard dissolved into one exact table per overcall.
    assert!(
        competitive
            .iter()
            .any(|node| node["auction"].as_str().expect("auction").ends_with('…')),
        "a guarded section renders with its guard condition in the heading"
    );
    assert!(
        competitive.iter().any(|node| matches!(
            node["note"].as_str(),
            Some(note) if note.contains("systems on")
        )),
        "a systems-on rebase renders as a note"
    );
}
