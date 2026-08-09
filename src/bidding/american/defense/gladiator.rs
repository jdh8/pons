//! The Gladiator structure after our `1NT` overcall
//!
//! Opt-in ([`set_nt_overcall_gladiator`]): a relay structure replacing plain
//! systems-on ([`set_nt_overcall_systems_on`]) when we overcall `1NT` over
//! their major, so the strong hand declares and advancer can invite, force, or
//! escape a double.

use super::advance_sohl::sohl_rows_over;
use super::*;

thread_local! {
    /// Whether the advancer runs **systems-on** after our natural 1NT overcall:
    /// the whole opening-1NT response structure (Stayman, transfers, Smolen)
    /// grafted below `(1t) 1NT`, so a 15–18 balanced overcall finds 4-4 major
    /// fits and right-sides via transfers; **true by default** (measured a
    /// clean single-dummy-lead win over both minor and major openings). See
    /// [`set_nt_overcall_systems_on`].
    static NT_OVERCALL_SYSTEMS_ON: Cell<bool> = const { Cell::new(true) };
    /// Whether the advancer runs **Gladiator** (not systems-on) after our 1NT
    /// overcall of their **major**: a `2♣` weak relay, a cue-of-their-major
    /// Stayman for the *one* unbid major, natural INV bids, and shape actions
    /// (splinter, Leaping Michaels).  Replaces the opening-1NT graft for major
    /// openings only (minors keep systems-on); **false by default** (an A/B
    /// candidate — the major graft washes plain/PD, wins only on sd-lead). See
    /// [`set_nt_overcall_gladiator`].
    static NT_OVERCALL_GLADIATOR: Cell<bool> = const { Cell::new(false) };
}

/// Run systems-on (cue-Stayman) advances after our natural 1NT overcall, for
/// books built *after* this call (thread-local, read at construction)
///
/// `true` (the **default**) grafts the full opening-1NT response structure below
/// `(1t) 1NT`, so `(1♦) 1NT` equals `(1♣) 1NT` equals an opening 1NT — Stayman,
/// Jacoby/minor transfers, and Smolen, identical over both minors, with the same
/// structure over a major (one Stayman-found major is theirs). Transfers preserve
/// right-siding (the strong overcaller declares). `false` leaves the `(1t) 1NT -`
/// advance to the instinct floor's naturals. Off flag: `bba-gen
/// --no-ns-nt-overcall-systems-on`.
pub fn set_nt_overcall_systems_on(on: bool) {
    NT_OVERCALL_SYSTEMS_ON.with(|cell| cell.set(on));
}

/// Whether systems-on advances of the 1NT overcall are authored
pub(crate) fn nt_overcall_systems_on() -> bool {
    NT_OVERCALL_SYSTEMS_ON.with(Cell::get)
}

/// Run **Gladiator** advances after our 1NT overcall of their **major**, for
/// books built *after* this call (thread-local, read at construction)
///
/// `false` (the **default**) keeps the systems-on opening-1NT graft over majors.
/// `true` replaces that graft (for major openings only — minors stay systems-on)
/// with Gladiator: `2♣` = weak relay (pass-or-correct to the best part-score),
/// the cue of their major = Stayman for the single unbid major, natural `2♦`/`2M`
/// = 5-card INV, `2NT` = NF INV clubs, plus splinter / Leaping-Michaels shape
/// actions. Independent of [`set_nt_overcall_systems_on`] (it only governs
/// the *major* branch — minors keep systems-on when that is set). Off flag:
/// `bba-gen --ns-nt-overcall-gladiator`.
pub fn set_nt_overcall_gladiator(on: bool) {
    NT_OVERCALL_GLADIATOR.with(|cell| cell.set(on));
}

/// Whether Gladiator advances replace the major-opening systems-on graft
pub fn nt_overcall_gladiator() -> bool {
    NT_OVERCALL_GLADIATOR.with(Cell::get)
}

/// Gladiator: the advances of our 1NT overcall of their major
/// ([`set_nt_overcall_gladiator`])
///
/// Over a MAJOR one Stayman-found major is theirs, so the systems-on graft of
/// the whole opening-1NT structure does not fit the geometry; Gladiator replaces
/// it with a weak `2♣` relay, a cue-Stayman for the one unbid major `O`, and
/// shape actions.  Authored in every seat the opening could have been made
/// (mirrors the overcall's fan).  Two entries are not rules: their `(2♣)` is
/// rebased away (it steals no room, so systems stay on and only the relay is
/// consumed, reappearing as `X`), and the advance behind it is a transplant that
/// moves the relay's logit onto `Double`.  Their 2-level suit action instead
/// goes to [`gladiator_sohl_package`].
pub(super) fn gladiator_package() -> Package {
    Package {
        name: "gladiator",
        gate: |agreements| agreements.decision.reading.nt_overcall_gladiator(),
        entries: |_| {
            let mut entries = Vec::new();
            for suit in [Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                let base = format!("P* ({opening}) 1NT -");
                let os = Strain::from(other_major(suit));
                let cue = call(2, theirs);
                let cheap = if os > theirs { 2 } else { 3 };
                entries.extend(rows_of(Pattern::node(&base), gladiator_advances(suit)));

                // Advancer places the contract from what the cue answer showed —
                // the same ladder after the direct and the delayed cue.  Over `1♠`
                // the jump is `4♥` and the `3NT` misfit is already game, so those
                // advancer bids are left to the floor to pass.
                let cue_placements = |prefix: &str| {
                    let mut rows = rows_of(
                        Pattern::node(&format!("{prefix} {} -", call(cheap, os))),
                        gladiator_cue_min_fit(suit),
                    );
                    rows.extend(rows_of(
                        Pattern::node(&format!("{prefix} 2NT -")),
                        gladiator_cue_min_misfit(),
                    ));
                    if cheap + 1 < 4 {
                        rows.extend(rows_of(
                            Pattern::node(&format!("{prefix} {} -", call(cheap + 1, os))),
                            gladiator_cue_max_fit_raise(suit),
                        ));
                    }
                    rows
                };

                // Cue (Stayman for the one unbid major): overcaller answers, then
                // advancer places.
                let after_cue = format!("{base} {cue} -");
                entries.extend(rows_of(
                    Pattern::node(&after_cue),
                    gladiator_cue_answer(suit),
                ));
                entries.extend(cue_placements(&after_cue));

                // Natural invitational 2♦/2O and the 2NT weak club transfer —
                // overcaller accepts, or completes 3♣ for advancer to pass.
                for (advance, answer) in [
                    (call(2, Strain::Diamonds), gladiator_inv_diamond_answer()),
                    (call(2, os), gladiator_inv_major_answer(suit)),
                    (call(2, Strain::Notrump), gladiator_club_transfer_rebid()),
                    // Game-forcing naturals 3♣/3♦/3O and the 3M splinter —
                    // overcaller drives to game.
                    (call(3, Strain::Clubs), gladiator_gf_minor_answer()),
                    (call(3, Strain::Diamonds), gladiator_gf_minor_answer()),
                    (call(3, os), gladiator_gf_major_answer(suit)),
                    (call(3, theirs), gladiator_gf_major_answer(suit)),
                    // Leaping Michaels — overcaller places the 5-5 game force.
                    (
                        call(4, Strain::Clubs),
                        gladiator_leaping_answer(suit, Some(Suit::Clubs)),
                    ),
                    (
                        call(4, Strain::Diamonds),
                        gladiator_leaping_answer(suit, Some(Suit::Diamonds)),
                    ),
                    (call(4, theirs), gladiator_leaping_answer(suit, None)),
                ] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} {advance} -")),
                        answer,
                    ));
                }

                // 2♣ relay → forced 2♦ → advancer's XYZ-style sort; overcaller
                // then accepts or declines each invitational rebid.
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 2♣ -")),
                    gladiator_relay_rebid(),
                ));
                let sorted = format!("{base} 2♣ - 2♦ -");
                entries.extend(rows_of(
                    Pattern::node(&sorted),
                    gladiator_relay_continuation(suit),
                ));
                for inv in ["2NT", "3♣", "3♦"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{sorted} {inv} -")),
                        gladiator_relay_inv_answer(),
                    ));
                }
                entries.extend(rows_of(
                    Pattern::node(&format!("{sorted} {} -", call(3, os))),
                    gladiator_relay_major_answer(suit),
                ));
                // The weak `2O` takeout is a signoff, not a free bid — overcaller
                // passes it unless a max with four trumps pushes once.
                entries.extend(rows_of(
                    Pattern::node(&format!("{sorted} {} -", call(2, os))),
                    gladiator_relay_signoff_answer(suit),
                ));
                // Delayed cue (relay → forced 2♦ → cue of their major = exactly 3
                // `O`, INV+, not flat): overcaller shows min/max × 5-`O`-fit/misfit,
                // then advancer places with the same logic as after the direct cue.
                let delayed = format!("{sorted} {cue} -");
                entries.extend(rows_of(
                    Pattern::node(&delayed),
                    gladiator_delayed_cue_answer(suit),
                ));
                entries.extend(cue_placements(&delayed));

                // --- RHO acts over our 1NT before advancer can bid Gladiator ---

                // (X): a doubled 1NT always wants a runout, and Gladiator cannot
                // borrow the graft's — turning off `systems_on_overcall_strip`
                // leaves the floor reading an auction it was never distilled on.
                // Author it (see `gladiator_doubled_runout`).
                entries.extend(rows_of(
                    Pattern::node(&format!("P* ({opening}) 1NT (X)")),
                    gladiator_doubled_runout(suit),
                ));

                // (2♣): systems on, but it is Gladiator.  2♣ steals no room — every
                // other advance still sits above it — so only the 2♣ relay is
                // consumed, reappearing as X.  Rebase (their 2♣ → pass, our X → the
                // relay) routes every continuation onto the uncontested Gladiator
                // tree above; the transplant hands X a finite logit to be chosen.
                let relay_call = call(2, Strain::Clubs);
                entries.push(rebase(
                    Pattern::first(&format!("P* ({opening}) 1NT"), "2♣"),
                    described_rewrite(
                        "systems on: their 2♣ is treated as a pass; X asks as the stolen Gladiator relay",
                        rewriter(move |auction: &[Call], depth: usize| {
                            if auction.get(depth) != Some(&relay_call) {
                                return None;
                            }
                            let mut rewritten = auction.to_vec();
                            rewritten[depth] = Call::Pass; // (2♣) steals no room → systems on
                            if auction.get(depth + 1) == Some(&Call::Double) {
                                rewritten[depth + 1] = relay_call; // stolen relay = Double
                            }
                            Some(rewritten)
                        }),
                    ),
                ));
                // The rebase routes continuations; hand advancer a finite logit on
                // Double so it can *choose* the stolen relay (2♣ is illegal here).
                let advances = gladiator_advances(suit);
                entries.push(classified(
                    Pattern::table(&format!("P* ({opening}) 1NT (2♣)")),
                    classifier(move |hand: Hand, context: &Context<'_>| {
                        let mut logits = advances.classify(hand, context);
                        let relay = *logits.0.get(relay_call);
                        *logits.0.get_mut(relay_call) = f32::NEG_INFINITY; // 2♣ is stolen
                        *logits.0.get_mut(Call::Double) = relay; // X inherits the relay
                        logits
                    }),
                ));
            }
            entries
        },
    }
}

/// Gladiator's contested advance: their 2-level action over our 1NT overcall
///
/// No room for the `2♣` relay tree, so the partnership plays its Transfer
/// Lebensohl as if partner had opened 1NT.  `gate_4333 = true`: the overcaller is
/// balanced like a 1NT opener.  Reading is free via the builders' alerts; RHO's
/// 3-level+ interference falls to the floor.
pub(super) fn gladiator_sohl_package() -> Package {
    Package {
        name: "gladiator-sohl",
        gate: |agreements| agreements.decision.reading.nt_overcall_gladiator(),
        entries: |agreements| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(1, Strain::from(major));
                for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    let overcall = Bid::new(2, Strain::from(over));
                    entries.extend(sohl_rows_over(
                        &format!("P* ({opening}) 1NT ({overcall})"),
                        over,
                        LebensohlStyle::Transfer,
                        true,
                        agreements,
                    ));
                }
            }
            entries
        },
    }
}

/// Advancer's **Gladiator** actions after `(1M) 1NT -` (our 15–18 1NT overcall
/// of their major `M`); `O` is the one unbid major
///
/// `2♣` = weak relay (any suit) → forced `2♦`, pass-or-correct; cue of `M` =
/// Stayman for `O` (exactly 4, INV+); `2♦`/`2O` = natural 5-card INV; `2NT` = NF
/// INV clubs; `3♣`/`3♦`/`3O` = GF 5+; `3M` = splinter (0–1 M, 4 O, GF); `4O` = to
/// play; `4♣`/`4♦`/`4M` = Leaping Michaels (5-5 GF two-suiters).  Points are
/// advancer values opposite a strong NT: INV ≈ 8–9, GF ≈ 10+.
fn gladiator_advances(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let m = Strain::from(their_major);
    let os = Strain::from(o);
    let inv = 8u8;
    let game = 10u8;

    Rules::new()
        // Leaping Michaels: 5-5 game-forcing two-suiters (O + a minor, or both
        // minors via the jump in their suit).
        .rule(
            Bid::new(4, Strain::Clubs),
            150,
            len(o, 5..) & len(Suit::Clubs, 5..) & points(game..),
        )
        .alert(LEAPING)
        .rule(
            Bid::new(4, Strain::Diamonds),
            150,
            len(o, 5..) & len(Suit::Diamonds, 5..) & points(game..),
        )
        .alert(LEAPING)
        .rule(
            Bid::new(4, m),
            150,
            len(Suit::Diamonds, 5..) & len(Suit::Clubs, 5..) & points(game..),
        )
        .alert(LEAPING)
        // Splinter: game-forcing raise of O with a singleton/void in their major.
        .rule(
            Bid::new(3, m),
            145,
            len(o, 4..) & len(their_major, ..=1) & points(game..),
        )
        .alert(GLADIATOR_SPLINTER)
        // To-play game with a long other major (6-card O invites route through
        // the relay, so this is a game-values jump).
        .rule(Bid::new(4, os), 135, len(o, 6..) & points(game..))
        // Cue = Stayman for the unbid major: exactly 4, invitational-or-better.
        // A flat (4333) is barred (the 4333 curse): with no doubleton it has no
        // ruffing value, so a 4-4 major fit does not beat 3NT — it invites in NT.
        .rule(
            Bid::new(2, m),
            140,
            len(o, 4..=4) & points(inv..) & !flat_4333(),
        )
        .alert(GLADIATOR_STAYMAN)
        // Game-forcing naturals: 3 of a real 5+ suit.
        .rule(
            Bid::new(3, Strain::Clubs),
            130,
            len(Suit::Clubs, 5..) & points(game..),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            130,
            len(Suit::Diamonds, 5..) & points(game..),
        )
        .rule(Bid::new(3, os), 130, len(o, 5..) & points(game..))
        // Balanced game, to play.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            balanced() & points(game..),
        )
        // 2NT = weak transfer to clubs (6+♣): a weak long-club hand signs off in
        // 3♣ (invitational clubs go through the relay to 3♣ instead).
        .rule(
            Bid::new(2, Strain::Notrump),
            105,
            len(Suit::Clubs, 6..) & points(..inv),
        )
        .alert(GLADIATOR_CLUB_TRANSFER)
        // Natural invitational, exactly 5 (6-card invites route through the relay).
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Suit::Diamonds, 5..=5) & points(inv..game),
        )
        .rule(Bid::new(2, os), 100, len(o, 5..=5) & points(inv..game))
        // 2♣ = Gladiator relay (XYZ-style): a weak ♦/O takeout, any invitational
        // hand not shown directly, or a game-forcing non-flat hand with exactly 3
        // `O` that wants to check the 5-3 fit via the delayed cue — the forced 2♦
        // then sorts them.  A flat/short weak hand passes 1NT (the Pass catch-all).
        .rule(
            Bid::new(2, Strain::Clubs),
            50,
            (points(..inv) & (len(Suit::Diamonds, 5..) | len(o, 5..)))
                | points(inv..game)
                | (points(game..) & balanced() & len(o, 3..=3) & !flat_4333()),
        )
        .alert(GLADIATOR_RELAY)
        .rule(Call::Pass, 30, hcp(0..))
}

/// Overcaller's reply to the Gladiator cue (advancer showed exactly 4 `O`, INV+)
///
/// User-locked schema: cheapest `O` = MIN fit (15–16), jump `O` = MAX fit
/// (17–18); `2NT` = MIN misfit, `3NT` = MAX misfit.  Jumping to game opposite a
/// maximum fit is safe — the cue is INV+, so advancer is never broke.
fn gladiator_cue_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let m = Strain::from(their_major);
    let os = Strain::from(o);
    let cheap = if os > m { 2 } else { 3 };

    Rules::new()
        .rule(Bid::new(cheap, os), 140, len(o, 4..) & hcp(15..=16))
        .rule(Bid::new(cheap + 1, os), 140, len(o, 4..) & hcp(17..=18))
        .rule(
            Bid::new(2, Strain::Notrump),
            130,
            len(o, ..=3) & hcp(15..=16),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            len(o, ..=3) & hcp(17..=18),
        )
        // Finite catch-all (the overcall is a known 15–18, so the four above
        // already partition it).
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Overcaller's reply to the Gladiator **delayed** cue (advancer showed exactly 3
/// `O`, INV+, not flat — checking the 5-3 fit)
///
/// Same min/max × fit/misfit schema as [`gladiator_cue_answer`], but "fit" now
/// means a 5-card `O` (opposite advancer's exactly 3) rather than 4: cheapest `O`
/// = MIN fit (15–16 + 5 `O`), jump `O` = MAX fit (17–18 + 5 `O`); `2NT` = MIN
/// misfit, `3NT` = MAX misfit.  Advancer then places via the same
/// [`gladiator_cue_min_fit`] / [`gladiator_cue_min_misfit`] logic (GF→game,
/// INV→pass).
fn gladiator_delayed_cue_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    let m = Strain::from(their_major);
    let cheap = if os > m { 2 } else { 3 };

    Rules::new()
        .rule(Bid::new(cheap, os), 140, len(o, 5..) & hcp(15..=16))
        .rule(Bid::new(cheap + 1, os), 140, len(o, 5..) & hcp(17..=18))
        .rule(
            Bid::new(2, Strain::Notrump),
            130,
            len(o, ..=4) & hcp(15..=16),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            len(o, ..=4) & hcp(17..=18),
        )
        // Finite catch-all (the overcall is a known 15–18).
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Overcaller's forced completion of the Gladiator `2♣` relay
///
/// ponytail: pure `2♦` puppet; the max-break rebids (`2♥`/`2♠` showing a
/// maximum) are deferred — rare, and the advancer's own invitational
/// continuations carry the strength.
fn gladiator_relay_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, hcp(0..))
        .alert(GLADIATOR_RELAY_PC)
}

/// Advancer's continuation over the forced `2♦` (the XYZ-style sort)
///
/// Weak hands sign off (pass `2♦`, or `2O` with 5+ `O`); invitational hands show
/// a 6-card suit at the 3-level (`3♣`/`3♦`/`3O`) or bid `2NT` (balanced).  The
/// **delayed cue** (cue of their major) is exactly 3 `O`, INV+, not flat (4333) —
/// the 5-3-fit check that pairs with a 5-card-major overcall (see
/// [`GLADIATOR_DELAYED_CUE`]); a flat 4333 invites in notrump instead.
fn gladiator_relay_continuation(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    let m = Strain::from(their_major);
    let inv = 8u8;
    let game = 10u8;

    Rules::new()
        // Delayed cue = exactly 3 `O`, INV+, not flat (4333): checks the 5-3 major
        // fit that a 5-card-major 1NT overcall can hold (the direct cue promises 4;
        // a flat 4333 has no ruffing value and invites in notrump).
        .rule(
            Bid::new(2, m),
            100,
            len(o, 3..=3) & points(inv..) & !flat_4333(),
        )
        .alert(GLADIATOR_DELAYED_CUE)
        // Invitational, a 6-card suit.
        .rule(
            Bid::new(3, Strain::Clubs),
            95,
            len(Suit::Clubs, 6..) & points(inv..game),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            95,
            len(Suit::Diamonds, 6..) & points(inv..game),
        )
        .rule(Bid::new(3, os), 95, len(o, 6..) & points(inv..game))
        // Weak takeout: 5+ `O` to `2O`.
        .rule(Bid::new(2, os), 90, len(o, 5..) & points(..inv))
        // Invitational, balanced (no 6-card suit).
        .rule(Bid::new(2, Strain::Notrump), 85, points(inv..game))
        // Weak, diamond tolerance (or nothing better) — pass the puppet.
        .rule(Call::Pass, 50, hcp(0..))
}

/// Overcaller's reply to a natural invitational `2♦` (advancer 5+♦, INV ≈ 8–9)
///
/// A maximum accepts to `3NT` (diamonds a running source); a minimum passes the
/// diamond partscore.
fn gladiator_inv_diamond_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 130, hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to a natural invitational `2O` (advancer 5+ `O`, INV)
///
/// A three-card fit plus a maximum bids the `O` game; a maximum without a fit
/// tries `3NT`; a minimum passes the partscore.
fn gladiator_inv_major_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(4, os), 140, len(o, 3..) & hcp(17..))
        .rule(Bid::new(3, Strain::Notrump), 120, len(o, ..3) & hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller completes the `2NT` weak club transfer — forced `3♣`
fn gladiator_club_transfer_rebid() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Clubs), 100, hcp(0..))
}

/// Overcaller's reply to an invitational relay rebid (`2NT` balanced, or a
/// 6-card `3♣`/`3♦`): max accepts `3NT`, min passes the partscore.
fn gladiator_relay_inv_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 130, hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to an invitational 6-card-`O` relay rebid (`3O`): a fit
/// plus a max bids `4O`; a max without a fit tries `3NT`; a min passes.
fn gladiator_relay_major_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(4, os), 140, len(o, 3..) & hcp(17..))
        .rule(Bid::new(3, Strain::Notrump), 120, len(o, ..3) & hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to a game-forcing `3O` or the `3M` splinter (advancer 4+
/// `O`, GF)
///
/// A three-card fit bids the `O` game; otherwise `3NT`.  The splinter shares this
/// — same raise, plus shortness in their major.
fn gladiator_gf_major_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new().rule(Bid::new(4, os), 140, len(o, 3..)).rule(
        Bid::new(3, Strain::Notrump),
        120,
        hcp(0..),
    )
}

/// Overcaller's reply to a game-forcing minor `3♣`/`3♦` — game-forced to `3NT`
fn gladiator_gf_minor_answer() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Notrump), 120, hcp(0..))
}

/// Overcaller's reply to the weak `2O` signoff off the relay
/// (`2♣ - 2♦ - 2O` — advancer 5+ `O`, under invitational)
///
/// Advancer took the relay to *run*, not to invite: it has denied invitational
/// values by not rebidding `2NT`/`3X`/the cue.  Pass, unless a maximum with real
/// support wants one more — `3O` on four trumps and 18, where nine trumps and
/// 22-plus points make the partscore push sound.  Unauthored, the floor read the
/// signoff as a free bid and raised on **three** trumps, or bid `3NT` opposite a
/// hand that had just denied 8 points.
fn gladiator_relay_signoff_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(3, os), 120, len(o, 4..) & hcp(18..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to Leaping Michaels (`4♣`/`4♦` = 5-5 `O` + that minor;
/// `4M` = 5-5 both minors — both game-forcing)
///
/// `shown` is the minor the jump named, [`None`] for the both-minors `4M`.  The
/// auction is already past `3NT`, so there is no notrump landing and the only
/// question is which known fit to take: three-card support for `O` plays the
/// major game, otherwise five of the minor (the longer one when the jump showed
/// both).  Unauthored, the floor answered `4♣` with **`5NT`**.
fn gladiator_leaping_answer(their_major: Suit, shown: Option<Suit>) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    match shown {
        Some(minor) => Rules::new().rule(Bid::new(4, os), 140, len(o, 3..)).rule(
            Bid::new(5, Strain::from(minor)),
            120,
            hcp(0..),
        ),
        None => Rules::new()
            .rule(
                Bid::new(5, Strain::Diamonds),
                120,
                at_least_as_long(Suit::Diamonds, Suit::Clubs),
            )
            .rule(
                Bid::new(5, Strain::Clubs),
                120,
                longer_suit(Suit::Clubs, Suit::Diamonds),
            )
            // Finite catch-all: the two above already partition, but a table
            // that can reject a hand falls through to the floor.
            .rule(Bid::new(5, Strain::Clubs), 50, hcp(0..)),
    }
}

/// Advancer places the contract after the cue-answer showed a MIN fit (cheapest
/// `O`, 15–16 + 4 `O`): game-forcing values raise to `4O`, invitational pass.
fn gladiator_cue_min_fit(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(4, os), 130, points(10..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Advancer after a MAX fit shown below game (jump `O` = `3O` over `1♥`): the max
/// fit forces game, so raise to `4O` with everything.
fn gladiator_cue_max_fit_raise(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new().rule(Bid::new(4, os), 130, hcp(0..))
}

/// Advancer after a MIN misfit (`2NT`, 15–16 + ≤3 `O`): GF → `3NT`, INV → pass
fn gladiator_cue_min_misfit() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 130, points(10..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Advancer's runout when RHO doubles our 1NT overcall (`(1M) 1NT (X)`)
///
/// A doubled 1NT always wants a runout.  The systems-on graft gets one for
/// free: [`systems_on_overcall_strip`][crate::bidding] deletes their opening, the
/// auction reads as an opening 1NT, and the deterministic floor's
/// `responder_one_nt_runout` rules fire on a well-formed picture.  Gladiator
/// turns that strip off — its advances differ, so the strip identity no longer
/// holds — and the distilled net, fed the unstripped auction, escaped to the
/// *three* level on a bust (`8732.932.J973.T4` bid `3♥` doubled).  A finite book
/// node shadows the floor, so author the house card here instead.
///
/// `XX` = values, play `1NT××`; otherwise run to a five-plus suit, the longer
/// the better.  **Never into their major** — our side bidding their suit reads
/// as a cue, and running into the suit they opened is the worst landing on the
/// board.  A bust with no other five-bagger sits.
fn gladiator_doubled_runout(their_major: Suit) -> Rules {
    // Matches the floor's `set_runout_xx_min` default: below it we run, at it
    // or above we sit for the redouble.
    let xx_min = 7;
    let mut rules = Rules::new().rule(Call::Redouble, 120, hcp(xx_min..));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == their_major {
            continue;
        }
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            5
        } else {
            0
        };
        rules = rules
            .rule(
                Bid::new(2, strain),
                100 + major_bonus,
                len(suit, 5..) & hcp(..xx_min),
            )
            .rule(
                Bid::new(2, strain),
                110 + major_bonus,
                len(suit, 6..) & hcp(..xx_min),
            );
    }
    rules.rule(Call::Pass, 30, hcp(0..))
}

#[cfg(test)]
mod tests;
