//! When they compete over *our* notrump convention
//!
//! They double or overcall our Stayman, our Jacoby transfer, our two-way `2♠`
//! or our `2NT` diamond transfer.  Each is keyed under the uncontested
//! `[1NT, P, <our call>]` node — a distinct trie path from the systems-on
//! blocks, where their call sits at depth 1 — and each shares the
//! `X (bid) …` systems-on rebase.
use super::*;

thread_local! {
    /// Whether opener authors continuations after the opponents contest our 2♣
    /// Stayman (`1NT-(P)-2♣-(X)` and `-(2♦/2♥/2♠)`); **on by default**, with an
    /// off-switch for A/B measurement.  See [`set_competition_over_stayman`].
    static COMPETITION_OVER_STAYMAN: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our 2♣ Stayman,
/// for books built *after* this call (thread-local; **on by default**).
///
/// Over a `(X)` (lead-directing clubs) opener answers in the *pass-denies-stopper*
/// coded scheme: a major or `2♦` promises a club stopper, Pass denies one, `XX` is
/// business clubs; responder's `XX` after opener's pass re-asks Stayman (forcing).
/// Over a `(2♦/2♥/2♠)` overcall opener bids a 4-card major naturally if it
/// outranks their suit, doubles for cards, else passes.
pub fn set_competition_over_stayman(on: bool) {
    COMPETITION_OVER_STAYMAN.with(|cell| cell.set(on));
}

/// Whether competition over our 2♣ Stayman is currently authored
fn competition_over_stayman() -> bool {
    COMPETITION_OVER_STAYMAN.with(Cell::get)
}

thread_local! {
    /// Whether opener authors continuations after the opponents contest our Jacoby
    /// transfer (`1NT-(P)-2♦/2♥-(X)` and `-(overcall)`); **off by default** (opt-in
    /// A/B).  See [`set_competition_over_transfer`].
    static COMPETITION_OVER_TRANSFER: Cell<bool> = const { Cell::new(false) };
}

/// Author opener's replies after the opponents double or overcall our Jacoby
/// transfer, for books built *after* this call (thread-local; **off by default**).
///
/// Over a `(X)` opener completes the transfer with three-card support, jump
/// super-accepts with four and a maximum, passes with a doubleton (declining —
/// responder's `XX` then re-asks, forcing), or redoubles with the doubled
/// transfer suit as its own.  Over an overcall opener super-accepts the major
/// with a fit, doubles for cards, else passes.  Opt-in: unlike the contested 2♣
/// Stayman (which won +3.5 IMPs/fired), a paired A/B vs BBA over 640 000 boards
/// found these continuations a DD **loss** (plain −0.94, PD −0.33 IMPs/board it
/// fires on) — the super-accept and forcing re-ask drive us into failing
/// contracts the floor's lower bids avoid — so it stays off by default.
pub fn set_competition_over_transfer(on: bool) {
    COMPETITION_OVER_TRANSFER.with(|cell| cell.set(on));
}

/// Whether competition over our Jacoby transfer is currently authored
fn competition_over_transfer() -> bool {
    COMPETITION_OVER_TRANSFER.with(Cell::get)
}

thread_local! {
    /// Whether opener authors continuations after the opponents contest our two-way
    /// 2♠ minor response (`1NT-(P)-2♠-(X)` and `-(overcall)`); **on by default**,
    /// with an off-switch for A/B measurement.  See
    /// [`set_competition_over_minor_transfer`].
    static COMPETITION_OVER_MINOR_TRANSFER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our two-way 2♠
/// (clubs-or-balanced-invite) response, for books built *after* this call
/// (thread-local; **on by default**).
///
/// Only the PUPPET 2♠ (the default — a club one-suiter *or* the balanced
/// invite that asks opener's size) has a min/max answer to protect, so the block
/// no-ops under the EUROPEAN pure-transfer scheme.  Their `(X)` of 2♠ is
/// lead-directing spades, so opener re-encodes its size-ask answer *and* a spade
/// stopper across four calls: `2NT` = minimum **with** a stopper, `3♣` = maximum
/// **with** one, `Pass` = minimum **no** stopper, `XX` = maximum **no** stopper.
/// After a stopper-showing bid responder's rebids match the uncontested tree
/// (strip the `X` to a Pass); after a no-stopper reply responder signs off in `3♣`
/// with clubs.  A `(2NT)`/`(3♣)` overcall (which steals the size-ask steps) keeps
/// the signal alive — `3NT` = maximum + stopper, `X` = maximum no stopper, Pass =
/// minimum; any higher overcall is systems-off (a `X` showing their suit, else
/// Pass).  Like the contested 2♣ Stayman this is a **constructive** win: a paired
/// A/B vs BBA over 640 000 boards measured **+4.80 IMPs/board it fires on** on plain
/// double-dummy (+5.63 under perfect-defense — *higher*, so it is a sound
/// contract-finding gain, not a doubling artifact), CI excluding 0, so it ships on.
/// Rare (it fired on 0.03 %): BBA seldom contests our 2♠.
pub fn set_competition_over_minor_transfer(on: bool) {
    COMPETITION_OVER_MINOR_TRANSFER.with(|cell| cell.set(on));
}

/// Whether competition over our two-way 2♠ minor response is currently authored
fn competition_over_minor_transfer() -> bool {
    COMPETITION_OVER_MINOR_TRANSFER.with(Cell::get)
}

thread_local! {
    /// Whether opener authors continuations after the opponents contest our 2NT
    /// diamond transfer (`1NT-(P)-2NT-(X)` and `-(overcall)`); **on by default**,
    /// with an off-switch for A/B measurement.  See
    /// [`set_competition_over_diamond_transfer`].
    static COMPETITION_OVER_DIAMOND_TRANSFER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our 2NT diamond
/// transfer (6+♦, or 5♦-4♣), for books built *after* this call (thread-local;
/// **on by default**).
///
/// Only the PUPPET scheme (the default) plays 2NT as the diamond transfer, so the
/// block no-ops under EUROPEAN (where 2NT is the balanced size-ask).  Their `(X)`
/// is lead-directing diamonds; the double frees `Pass` to be the catch-all
/// "no fit" call, which lets opener's `3♣` shed its uncontested
/// relay-denies-a-fit meaning and become **natural** (4+♣, finding responder's
/// 5♦-4♣ club fit): `3♦` = accept with a diamond fit (3+♦), `3♣` = no fit but
/// 4+♣, `XX` = maximum values without a fit (penalty-oriented), `Pass` = minimum
/// catch-all.  After a fit-showing `3♦`/`3♣` responder's rebids match the
/// uncontested tree (strip the `X` to a Pass); after `Pass`/`XX` (no fit)
/// responder always holds 5+♦ and signs off in `3♦`.  An overcall is handled
/// naturally: `3♣` leaves room to complete `3♦` with a fit (else `X` = penalty,
/// Pass = minimum); a higher overcall keeps `3NT` (max + stopper) / `X` (their
/// suit) / Pass.  **On by default** (off-switch `bba-gen
/// --no-ns-comp-over-diamond-transfer`): a paired A/B vs BBA over 1 000 000
/// `--filter-1nt` boards (410 fired, 0.04 %) measured a plain-DD **wash** (+0.24
/// IMPs/board it fires on, CI straddling 0) and a clear perfect-defense gain (+3.40
/// PD).  Unlike the 2♠ minor (which won on *both* scorers), the honest-DD signal is
/// a wash — but it never *loses* on plain DD, and the PD gain is real value the day
/// the opponents punish the floor's `X`-then-pull-to-`3NT` overreach, so it ships on.
pub fn set_competition_over_diamond_transfer(on: bool) {
    COMPETITION_OVER_DIAMOND_TRANSFER.with(|cell| cell.set(on));
}

/// Whether competition over our 2NT diamond transfer is currently authored
fn competition_over_diamond_transfer() -> bool {
    COMPETITION_OVER_DIAMOND_TRANSFER.with(Cell::get)
}

/// Opener's coded reply after the opponents double our 2♣ Stayman
/// (`1NT-(P)-2♣-(X)`)
///
/// The `(X)` is lead-directing clubs, so the *pass-denies-stopper* scheme spends
/// the free pass on a club-stopper signal: a 4-card major (`2♥`/`2♠`) or `2♦`
/// (no major) promises a club stopper; **Pass denies one** (it may still hide a
/// major, shown after responder re-asks); `XX` is business clubs (offer to play
/// 2♣ doubled-redoubled).  Direct XX is business — distinct from responder's
/// SOS/re-ask XX below.
fn stayman_doubled_opener() -> Rules {
    Rules::new()
        .rule(
            Call::Redouble,
            100,
            len(Suit::Clubs, 5..) & suit_hcp(Suit::Clubs, 5..),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            100,
            len(Suit::Hearts, 4..) & stopper_in(Suit::Clubs),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4) & stopper_in(Suit::Clubs),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            50,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4) & stopper_in(Suit::Clubs),
        )
        .rule(Call::Pass, 25, !stopper_in(Suit::Clubs))
}

/// Responder's re-ask after opener passed our doubled Stayman to deny a club
/// stopper (`1NT-(P)-2♣-(X)-P-(P)`)
///
/// Balancing XX is SOS, not business: `XX` re-asks Stayman (forcing — responder
/// still holds the 4-card major), and opener must answer (`stayman_answers`, no
/// Pass).  An owning Pass is the always-mass catch-all.
fn stayman_redouble_reask() -> Rules {
    Rules::new()
        .rule(
            Call::Redouble,
            100,
            len(Suit::Hearts, 4..) | len(Suit::Spades, 4..),
        )
        .alert(STAYMAN_REDOUBLE)
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's natural reply after the opponents overcall our 2♣ Stayman at the
/// 2-level (`1NT-(P)-2♣-(2♦/2♥/2♠)`)
///
/// Show the 4-card major if it outranks their suit; else `X` shows length in
/// their suit (cards/penalty — and, when they overcalled the very major opener
/// holds, the major opener could not bid); else Pass.  Responder stays captain.
fn stayman_overcalled_opener(over: Suit) -> Rules {
    let mut rules = Rules::new();
    if (Suit::Hearts as u8) > (over as u8) {
        rules = rules.rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..));
    }
    if (Suit::Spades as u8) > (over as u8) {
        rules = rules.rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        );
    }
    rules
        .rule(Call::Double, 60, len(over, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's reply after the opponents double our Jacoby transfer
/// (`1NT-(P)-2♦/2♥-(X)`)
///
/// The transfer is still a command, but the `(X)` buys opener a meaningful pass:
/// **complete** (bid `major`) with three-card support, **jump super-accept**
/// (`3-major`) with four and a maximum, **Pass** with a doubleton (declines —
/// responder re-asks below), or `XX` when the doubled transfer suit (`bid`) is
/// opener's own and it wants to defend.
fn transfer_doubled_opener(major: Suit, bid: Suit) -> Rules {
    let strain = Strain::from(major);
    let mut rules = Rules::new();
    if transfer_super_accept() {
        rules = rules.rule(Bid::new(3, strain), 150, len(major, 4..) & hcp(17..));
    }
    rules
        .rule(Bid::new(2, strain), 100, len(major, 3..))
        .rule(Call::Redouble, 60, len(bid, 5..) & suit_hcp(bid, 5..))
        .rule(Call::Pass, 25, len(major, ..3))
}

/// Responder's re-ask after opener passed our doubled transfer
/// (`1NT-(P)-2♦/2♥-(X)-P-(P)`)
///
/// Opener's pass declined the transfer; responder still holds the five-card
/// major, so `XX` insists opener complete (forcing — opener answers with
/// [`complete_transfer`], no Pass).  An owning Pass is the catch-all.
fn transfer_pass_reask(major: Suit) -> Rules {
    Rules::new()
        .rule(Call::Redouble, 100, len(major, 5..))
        .alert(TRANSFER_REDOUBLE)
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's reply after the opponents overcall our Jacoby transfer
/// (`1NT-(P)-2♦/2♥-(overcall)`)
///
/// Super-accept the `major` at the cheapest level above their `over_suit` with
/// four-card support; else `X` shows length in their suit (cards); else Pass.
/// Responder stays captain.
fn transfer_overcalled_opener(major: Suit, over_suit: Suit, over_level: u8) -> Rules {
    let strain = Strain::from(major);
    let lvl = if strain > Strain::from(over_suit) {
        over_level
    } else {
        over_level + 1
    };
    Rules::new()
        .rule(
            Bid::new(lvl, strain),
            100,
            min_level_is(lvl, strain) & len(major, 4..),
        )
        .rule(Call::Double, 60, len(over_suit, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's coded reply after the opponents double our two-way 2♠
/// (`1NT-(P)-2♠-(X)`)
///
/// Their `X` is lead-directing spades, so opener answers the size-ask *and* shows
/// a spade stopper in one call: `2NT`/`3♣` keep their uncontested min/max meaning
/// and promise a stopper (responder then plays the rebased systems-on tree), while
/// `Pass`/`XX` deny a stopper for the minimum/maximum respectively (responder signs
/// off in clubs below).
fn minor_doubled_opener() -> Rules {
    Rules::new()
        // Maximum + spade stopper: the uncontested `3♣` max answer.
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            hcp(17..) & stopper_in(Suit::Spades),
        )
        // Minimum + spade stopper: the uncontested `2NT` min answer.
        .rule(Bid::new(2, Strain::Notrump), 90, stopper_in(Suit::Spades))
        // Maximum, no stopper: `XX`.
        .rule(Call::Redouble, 80, hcp(17..))
        // Minimum, no stopper: `Pass`.
        .rule(Call::Pass, 25, hcp(0..))
}

/// Responder's placement after opener denied a spade stopper over our doubled 2♠
/// (`1NT-(P)-2♠-(X)-P-(P)` minimum, or `…-XX-(P)` maximum)
///
/// Opener has shown min/max but no stopper, so notrump is off; the six-card club
/// hand signs off in `3♣`.  Pass is the catch-all — the balanced-invite hand has no
/// safe spot and defends the doubled 2♠ (rare; the convention is opt-in).
//
// ponytail: the invite hand passing 2♠-doubled is the known soft spot; refine only
// if an A/B says the no-stopper branch leaks.
fn minor_no_stopper_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Clubs), 80, len(Suit::Clubs, 6..))
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's reply after the opponents overcall our two-way 2♠ at `2NT` or `3♣` —
/// the bids that steal opener's size-ask steps (`1NT-(P)-2♠-(2NT/3♣)`)
///
/// Keep the min/max + stopper signal alive in the room that remains: `3NT` =
/// maximum with a spade stopper (to play), `X` = maximum without one (penalty /
/// values), `Pass` = minimum.
fn minor_overcalled_high() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(17..) & stopper_in(Suit::Spades),
        )
        .rule(Call::Double, 70, hcp(17..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's systems-off reply after the opponents overcall our two-way 2♠ above
/// `3♣` (`1NT-(P)-2♠-(3♦/3♥/3♠)`)
///
/// Their suit is too high to keep the size-ask, so opener falls back to natural
/// competition: `X` shows length in their suit (cards), else Pass and leave
/// responder captain.
fn minor_overcalled_low(over: Suit) -> Rules {
    Rules::new()
        .rule(Call::Double, 60, len(over, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's reply after the opponents double our 2NT diamond transfer
/// (`1NT-(P)-2NT-(X)`)
///
/// `Pass` now carries the "no diamond fit" message (the uncontested job of `3♣`),
/// so opener's `3♣` is freed to be natural 4+♣ (finding responder's 5♦-4♣ fit):
/// `3♦` = accept with 3+♦, `3♣` = no fit but 4+♣, `XX` = maximum values (no fit,
/// penalty-oriented), `Pass` = minimum catch-all.
fn diamond_doubled_opener() -> Rules {
    Rules::new()
        // Accept the transfer with a diamond fit — primary.
        .rule(Bid::new(3, Strain::Diamonds), 100, len(Suit::Diamonds, 3..))
        // No fit but real clubs: natural, lands responder's 5♦-4♣ in the club fit.
        .rule(
            Bid::new(3, Strain::Clubs),
            70,
            len(Suit::Diamonds, ..3) & len(Suit::Clubs, 4..),
        )
        // Maximum without a fit: redouble shows values (penalty-oriented).
        .rule(Call::Redouble, 60, hcp(17..))
        // Catch-all: minimum, no fit, no clubs.
        .rule(Call::Pass, 25, hcp(0..))
}

/// Responder's signoff after opener denied a diamond fit over our doubled 2NT
/// (`1NT-(P)-2NT-(X)-P-(P)` minimum, or `…-XX-(P)` maximum)
///
/// Responder always holds 5+♦ from the transfer, so pull to `3♦` rather than
/// languish in a doubled 2NT; Pass is a near-dead catch-all.
//
// ponytail: a strong responder bidding game over opener's XX is the rare soft
// spot left to the floor — refine only if an A/B says this branch leaks.
fn diamond_no_fit_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 80, len(Suit::Diamonds, 5..))
        .rule(Call::Pass, 10, hcp(0..))
}

/// Opener's reply after the opponents overcall our 2NT diamond transfer at `3♣`
/// (the one overcall that leaves the `3♦` completion legal)
fn diamond_overcalled_low() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, len(Suit::Diamonds, 3..))
        .rule(Call::Double, 60, len(Suit::Clubs, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's reply after the opponents overcall our 2NT diamond transfer above `3♣`
/// (`3♦` cue / `3♥` / `3♠` — the `3♦` completion is gone)
///
/// `3NT` = maximum with a stopper in their suit (to play), `X` = length in their
/// suit (penalty), else Pass and leave responder captain.
fn diamond_overcalled_high(over: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(17..) & stopper_in(over),
        )
        .rule(Call::Double, 60, len(over, 4..))
        .rule(Call::Pass, 20, hcp(0..))
}

/// The `X (bid) …` systems-on rebase, shared by the four
/// competition-over-our-own-convention packages
///
/// They doubled our artificial call and we answered with a bid; from there
/// responder's rebids are the *uncontested* tree, so strip the double to a
/// Pass and re-key.  `bid` is one answer the guard admits — the sample the
/// row layer seat-checks and probes with.
///
/// The guard is a two-call prefix with a free tail, which no named [`Pattern`]
/// construct spells: `Pattern::first("…", "X")` would also swallow the
/// `X (P) (P)` re-ask whose own table is declared just below it, rebasing the
/// re-ask instead of classifying it.  So it rides in verbatim through
/// [`Pattern::guarded`].
fn systems_on_over_double(key: &str, bid: &str) -> Entry {
    rebase(
        Pattern::guarded(
            key,
            &format!("(X) {bid}"),
            described_guard(
                "X (bid) …",
                guard(|_: &Context<'_>, s: &[Call]| {
                    s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
                }),
            ),
        ),
        described_rewrite(
            "systems on: their X is stripped to a pass",
            rewriter(|auction: &[Call], depth: usize| {
                if auction.get(depth) != Some(&Call::Double) {
                    return None;
                }
                let mut rewritten = auction.to_vec();
                rewritten[depth] = Call::Pass; // strip the X → systems on
                Some(rewritten)
            }),
        ),
    )
}

/// Competition over our own `2♣` Stayman as a row package
/// ([`set_competition_over_stayman`], default on)
///
/// Opener's replies after they double `1NT-(P)-2♣-(X)` or overcall it.  Keyed
/// at the `[1NT, P, 2♣]` node — a distinct trie path from the systems-on
/// `[1NT, (2♣)]` block, where their `2♣` sits at depth 1.
pub(super) fn competition_over_stayman_package() -> Package {
    Package {
        name: "competition-over-stayman",
        gate: competition_over_stayman,
        entries: || {
            const STAYMAN: &str = "P* 1NT (P) 2♣";
            // A.1 — our Stayman doubled.  Opener's coded reply, then the
            // systems-on rebase off his stopper-bid.
            let mut entries = rows_of(Pattern::after(STAYMAN, "(X)"), stayman_doubled_opener());
            entries.push(systems_on_over_double(STAYMAN, "2♦"));
            // Opener passed to deny a stopper; responder re-asks, opener must
            // answer — `stayman_answers()` has no Pass rule, and its 2♦ is
            // exactly the artificial "no major" denial.
            entries.extend(rows_of(
                Pattern::after(STAYMAN, "(X) P (P)"),
                stayman_redouble_reask(),
            ));
            entries.extend(rows_of(
                Pattern::after(STAYMAN, "(X) P (P) XX (P)"),
                stayman_answers(),
            ));

            // A.1c — opener's 2-level answer (2♦/2♥/2♠) doubled.  The double
            // steals no room (responder's escapes all sit above 2♦), so
            // responder is systems-on: this is the escape the invitational-5-4
            // reroute needs — a 5♠4♥ that Staymaned bids its 2♠ instead of
            // sitting for a doubled 2♦ — and it also lets a 4-4 hand run to
            // 2NT rather than passing the double out.
            entries.push(rebase(
                Pattern::guarded(
                    STAYMAN,
                    "(P) 2♦ (X)",
                    described_guard(
                        "- 2♦/2♥/2♠ X …",
                        guard(|_: &Context<'_>, s: &[Call]| {
                            s.first() == Some(&Call::Pass)
                                && matches!(
                                    s.get(1),
                                    Some(Call::Bid(b))
                                        if b.level.get() == 2
                                            && matches!(
                                                b.strain,
                                                Strain::Diamonds | Strain::Hearts | Strain::Spades
                                            )
                                )
                                && s.get(2) == Some(&Call::Double)
                        }),
                    ),
                ),
                described_rewrite(
                    "systems on: their X is stripped to a pass",
                    rewriter(|auction: &[Call], depth: usize| {
                        if auction.get(depth + 2) != Some(&Call::Double) {
                            return None;
                        }
                        let mut rewritten = auction.to_vec();
                        rewritten[depth + 2] = Call::Pass; // strip the X → systems on
                        Some(rewritten)
                    }),
                ),
            ));

            // A.2 — our Stayman overcalled at the 2-level.  Opener's natural reply.
            for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                entries.extend(rows_of(
                    Pattern::after(STAYMAN, &format!("(2{})", Strain::from(over))),
                    stayman_overcalled_opener(over),
                ));
            }
            entries
        },
    }
}

/// Competition over our own Jacoby transfers as a row package
/// ([`set_competition_over_transfer`], default off)
///
/// Opener's replies after they double `1NT-(P)-2♦/2♥-(X)` or overcall it.
/// Keyed at the `[1NT, P, 2♦]` / `[1NT, P, 2♥]` nodes — distinct trie paths
/// from the Transfer-Lebensohl `[1NT, (2♦/2♥)]` block, where theirs sits at
/// depth 1.
pub(super) fn competition_over_transfer_package() -> Package {
    Package {
        name: "competition-over-transfer",
        gate: competition_over_transfer,
        entries: || {
            let mut entries = Vec::new();
            for (resp, major) in [(Suit::Diamonds, Suit::Hearts), (Suit::Hearts, Suit::Spades)] {
                let key = format!("P* 1NT (P) 2{}", Strain::from(resp));
                let completion = format!("2{}", Strain::from(major));

                // Our transfer doubled: opener's reply, then the systems-on
                // rebase off his completion or super-accept.
                entries.extend(rows_of(
                    Pattern::after(&key, "(X)"),
                    transfer_doubled_opener(major, resp),
                ));
                entries.push(systems_on_over_double(&key, &completion));
                // Opener passed to decline; responder re-asks, and opener's
                // forced completion has no Pass rule so he cannot sit.
                entries.extend(rows_of(
                    Pattern::after(&key, "(X) P (P)"),
                    transfer_pass_reask(major),
                ));
                entries.extend(rows_of(
                    Pattern::after(&key, "(X) P (P) XX (P)"),
                    complete_transfer(major),
                ));

                // Our transfer overcalled.  Opener's natural reply.
                let overcalls: &[(Suit, u8)] = match resp {
                    Suit::Diamonds => &[(Suit::Spades, 2), (Suit::Clubs, 3), (Suit::Diamonds, 3)],
                    _ => &[(Suit::Clubs, 3), (Suit::Diamonds, 3)],
                };
                for &(over_suit, over_level) in overcalls {
                    entries.extend(rows_of(
                        Pattern::after(&key, &format!("({over_level}{})", Strain::from(over_suit))),
                        transfer_overcalled_opener(major, over_suit, over_level),
                    ));
                }
            }
            entries
        },
    }
}

/// Competition over our own two-way `2♠` minor response as a row package
/// ([`set_competition_over_minor_transfer`], default on)
///
/// Opener's replies after they double `1NT-(P)-2♠-(X)` or overcall it.  Only
/// the PUPPET `2♠` (clubs *or* the balanced size-ask) has a min/max answer to
/// protect, so the package no-ops under the EUROPEAN pure-transfer scheme.
pub(super) fn competition_over_minor_transfer_package() -> Package {
    Package {
        name: "competition-over-minor-transfer",
        gate: || competition_over_minor_transfer() && notrump_minors() == PUPPET,
        entries: || {
            const TWO_SPADE: &str = "P* 1NT (P) 2♠";
            // A.1 — our 2♠ doubled.  Opener's coded min/max + stopper reply,
            // then the systems-on rebase off his 2NT/3♣ stopper-bid (the
            // `two_spade_over_min`/`max` machinery).
            let mut entries = rows_of(Pattern::after(TWO_SPADE, "(X)"), minor_doubled_opener());
            entries.push(systems_on_over_double(TWO_SPADE, "2NT"));
            // Opener denied a stopper (Pass = min, XX = max); responder signs
            // off in clubs.
            for deny in ["(X) P (P)", "(X) XX (P)"] {
                entries.extend(rows_of(
                    Pattern::after(TWO_SPADE, deny),
                    minor_no_stopper_rebid(),
                ));
            }

            // A.2 — our 2♠ overcalled.  `2NT`/`3♣` steal the size-ask steps, so
            // opener keeps the min/max + stopper signal; a higher overcall
            // (`3♦/3♥/3♠`) is systems-off.
            for (over, rules) in [
                ("(2NT)", minor_overcalled_high()),
                ("(3♣)", minor_overcalled_high()),
                ("(3♦)", minor_overcalled_low(Suit::Diamonds)),
                ("(3♥)", minor_overcalled_low(Suit::Hearts)),
                ("(3♠)", minor_overcalled_low(Suit::Spades)),
            ] {
                entries.extend(rows_of(Pattern::after(TWO_SPADE, over), rules));
            }
            entries
        },
    }
}

/// Competition over our own `2NT` diamond transfer as a row package
/// ([`set_competition_over_diamond_transfer`], default on)
///
/// Opener's replies after they double `1NT-(P)-2NT-(X)` or overcall it.  Only
/// the PUPPET scheme plays `2NT` as the diamond transfer, so the package
/// no-ops under EUROPEAN.
pub(super) fn competition_over_diamond_transfer_package() -> Package {
    Package {
        name: "competition-over-diamond-transfer",
        gate: || competition_over_diamond_transfer() && notrump_minors() == PUPPET,
        entries: || {
            const TWO_NT: &str = "P* 1NT (P) 2NT";
            // Our 2NT doubled: opener's 3♦-fit / 3♣-clubs / XX-values / Pass
            // reply, then the systems-on rebase off his fit-showing bid.
            let mut entries = rows_of(Pattern::after(TWO_NT, "(X)"), diamond_doubled_opener());
            entries.push(systems_on_over_double(TWO_NT, "3♦"));
            // Opener denied a fit (Pass = min, XX = max values); responder
            // signs off in 3♦ (always 5+♦).
            for deny in ["(X) P (P)", "(X) XX (P)"] {
                entries.extend(rows_of(
                    Pattern::after(TWO_NT, deny),
                    diamond_no_fit_rebid(),
                ));
            }

            // Our 2NT overcalled.  `3♣` leaves the `3♦` completion legal; a
            // higher overcall (`3♦` cue / `3♥` / `3♠`) keeps `3NT`/`X`/Pass
            // natural.
            for (over, rules) in [
                ("(3♣)", diamond_overcalled_low()),
                ("(3♦)", diamond_overcalled_high(Suit::Diamonds)),
                ("(3♥)", diamond_overcalled_high(Suit::Hearts)),
                ("(3♠)", diamond_overcalled_high(Suit::Spades)),
            ] {
                entries.extend(rows_of(Pattern::after(TWO_NT, over), rules));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
