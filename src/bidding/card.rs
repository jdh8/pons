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
//! * **pons-only** — a convention we play that EPBot's schema has no name for
//!   (`South African Texas`), written on a filler slot under its own name.
//!   Invisible to BBA either way; readable, unlike `Not defined`.
//! * **filler** — the remaining `Not defined = 0` rows and the trailing
//!   `Opponent type = 0`, emitted verbatim to keep the file's shape reviewable.
//!
//! # The card is also the floor's input
//!
//! The card describes the **authored book**, and since the configured net
//! shipped it is *also* what the default floor reads: `american()` encodes this
//! card into [`Config`][crate::bidding::features::Config] and hands it to
//! [`ConfiguredFloorBba`][crate::bidding::neural_floor::ConfiguredFloorBba].  So
//! a row here moves three things at once — what we disclose, what the rules
//! play, and what the net is told it is playing.  That is the point: a knob
//! cannot change the system without the disclosure and the net following.
//!
//! It still cannot express *which* floor is attached, so `american()` and
//! [`american_instinct`][crate::american_instinct] generate the same card and a
//! `--our-floor` swap is not disclosed.  BBA's own defaults approximate the
//! distilled floor; no row exists for it.

use super::agreements::Agreements;
use super::american::{EUROPEAN, LebensohlStyle, NotrumpDefense, NotrumpShape};
use super::instinct::relocating;
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
        for _ in 0..not_defined() {
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
/// [`PONS_SCHEMA`] spends one apiece, so the rendered length never moves.
const NOT_DEFINED: usize = 123;

/// Filler rows left after [`PONS_SCHEMA`] has taken its slots.
const fn not_defined() -> usize {
    NOT_DEFINED - PONS_SCHEMA.len()
}

/// Conventions we play that EPBot's schema has **no name for**, carried on
/// filler slots
///
/// A name EPBot does not know is a *silent no-op* in `epbot_set_conventions`:
/// probed with `South African Texas = 1`, the set does nothing and the get reads
/// back `0` — where a literal `Not defined = 1` does stick, and is merely
/// behaviourally inert.  So these rows are exactly as invisible to BBA as the
/// `Not defined = 0` they replace, while saying what they mean.
///
/// The name is not decoration; a spare slot cannot be documented any other way,
/// and cannot be used *positionally* either.  The format has no comment syntax —
/// `load_bbsa` rejects any non-empty line without ` = `, and BEN's own
/// `BBA.py::load_ccs` unpacks `line.strip().split(' = ')` into exactly two under
/// a bare `except` that calls `sys.exit(1)`, so a `#` line or a blank one kills
/// the process rather than being skipped.  `load_ccs` also keys a **dict** by
/// name, which collapses all 123 identically-named `Not defined` rows into one
/// entry: meaning carried by row index would not survive the loader BBA's own
/// side uses.
///
/// **A name here must not collide with one EPBot knows** — that row would then
/// stick, and silently flip a real BBA convention.  [`SCHEMA`] is our
/// transcription of EPBot's list, and `pons_rows_do_not_shadow_the_schema`
/// enforces disjointness from it; there is no EPBot in `cargo test` to check
/// against directly.
// Alert slugs with **no possible row** (recorded per
// `alerted_call_sites_match_the_disclosure_fixture`): `comp:negative-double`
// and `comp:cue-raise`, the per-overcall direct-seat tables.  EPBot's schema
// toggles *optional* conventions only; the negative double and the limit-plus
// cue raise of their overcall are its unconditional baseline — the same
// agreements we author — so no row exists to set and none is missing.  The
// schema's double rows (Snapdragon, Responsive, Maximal, Support) and its
// `Cue bid` (keycard interference) are different conventions.
const PONS_SCHEMA: &[&str] = &["South African Texas", "Queen ask by available bid"];

/// The value of a [`PONS_SCHEMA`] row under the live knob state
///
/// # Panics
///
/// Panics if `name` is not in [`PONS_SCHEMA`].
fn pons_row(name: &str) -> i32 {
    match name {
        // 1NT - 4♣/4♦ transfers to the majors (`american/notrump.rs`), always on:
        // the knobs beside it set the game-blast floor and the slam-drive
        // reroute, not whether we play it.
        "South African Texas" => 1,
        // The queen relay, one step above the keycard answer.  BBA plays one
        // too — `probe-bba-kickback` asserts its disclosure label, and reads
        // back `hearts queen ask` / `no !H queen` — but the schema carries only
        // *toggleable* conventions and BBA's relay is unconditional, so no row
        // exists to express it.  Ours needs a row all the same: the card has
        // to disclose the relay to the opponents whether or not BBA can toggle
        // it.  Named beside the `King ask by available bid` it feeds.
        "Queen ask by available bid" => 1,
        _ => panic!("`{name}` is in `PONS_SCHEMA` with no value in `pons_row`"),
    }
}

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
/// use pons::bidding::agreements::Agreements;
/// use pons::bidding::card::american_card;
/// use pons::bidding::american::set_nt_splinter;
///
/// let row = |a: &Agreements| american_card(a).row("1N-3M splinter");
/// assert_eq!(row(&Agreements::current()), Some(1)); // shipped default-on
/// set_nt_splinter(false);
/// assert_eq!(row(&Agreements::current()), Some(0));
/// set_nt_splinter(true); // restore the default
/// ```
#[must_use]
///
/// The card is the fourth reader of the knob state, and it must agree with the
/// book and the floor about what we play — a card that discloses a row the
/// rules are not playing is a lie told to the opponents.  One capture feeds
/// all of them.
pub fn american_card(a: &Agreements) -> Card {
    Card {
        // 2/1 game forcing.
        system: 0,
        rows: SCHEMA
            .iter()
            .map(|name| (*name, american_row(name, a)))
            .chain(PONS_SCHEMA.iter().map(|name| (*name, pons_row(name))))
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
pub fn dutch_card(a: &Agreements) -> Card {
    let mut card = american_card(a);
    card.system = 2;
    // Dutch's 1♦ is 5+♦, or exactly the singleton-club 4=4=4=1 (`dutch::openings`);
    // the wide 1♣ soaks up every other four-diamond hand.  American's better-minor
    // 1♦ can be three cards, so this is the one row the two systems disagree on
    // that BBA's schema can express.
    card.set("1D opening with 5 cards", 1);
    card
}

/// The card a **foreign** bidder holds, read one row at a time
///
/// [`american_card`] describes *us* from our own knobs.  This describes someone
/// else — an EPBot seat, say — by asking `read` for every schema row in order,
/// so the caller supplies the engine and this supplies the schema.  Keeping the
/// schema private is the point: a caller enumerating it itself could drift in
/// order or length, and every feature after the drift would shift.
///
/// The pons-only rows are **not** read; they are forced to `0`.  They name
/// conventions EPBot has no row for, deliberately parked on filler slots, so
/// `read` would be answering for a name the engine does not know — and a foreign
/// engine does not play them regardless.
///
/// The result is a full [`LEN_CARD_ROWS`][super::features::LEN_CARD_ROWS]-row
/// card, ready for [`Config::new`][super::features::Config::new].
#[must_use]
pub fn foreign_card(system: i32, mut read: impl FnMut(&str) -> i32) -> Card {
    Card {
        system,
        rows: SCHEMA
            .iter()
            .map(|name| (*name, read(name)))
            .chain(PONS_SCHEMA.iter().map(|name| (*name, 0)))
            .collect(),
    }
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
fn american_row(name: &str, a: &Agreements) -> i32 {
    // The 1NT minor scheme drives six mutually-exclusive rows at once.
    let european = a.decision.reading.notrump_minors() == EUROPEAN;
    match name {
        // ---- computed: a knob or a book fact moves these ----
        //
        // `2♠` transfers to clubs and `2NT` to diamonds under Puppet; European
        // moves the club transfer to `2♠`-natural style with `3♣` = diamonds.
        "1N-2S transfer to clubs" => 1,
        "1N-2N transfer to diamonds" => i32::from(!european),
        "1N-3C transfer to diamonds" => i32::from(european),
        "1N-3C Puppet Stayman" => i32::from(!european),
        "1N-3M splinter" => i32::from(a.decision.reading.nt_splinter()),
        // Either policy can admit the 5422: the shape ladder from `Wide` up, or
        // the off-shape treatment on its own (which admits *any* 5422, not just
        // the five-card-minor ones).
        "1NT opening shape 5422" => i32::from(
            a.opening.notrump_shape != NotrumpShape::Balanced || a.opening.one_notrump_offshape,
        ),
        "1NT opening shape 6 minor" => i32::from(a.opening.notrump_shape == NotrumpShape::Wide6322),
        "Checkback" => i32::from(a.rebid.new_minor_forcing),
        // `set_transfer_super_accept` is **off by default**, so we do not jump
        // super-accept a Jacoby transfer with four-card support and a maximum.
        // The hand-written card declared `= 1` here; that was a claim to a
        // convention we do not play, and generating it fixes the disclosure.
        "Super acceptance after NT" => i32::from(a.notrump.transfer_super_accept),
        "Fourth suit" | "Fourth suit game force" => i32::from(a.rebid.fourth_suit_forcing),
        "Garbage Stayman" => i32::from(a.decision.reading.garbage_stayman()),
        "Jordan Truscott 2NT" => i32::from(a.competition.jordan_truscott),
        // `set_landy` carries the (min, max) two-suiter range when on; the
        // mutually-exclusive direct-seat system lives in `NotrumpDefense`.
        "Landy" => {
            i32::from(a.decision.reading.landy_range().is_some() || a.decision.reading.notrump_defense() == NotrumpDefense::DirectLandy)
        }
        "Multi-Landy" => i32::from(a.decision.reading.notrump_defense() == NotrumpDefense::Woolsey),
        "Leaping Michaels" => i32::from(a.defense.leaping_michaels_enabled),
        "Lebensohl after 1NT" => i32::from(a.competition.lebensohl_style != LebensohlStyle::Off),
        "Rubensohl after double" => i32::from(a.competition.lebensohl_style == LebensohlStyle::Transfer),
        "New Minor Forcing" => i32::from(a.rebid.new_minor_forcing && !a.decision.reading.xyz()),
        "Two Way New Minor Forcing" => i32::from(a.decision.reading.xyz()),
        "Responsive double" => i32::from(a.defense.responsive_takeout_enabled),
        "Support double redouble" => i32::from(a.competition.major_support_double),
        // Systems on when RHO overcalls our 1NT with 2♣ — EPBot's
        // `conventions[156]` is one of the three flags feeding
        // `accepted_LHO_BID_TO_STAYMAN_AND_TRANSFERS`, and the (2♣) systems-on
        // rebase in `competition.rs` rides the same gate as Lebensohl itself.
        "Transfers if RHO bids clubs" => i32::from(a.competition.lebensohl_style != LebensohlStyle::Off),

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
        // `1NT - 3♦` shows both majors (authored; the floor misreads it as natural).
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
        // The shape ladder's wide arms are 5422/6322 *minors*, never 4441; the
        // off-shape treatment is what admits a 4441 (with a singleton Q/J).
        "1NT opening shape 4441" => i32::from(a.opening.one_notrump_offshape),
        // Free bids are forcing (one round at the two level).
        "1X-(Y)-2Z forcing" => 1,
        "1X-(1Y)-2Z strong" | "1X-(1Y)-2Z weak" => 0,
        // RKCB 1430 into the agreed suit; no Crosswood, no Exclusion, and none of
        // the other two keycard orderings.  Whether the ask is *relocated* is a
        // separate row that rides `set_rkcb_variant` — see "Kickback 1430" below.
        "Blackwood 1430" => 1,
        "Blackwood 0123"
        | "Blackwood 0314"
        | "Blackwood without K and Q"
        | "Crosswood 0123"
        | "Crosswood 0314"
        | "Crosswood 1430"
        | "Kickback 0123"
        | "Kickback 0314"
        | "Exclusion" => 0,
        // We play 1430, and under the Kickback stance we play it **relocated**
        // — so this row has to ride the knob.  Hardcoding it to 0 disclosed a
        // system in which our 4♥ is natural while our own side treated it as a
        // diamond ask: an undisclosed convention, which is a fairness problem
        // before it is a measurement one.  It invalidates any kickback-vs-BBA
        // anchor.  Redwood rides the same row: BBA's card has no minors-only
        // relocation, so the nearest disclosure over-claims the hearts lane —
        // a lane where we then bid a plain 4NT it also reads.
        "Kickback 1430" => i32::from(relocating(&a.decision)),
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
        "King ask by 5NT inviting" => 0,
        // The king ask is 5NT off the knob.  On it, the relay carries its own:
        // partner's queen reply already names a king, and the **second relay** —
        // one step above that reply — asks for another, so kickback lowers this
        // ask exactly as it lowers the first one.
        //
        // BBA has no queen-ask *toggle*, but it very much has the ask: it is
        // unconditional inside Blackwood (`get_potencjalne_pytanie_o_dame_krole`,
        // the first available non-trump non-NT bid above the answer), and the
        // row that governs whether it exists at all is `Blackwood without K and
        // Q` — which we already disclose as 0, correctly, because our 1430
        // answers do split on the queen at steps 3/4 and we do own a king ask.
        // So the relay is under-disclosed only in its *shape*, not its
        // existence.  See `docs/ai-bidder/bba-kickback.md` §3.
        // The relay's second ask is the step above the queen reply — the
        // default since 2026-08-02 — so 1 is simply the honest value, and it
        // is free: this row is **inert in BBA** (probed both ways and crossed
        // against `King ask by 5NT`, turning it on is byte-identical to
        // setting no king-ask row at all, `docs/ai-bidder/bba-kickback.md`
        // §3), which is exactly what makes it usable as a slot that describes
        // us rather than instructing them (jdh8).  `King ask by 5NT` stays 1
        // beside it, because that is the row BBA acts on and dropping it
        // would tell them we have no king ask at all.
        "King ask by available bid" => 1,
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
        // `conventions[151]` changes what `1M - 1NT - 2M` and a raise over RHO's
        // double mean, which does not line up cleanly with anything we author —
        // our limit-raise-over-a-double is `Jordan Truscott 2NT`.  Carried at the
        // hand-written `1` rather than guessed at in either direction.
        "Support 1NT" => 1,
        // EPBot pairs the long-suit try with an artificial "shortness
        // somewhere" try (its comment 2483, `!Unspecified shortness, game
        // try`); `set_major_game_tries` authors only the long-suit half plus
        // the general `3M` re-raise, so the second way does not exist however
        // the knob is set and `0` is the honest value whatever it says — the
        // 21GF ledger books row 124 as a gap, not as shipped.  Cosmetic to BBA
        // (0 of 8406 replayed decisions move), and free to our own net since
        // the bias fold zeroed this row's frozen column (the honest value
        // measured −0.0174/−0.0130 per board of pure coordinate noise before
        // it; `docs/ai-bidder/card-manifold.md`).
        "Two way game tries" => 0,
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
mod tests;
