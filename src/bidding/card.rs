//! `.bbsa` convention cards generated from the live system
//!
//! A `.bbsa` card is how BBA/EPBot is told what a pair plays: a `System type`
//! header selecting the base system, then one `Name = value` row per convention.
//! `bba-gen --disclose` replays a card onto the seats pons occupies, so the BBA
//! opponents read our alerts instead of taking us for a BBA.
//!
//! The card used to be hand-written, and it drifted: `set_nt_splinter` shipped
//! default-on while `cards/American.bbsa` still declared `1N-3M splinter = 0`.
//! A hand-maintained file also cannot describe an **A/B arm** — an arm that flips
//! a knob plays one system and discloses another.  So the card is *generated*
//! here from the live thread-local knob state, and `cards/*.bbsa` are golden
//! snapshots that `the_checked_in_cards_match_the_generator` asserts equal the
//! generator at crate defaults.  Regenerate with
//! `cargo run --example bba-card -- --system american > cards/American.bbsa`.
//!
//! Rows come in three kinds:
//!
//! * **computed** — a `set_*` knob or a book fact moves the value, so the row
//!   reads that knob.  A row is computed *iff* something can move it; that is
//!   the whole discipline, and it is what makes drift impossible rather than
//!   merely detectable.
//! * **constant** — a convention we do not author and no knob toggles
//!   (`Ghestem`, `Benjamin 2D`, `Kickback`).  Written literally, with the
//!   reason.
//! * **filler** — the 123 `Not defined = 0` rows and the trailing
//!   `Opponent type = 0`, emitted verbatim to keep the file's shape reviewable.
//!
//! # Limitation
//!
//! The card describes the **authored book**.  It cannot express floor behaviour,
//! so `american()` and [`american_instinct`][crate::american_instinct] generate
//! the same card, and a `--our-floor` swap is not disclosed.  The default floor
//! is BBA-distilled, so BBA's own defaults approximate it; no row exists for it.

use super::american::{
    EUROPEAN, LebensohlStyle, NotrumpDefense, NotrumpShape, fourth_suit_forcing, garbage_stayman,
    jordan_truscott, landy_range, leaping_michaels_enabled, lebensohl_style, major_game_tries,
    major_support_double, new_minor_forcing, notrump_defense, notrump_minors,
    notrump_shape_setting, nt_splinter, responsive_takeout_enabled, transfer_super_accept, xyz,
};
use core::fmt;

/// A generated `.bbsa` convention card
///
/// `Display` renders the exact file format: the `System type` header, one
/// `Name = value` row per convention, the `Not defined` filler, and the trailing
/// `Opponent type` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    /// EPBot's base-system id: 0 = 2/1 GF, 1 = SAYC, 2 = WJ (Polish),
    /// 3 = Precision Club, 4 = Acol.  The only channel for facts no row
    /// expresses — notably the shape and strength of a wide 1♣.
    pub system: i32,
    /// Every convention row, in the card's fixed schema order
    pub rows: Vec<(&'static str, i32)>,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "System type = {}", self.system)?;
        for (name, value) in &self.rows {
            writeln!(f, "{name} = {value}")?;
        }
        for _ in 0..NOT_DEFINED {
            writeln!(f, "Not defined = 0")?;
        }
        writeln!(f, "Opponent type = 0")
    }
}

/// Positional filler rows between the last named convention and `Opponent type`
///
/// BBA writes the card at a fixed length; the unused slots all carry the name
/// `Not defined`.  They are inert (the loader passes them through by name) but
/// preserved so a generated card diffs cleanly against one BBA exported.
const NOT_DEFINED: usize = 123;

/// Every convention row name, in the card's fixed schema order
///
/// Shared by every system — the vendored cards in `vendor/bba/` all carry these
/// same names in this same order, and only the values differ.
const SCHEMA: &[&str] = &[
    "(1X)-1Y-(1Z)-2Z natural",
    "1D opening with 4 cards",
    "1D opening with 5 cards",
    "1m opening allows 5M",
    "1M-3M blocking",
    "1M-3M inviting",
    "1N-2S Minor Suit Stayman",
    "1N-2S transfer to clubs",
    "1N-2N transfer to clubs",
    "1N-2N transfer to diamonds",
    "1N-3C transfer to diamonds",
    "1N-3C Puppet Stayman",
    "1N-3D majors",
    "1N-3D minors",
    "1N-3D natural",
    "1N-3D splinter",
    "1N-3M splinter",
    "1NT opening natural",
    "1NT opening NT style",
    "1NT opening range 12-14",
    "1NT opening range 13-15",
    "1NT opening range 14-16",
    "1NT opening range 15-17",
    "1NT opening shape 4441",
    "1NT opening shape 5422",
    "1NT opening shape 6 minor",
    "1X-(Y)-2Z forcing",
    "1X-(1Y)-2Z strong",
    "1X-(1Y)-2Z weak",
    "2N-3C-3N both majors",
    "2N-3C Puppet Stayman",
    "4NT opening",
    "5431 after 1NT",
    "5NT pick a slam",
    "Benjamin 2D",
    "Bergen",
    "Blackwood 0123",
    "Blackwood 0314",
    "Blackwood 1430",
    "Blackwood without K and Q",
    "BROMAD",
    "Cappelletti",
    "Checkback",
    "Crosswood 0123",
    "Crosswood 0314",
    "Crosswood 1430",
    "Cue bid",
    "DEPO",
    "Direct Jump Cuebid",
    "DOPI",
    "Drury",
    "Exclusion",
    "Extended Stayman",
    "Extended acceptance after NT",
    "Fit showing jumps",
    "Forcing 1NT",
    "Fourth suit",
    "Fourth suit game force",
    "French 2D",
    "Gambling",
    "Garbage Stayman",
    "Gazzilli",
    "Gerber",
    "Gerber only for NT openings",
    "Ghestem",
    "Imposible 2S",
    "Inverted minors",
    "Inviting Jump Shifts",
    "Jacoby 2NT",
    "Jordan Truscott 2NT",
    "Kickback 0123",
    "Kickback 0314",
    "Kickback 1430",
    "King ask by 5NT",
    "King ask by 5NT inviting",
    "King ask by available bid",
    "Landy",
    "Leaping Michaels",
    "Lebensohl after 1NT",
    "Lebensohl after 1m",
    "Lebensohl after double",
    "Maximal Doubles",
    "Michaels Cuebid",
    "Mini Splinter",
    "Minor Suit Slam Try after 2NT",
    "Minor Suit Stayman after 2NT",
    "Minor Suit Transfers after 2NT",
    "Mixed raise",
    "Multi",
    "Multi-Landy",
    "Namyats",
    "Natural 3N entering style",
    "New Minor Forcing",
    "Non-Leaping Michaels",
    "Ogust",
    "Polish two suiters",
    "Quantitative 4NT",
    "Raptor 1NT",
    "Responsive double",
    "Reverse Bergen",
    "Reverse drury",
    "ROPI",
    "Rubensohl after 1NT",
    "Rubensohl after 1m",
    "Rubensohl after double",
    "Semi forcing 1NT",
    "Shape Bergen structure",
    "SMOLEN",
    "Snapdragon Double",
    "Soloway Jump Shifts",
    "Soloway Jump Shifts Extended",
    "Splinter",
    "Strength Lawrence structure",
    "Super acceptance after NT",
    "Support 1NT",
    "Support double redouble",
    "Surplus pass",
    "Texas",
    "Transfers if RHO passes",
    "Transfers if RHO doubles",
    "Transfers if RHO bids clubs",
    "Two suit takeout double",
    "Two way game tries",
    "Two Way New Minor Forcing",
    "Unusual 1NT",
    "Unusual 2NT",
    "Unusual 3NT",
    "Unusual 4NT",
    "Weak Jump Shifts 2",
    "Weak Jump Shifts 3",
    "Weak natural 2D",
    "Weak natural 2M",
    "Wilkosz",
];

/// The card for [`american`][crate::american()], read off the live knob state
///
/// Call it after any `set_*` you want disclosed; every row that a knob can move
/// reads that knob here, so an A/B arm discloses the system it actually plays.
///
/// ```
/// use pons::bidding::card::american_card;
/// use pons::bidding::american::set_nt_splinter;
///
/// assert_eq!(american_card().row("1N-3M splinter"), Some(1)); // shipped default-on
/// set_nt_splinter(false);
/// assert_eq!(american_card().row("1N-3M splinter"), Some(0));
/// set_nt_splinter(true); // restore the default
/// ```
#[must_use]
pub fn american_card() -> Card {
    Card {
        // 2/1 game forcing.
        system: 0,
        rows: SCHEMA
            .iter()
            .map(|name| (*name, american_row(name)))
            .collect(),
    }
}

/// The card for [`dutch`][crate::dutch()]
///
/// [`dutch`][crate::dutch()] overwrites only the divergent nodes of
/// `american_book()` and carries no `set_*` knobs of its own, so its card is
/// [`american_card`] plus a short override list — and it inherits every knob the
/// American rows read.
///
/// The header is **WJ** (Wspólny Język, the Polish club), not 2/1: no row in the
/// schema expresses "1♣ is 2+ cards, 11–23 HCP, non-forcing", so the header is
/// the only channel for the system's defining feature.  Declaring 2/1 would tell
/// BBA the one thing about Dutch that is most false.  WJ's own conventions that
/// we do not play are overridden back to the American values by construction —
/// every row not listed below keeps its American value.
#[must_use]
pub fn dutch_card() -> Card {
    let mut card = american_card();
    card.system = 2;
    // Dutch's 1♦ is 5+♦, or exactly the singleton-club 4=4=4=1 (`dutch::openings`);
    // the wide 1♣ soaks up every other four-diamond hand.  American's better-minor
    // 1♦ can be three cards, so this is the one row the two systems disagree on
    // that BBA's schema can express.
    card.set("1D opening with 5 cards", 1);
    card
}

impl Card {
    /// The value of one row, or [`None`] if the schema has no such row
    #[must_use]
    pub fn row(&self, name: &str) -> Option<i32> {
        self.rows
            .iter()
            .find(|(row, _)| *row == name)
            .map(|(_, value)| *value)
    }

    /// Overwrite one row in place
    ///
    /// # Panics
    ///
    /// If `name` is not in the card's schema — a typo in an override list is a
    /// silent misdescription otherwise.
    pub fn set(&mut self, name: &str, value: i32) {
        let row = self
            .rows
            .iter_mut()
            .find(|(row, _)| *row == name)
            .unwrap_or_else(|| panic!("`{name}` is not a row of the .bbsa schema"));
        row.1 = value;
    }
}

/// One American row's value, off the live knobs
///
/// Exhaustive over [`SCHEMA`]: a row added there without a value here panics
/// rather than silently disclosing a zero.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per card row is the mapping; splitting it hides it"
)]
fn american_row(name: &str) -> i32 {
    // The 1NT minor scheme drives six mutually-exclusive rows at once.
    let european = notrump_minors() == EUROPEAN;
    match name {
        // ---- computed: a knob or a book fact moves these ----
        //
        // `2♠` transfers to clubs and `2NT` to diamonds under Puppet; European
        // moves the club transfer to `2♠`-natural style with `3♣` = diamonds.
        "1N-2S transfer to clubs" => 1,
        "1N-2N transfer to diamonds" => i32::from(!european),
        "1N-3C transfer to diamonds" => i32::from(european),
        "1N-3C Puppet Stayman" => i32::from(!european),
        "1N-3M splinter" => i32::from(nt_splinter()),
        "1NT opening shape 5422" => i32::from(notrump_shape_setting() != NotrumpShape::Balanced),
        "1NT opening shape 6 minor" => i32::from(notrump_shape_setting() == NotrumpShape::Wide6322),
        "Checkback" => i32::from(new_minor_forcing()),
        // `set_transfer_super_accept` is **off by default**, so we do not jump
        // super-accept a Jacoby transfer with four-card support and a maximum.
        // The hand-written card declared `= 1` here; that was a claim to a
        // convention we do not play, and generating it fixes the disclosure.
        "Super acceptance after NT" => i32::from(transfer_super_accept()),
        "Fourth suit" | "Fourth suit game force" => i32::from(fourth_suit_forcing()),
        "Garbage Stayman" => i32::from(garbage_stayman()),
        "Jordan Truscott 2NT" => i32::from(jordan_truscott()),
        // `set_landy` carries the (min, max) two-suiter range when on; the
        // mutually-exclusive direct-seat system lives in `NotrumpDefense`.
        "Landy" => {
            i32::from(landy_range().is_some() || notrump_defense() == NotrumpDefense::DirectLandy)
        }
        "Multi-Landy" => i32::from(notrump_defense() == NotrumpDefense::Woolsey),
        "Leaping Michaels" => i32::from(leaping_michaels_enabled()),
        "Lebensohl after 1NT" => i32::from(lebensohl_style() != LebensohlStyle::Off),
        "Rubensohl after double" => i32::from(lebensohl_style() == LebensohlStyle::Transfer),
        "New Minor Forcing" => i32::from(new_minor_forcing() && !xyz()),
        "Two Way New Minor Forcing" => i32::from(xyz()),
        "Responsive double" => i32::from(responsive_takeout_enabled()),
        "Support double redouble" => i32::from(major_support_double()),
        "Two way game tries" => i32::from(major_game_tries()),

        // Systems on when RHO overcalls our 1NT with 2♣ — EPBot's
        // `conventions[156]` is one of the three flags feeding
        // `accepted_LHO_BID_TO_STAYMAN_AND_TRANSFERS`, and the (2♣) systems-on
        // rebase in `competition.rs` rides the same gate as Lebensohl itself.
        "Transfers if RHO bids clubs" => i32::from(lebensohl_style() != LebensohlStyle::Off),

        // ---- constant: we author these (or pointedly do not), and no knob moves them ----
        //
        // Better minor: `1♦` can be three cards, so neither length row holds.
        // Five-card majors, so `1m` never hides one.
        "1D opening with 4 cards" | "1D opening with 5 cards" | "1m opening allows 5M" => 0,
        // Limit raises, not blocking jumps.
        "1M-3M blocking" => 0,
        "1M-3M inviting" => 1,
        // Puppet-scheme rows the European variant does not reach; `2♠` is never
        // Minor Suit Stayman and `2NT` is never clubs under either scheme.
        "1N-2S Minor Suit Stayman" | "1N-2N transfer to clubs" => 0,
        // `1NT-3♦` shows both majors (authored; the floor misreads it as natural).
        "1N-3D majors" => 1,
        "1N-3D minors" | "1N-3D natural" | "1N-3D splinter" => 0,
        // Our 1NT is a natural 15-17.  This pair is one mutually-exclusive radio
        // group and it describes the **responses**, not the opening: `(0, 1)` is
        // "responses are conventional", which is what makes BBA read our `2♥` as
        // a transfer.  See `docs/ai-bidder/bba-card-audit.md`.
        "1NT opening natural" => 0,
        "1NT opening NT style" => 1,
        "1NT opening range 15-17" => 1,
        "1NT opening range 12-14" | "1NT opening range 13-15" | "1NT opening range 14-16" => 0,
        // The wide shapes are 5422/6322 minors, never 4441.
        "1NT opening shape 4441" => 0,
        // Free bids are forcing (one round at the two level).
        "1X-(Y)-2Z forcing" => 1,
        "1X-(1Y)-2Z strong" | "1X-(1Y)-2Z weak" => 0,
        // RKCB 1430 into the agreed suit; no Kickback, no Crosswood, no Exclusion.
        "Blackwood 1430" => 1,
        "Blackwood 0123"
        | "Blackwood 0314"
        | "Blackwood without K and Q"
        | "Crosswood 0123"
        | "Crosswood 0314"
        | "Crosswood 1430"
        | "Kickback 0123"
        | "Kickback 0314"
        | "Kickback 1430"
        | "Exclusion" => 0,
        // EPBot's `conventions[58]` is the *side-suit* super-accept: after a
        // transfer, opener bids a feature above the completion (its gate wants
        // 4+ support with a doubleton or shorter somewhere).  `notrump.rs` only
        // ever authors the plain jump — the fit-/shortness-showing forms are a
        // deliberate omission there — so this row is 0 whatever
        // `set_transfer_super_accept` says.
        "Extended acceptance after NT" => 0,
        // Cue bids, DOPI/ROPI/DEPO over their interference in a keycard
        // auction (the floor's authored rungs in `instinct.rs` — DOPI below
        // five of trump, DEPO at or above), and the 5NT king ask.
        "Cue bid" | "DOPI" | "ROPI" | "DEPO" | "King ask by 5NT" => 1,
        "King ask by 5NT inviting" | "King ask by available bid" => 0,
        // Michaels and Unusual 2NT we author outright; `set_unusual_notrump_defense`
        // gates only our *defense* to theirs, not our own two-suiter bids.
        "Michaels Cuebid" | "Unusual 2NT" => 1,
        // Floor territory, and the reason this module documents that limitation.
        // Each fires only where the book is silent — `Unusual 1NT` wants a passed
        // hand or a 20+ opponent (our authored `1NT` overcall is natural 15–18, so
        // it cannot be the caller there), `Unusual 4NT` and the two-suited takeout
        // double want a two-suiter already shown.  The BBA-distilled floor bids
        // those seats, and every vendor card including BBA's own sets all three,
        // so `1` describes our bidder better than `0` would.
        "Unusual 1NT" | "Unusual 4NT" | "Two suit takeout double" => 1,
        "Unusual 3NT" | "Non-Leaping Michaels" | "Ghestem" | "Polish two suiters" | "Wilkosz" => 0,
        // The forcing 1NT response and its two-suiter rebids; not semi-forcing.
        "Forcing 1NT" => 1,
        "Semi forcing 1NT" => 0,
        // Inverted minor raises, Jacoby 2NT, splinters, Smolen, Texas, quantitative
        // 4NT, minor transfers after 2NT, Ogust over a weak two, Lebensohl's
        // strength structure, weak twos in both majors and diamonds.
        "Inverted minors"
        | "Jacoby 2NT"
        | "Splinter"
        | "SMOLEN"
        | "Texas"
        | "Quantitative 4NT"
        | "Ogust"
        | "Weak natural 2D"
        | "Weak natural 2M" => 1,
        // Radio pair, like the `1NT opening natural` / `NT style` group: EPBot's
        // setter for either index clears the other, so never flip one alone.
        //
        // Despite the names, neither is about raises — both live in EPBot's
        // `rebid_po_forsujacym_21` (`21` is *two-over-one*), opener's rebid
        // after a game-forcing `2/1` response, and both gate the **three-level
        // side suit** specifically.  `Shape` bids it on any strength;
        // `Strength` demands roughly `25 - partner_min + 1` and reads that
        // strength back off the call.  `game_force::opener_rebid` offers
        // `call(3, x)` on `len(x, 4..)` with no point term at all, so we are
        // Shape — every strength-showing call there is a separate rule (the
        // `3M` jump wants `points(15..)`, the raise wants support).
        "Shape Bergen structure" => 1,
        "Strength Lawrence structure" => 0,
        // Unresolved, and cosmetic (zero decisions moved).  EPBot's
        // `conventions[151]` changes what `1M`–`1NT`–`2M` and a raise over RHO's
        // double mean, which does not line up cleanly with anything we author —
        // our limit-raise-over-a-double is `Jordan Truscott 2NT`.  Carried at the
        // hand-written `1` rather than guessed at in either direction.
        "Support 1NT" => 1,
        // Not authored.  Each is a real convention BBA can play and we do not;
        // a zero here is a claim, not a default.
        "(1X)-1Y-(1Z)-2Z natural"
        | "2N-3C-3N both majors"
        | "2N-3C Puppet Stayman"
        | "4NT opening"
        | "5431 after 1NT"
        | "5NT pick a slam"
        | "Benjamin 2D"
        | "Bergen"
        | "BROMAD"
        | "Cappelletti"
        | "Direct Jump Cuebid"
        | "Drury"
        | "Extended Stayman"
        | "Fit showing jumps"
        | "French 2D"
        | "Gambling"
        | "Gazzilli"
        | "Gerber"
        | "Gerber only for NT openings"
        | "Imposible 2S"
        | "Inviting Jump Shifts"
        | "Lebensohl after 1m"
        | "Lebensohl after double"
        | "Maximal Doubles"
        | "Mini Splinter"
        | "Minor Suit Slam Try after 2NT"
        | "Minor Suit Stayman after 2NT"
        // Responder's 2NT structure is 3♣ Stayman, 3♦/3♥ to the majors, 3NT,
        // 4NT quantitative (`notrump_responses_at_three`) — the minors have no
        // transfer, at any level.  The 1NT minor transfers are a different row.
        | "Minor Suit Transfers after 2NT"
        | "Mixed raise"
        | "Multi"
        | "Namyats"
        | "Natural 3N entering style"
        | "Raptor 1NT"
        | "Reverse Bergen"
        | "Reverse drury"
        | "Rubensohl after 1NT"
        | "Rubensohl after 1m"
        | "Snapdragon Double"
        | "Soloway Jump Shifts"
        | "Soloway Jump Shifts Extended"
        | "Surplus pass"
        | "Transfers if RHO passes"
        | "Transfers if RHO doubles" => 0,
        // Responder's single jump in a new suit is weak — 6+ cards, 2–5 points
        // (`responses.rs`, `wjs_bid`), at whichever level the jump lands: `2♠`
        // over `1♥`, the 3 level below the major.  EPBot splits the two levels
        // into `conventions[166]`/`[167]`, both of which we want.  No knob —
        // this is the response structure, not a treatment.
        "Weak Jump Shifts 2" | "Weak Jump Shifts 3" => 1,
        _ => panic!("`{name}` is in the .bbsa schema with no value in `american_row`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::american::{set_notrump_minors, set_nt_splinter, set_xyz};

    /// The checked-in cards are current
    ///
    /// This is the gate the whole module exists for: ship a convention, forget
    /// the card, and this goes red.  `cards/*.bbsa` are snapshots for humans and
    /// for `--disclose FILE`; the generator is the source of truth.  Bless with
    /// `cargo run --example bba-card -- --system american >cards/American.bbsa`
    /// (and `--system dutch >cards/Dutch.bbsa`).
    #[test]
    fn the_checked_in_cards_match_the_generator() {
        assert_eq!(
            american_card().to_string(),
            include_str!("../../cards/American.bbsa"),
            "cards/American.bbsa is stale — re-bless it (see this test's doc)",
        );
        assert_eq!(
            dutch_card().to_string(),
            include_str!("../../cards/Dutch.bbsa"),
            "cards/Dutch.bbsa is stale — re-bless it (see this test's doc)",
        );
    }

    #[test]
    fn schema_has_no_duplicate_rows() {
        let mut names: Vec<_> = SCHEMA.to_vec();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate row name in the .bbsa schema");
    }

    #[test]
    fn the_rendered_card_has_the_cards_full_length() {
        // Header + named rows + filler + `Opponent type`, as BBA writes it.
        let text = american_card().to_string();
        assert_eq!(text.lines().count(), 1 + SCHEMA.len() + NOT_DEFINED + 1);
        assert_eq!(text.lines().next(), Some("System type = 0"));
        assert_eq!(text.lines().next_back(), Some("Opponent type = 0"));
    }

    #[test]
    fn a_knob_moves_its_row() {
        set_nt_splinter(false);
        assert_eq!(american_card().row("1N-3M splinter"), Some(0));
        set_nt_splinter(true);
        assert_eq!(american_card().row("1N-3M splinter"), Some(1));

        set_xyz(false);
        assert_eq!(american_card().row("Two Way New Minor Forcing"), Some(0));
        set_xyz(true);
        assert_eq!(american_card().row("Two Way New Minor Forcing"), Some(1));

        // The minor scheme is a radio group: exactly one of Puppet `3♣` and the
        // European `3♣`-diamond transfer is ever live.
        set_notrump_minors(EUROPEAN);
        let card = american_card();
        assert_eq!(card.row("1N-3C Puppet Stayman"), Some(0));
        assert_eq!(card.row("1N-3C transfer to diamonds"), Some(1));
        assert_eq!(card.row("1N-2N transfer to diamonds"), Some(0));
        set_notrump_minors(crate::bidding::american::PUPPET);
    }

    #[test]
    fn dutch_differs_from_american_in_the_diamond_opening() {
        let (american, dutch) = (american_card(), dutch_card());
        assert_eq!(dutch.system, 2, "Dutch declares the WJ base");
        let moved: Vec<_> = SCHEMA
            .iter()
            .filter(|name| american.row(name) != dutch.row(name))
            .collect();
        assert_eq!(moved, [&"1D opening with 5 cards"]);
    }

    #[test]
    #[should_panic(expected = "not a row of the .bbsa schema")]
    fn setting_an_unknown_row_panics() {
        american_card().set("Ghestem Cuebid", 1);
    }
}
