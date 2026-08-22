//! The hand-written convention readers still awaiting retirement
//!
//! Each of these predates envelope unions and encodes a convention's meaning a
//! second time, beside its authoring rules.  `docs/reader-retirement.md` holds
//! the ledger; every function here is a chop candidate, and the tail helpers
//! below serve the natural walk in [`super::read`].

use super::envelope::{Envelope, EnvelopeUnion, Range, Relative, relative_of};
use super::knobs::ReadingProfile;
use super::read::support_band_to_points;
use super::{LENGTH_CAP, POINTS_CAP};
use crate::bidding::agreements::TheirDisclosures;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The Rubens-artificial calls of an advance, and the advance's strength reading
///
/// In a [Rubens advance][crate::bidding::instinct::overcall_shape] of a simple overcall,
/// some calls *name* a suit they do not *hold*: the advancer's transfer (a relay
/// to the next suit up) or cue-raise, and the overcaller's forced completion.
/// Returns `(suppress, cue, transfer)` — `suppress` lists those indices, whose
/// bid suit must not be read as natural length; `cue` is `(index, Y)` of a
/// two-level cue-raise, read separately as a limit-plus raise (three-plus cards
/// in partner's overcall `Y`, ten-plus points); `transfer` is
/// `(index, suit, min-len)` of a one-level transfer's meaning — the transfer
/// into partner's suit is the same limit-plus raise (`(index, Y, 3)`), a
/// new-suit transfer shows its own five-card target (`(index, target, 5)`),
/// both ten-plus points
/// ([`rubens_transfer`][field@crate::bidding::ReadingProfile::rubens_transfer], recorded post-walk
/// for the advancer's *own side only* — an opponent's in-band advance may be
/// natural).
///
/// The shown values are what let the overcaller judge game — and the completion
/// is a forced relay, still never read as length (soundness over tightness, as
/// with transfers over our own notrump).
#[allow(clippy::type_complexity)]
pub(super) fn rubens_reading(
    auction: &[Call],
    profile: ReadingProfile,
) -> (
    [Option<usize>; 2],
    Option<(usize, Suit)>,
    Option<(usize, Suit, u8)>,
) {
    let none = ([None, None], None, None);
    // The bidder's knob governs the reading too: with Rubens advances off, an
    // advance in the band is a genuine suit and must be read naturally.
    if !profile.rubens_advances {
        return none;
    }
    let Some((x, y, overcall_index, level)) = crate::bidding::instinct::overcall_shape(auction)
    else {
        return none;
    };
    // The advance comes after the overcaller's partner (RHO of the overcaller)
    // passes; the advancer's call sits two past the overcall.
    if auction.get(overcall_index + 1) != Some(&Call::Pass) {
        return none;
    }
    let advance_index = overcall_index + 2;
    let Some(&Call::Bid(advance)) = auction.get(advance_index) else {
        return none;
    };
    if level == 2 {
        // Two-level overcall: the cue-raise (2X) is the lone artificial call.
        return if advance == Bid::new(2, Strain::from(x)) {
            ([Some(advance_index), None], Some((advance_index, y)), None)
        } else {
            none
        };
    }
    // One-level overcall: a transfer 2S (X ≤ S < Y), then the completion 2(S+1).
    let Some(source) = advance.strain.suit() else {
        return none;
    };
    if advance.level.get() != 2 || (source as u8) < (x as u8) || (source as u8) >= (y as u8) {
        return none;
    }
    let target_suit = Suit::ASC[(source as u8 + 1) as usize];
    let target = Strain::from(target_suit);
    // The overcaller completes through opener's lead-directing double too, so
    // the completion stays a relay (never a holding) in both shapes.
    let completion = (matches!(
        auction.get(advance_index + 1),
        Some(Call::Pass | Call::Double)
    ) && auction.get(advance_index + 2) == Some(&Call::Bid(Bid::new(2, target))))
    .then_some(advance_index + 2);
    // The transfer's meaning, fixed the moment it is made (the completion is
    // not required): into partner's suit = the limit-plus raise, a new suit =
    // the advancer's own five-card target.
    let transfer = profile.rubens_transfer.then_some(if target_suit == y {
        (advance_index, y, 3)
    } else {
        (advance_index, target_suit, 5)
    });
    ([Some(advance_index), completion], None, transfer)
}

/// The advancer's `2♦` relay / `2♥`-`2♠` preference over a Landy/Woolsey both-majors
/// `2♣`, whose natural single-suit reading is suppressed
///
/// The one suppression the projection pass cannot supply: a relay names no length of
/// its own, so its authored rule projects nothing and the artificial detector (which
/// drives the rest of the suppression now, M6.2c) misses it.  The `2♣` overcall
/// itself, and every other retired convention's shape, are read straight off their
/// projected rule; this is the lone hand stub the doc keeps.
///
/// `None` unless Landy or Woolsey is on *and* the defending side's first action over
/// their `1NT` was the both-majors `2♣`, so a natural `2♣` is never mistaken for it.
// ponytail: a relay projects no info, so suppress it by hand; the upgrade path is to
// author the relay's rule with the negated lengths so the detector catches it too.
pub(super) fn landy_advance_suppress(auction: &[Call], profile: ReadingProfile) -> Option<usize> {
    let on = profile.landy || profile.woolsey_enabled();
    if !on {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The both-majors 2♣ must be the defending side's first action.
    let overcall_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Bid(bid) if index % 2 != opener_parity => {
                Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
            }
            // The opener answered, or a defender did something else — not a 2♣ Landy.
            _ => Some(None),
        })
        .flatten()?;

    advancer_artificial(auction, overcall_index, opener_parity)
}

/// The opponents' disclosed Landy `2♣` over **our** `1NT`, and its advances
///
/// [`TheirDisclosures::two_clubs_landy`] is a fact about the reader's
/// opponents, so unlike every symmetric reader above it is seat-gated: it
/// fires only when the `1NT` opener is on the *reader's* side and the `2♣` is
/// the other side's first action.  The mirror image — our own `2♣` overcall
/// of their `1NT` — is our agreement, governed by our own knobs, and never
/// matches.
#[derive(Clone, Copy)]
pub(super) struct TheirLandyReading {
    /// Their both-majors `2♣`: its natural club reading is suppressed and
    /// 4-4+ in the majors is recorded post-walk (no strength claim — their
    /// band is undeclared).
    pub(super) overcall_index: usize,
    /// Their advancer's first `2♦`/`2♥`/`2♠` — a relay or a preference among
    /// partner's majors, playable on a doubleton, so its natural reading is
    /// suppressed.
    advance: Option<usize>,
    /// Their advancer's direct `3♥`/`3♠` — an invitational raise of a shown
    /// major, which the walk would otherwise read as a weak-jump six-carder.
    /// Suppressed; nothing recorded (their raise style is undeclared).
    jump_advance: Option<usize>,
}

/// Locate a disclosed call made by the opponents over our `1NT`
///
/// Returns the opening side's parity and the call's index.  The call must be
/// the defending side's first non-pass action; the seat gate prevents a fact
/// about *their* defense from decoding our mirror-image overcall.
fn their_disclosed_overcall(auction: &[Call], len: usize, wanted: Bid) -> Option<(usize, usize)> {
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;
    if opener_parity != len % 2 {
        return None;
    }
    let overcall_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Bid(bid) if index % 2 != opener_parity => Some((bid == wanted).then_some(index)),
            _ => Some(None),
        })
        .flatten()?;
    Some((opener_parity, overcall_index))
}

/// Read their disclosed Landy `2♣` over our `1NT`
/// ([`ReadingProfile::their_landy_reading`] is the wiring switch)
// ponytail: a hand reader, not a projection — their calls have no authored rule
// of ours to project; the upgrade path is the declared-opponent their_profile
// split, which would decode their 2♣ off their own declared book.
pub(super) fn their_landy_reading(
    auction: &[Call],
    len: usize,
    profile: ReadingProfile,
    their: TheirDisclosures,
) -> Option<TheirLandyReading> {
    if !(profile.their_landy_reading && their.two_clubs_landy) {
        return None;
    }
    let (opener_parity, overcall_index) =
        their_disclosed_overcall(auction, len, Bid::new(2, Strain::Clubs))?;

    // The advancer's first bid, wherever it sits: the scan jumps over every
    // call of ours (pass, double, or a competing bid), as
    // `advancer_artificial` does.
    let first_advance = auction
        .iter()
        .enumerate()
        .skip(overcall_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => Some((index, bid)),
            _ => None,
        });
    let (advance, jump_advance) = match first_advance {
        Some((index, bid)) if bid.level.get() == 2 && bid.strain != Strain::Notrump => {
            (Some(index), None)
        }
        Some((index, bid))
            if bid.level.get() == 3 && matches!(bid.strain, Strain::Hearts | Strain::Spades) =>
        {
            (None, Some(index))
        }
        _ => (None, None),
    };
    Some(TheirLandyReading {
        overcall_index,
        advance,
        jump_advance,
    })
}

/// The opponents' disclosed Multi `2♦` and pass-or-correct advance
#[derive(Clone, Copy)]
pub(super) struct TheirMultiReading {
    /// Their artificial `2♦`: one unknown six-card major.
    overcall_index: usize,
    /// Their advancer's first pass-or-correct, which names no holding of its
    /// own — the whole ladder, not just the two-level rung.
    advance: Option<usize>,
}

impl TheirMultiReading {
    fn suppresses(self, index: usize) -> bool {
        self.overcall_index == index || self.advance == Some(index)
    }
}

/// Read their disclosed Multi `2♦` over our `1NT`
///
/// ponytail: this hand reader exists only because BBA has no declared opponent
/// book to project; delete it when `their_profile` can decode that foreign book.
fn their_multi_reading(
    auction: &[Call],
    len: usize,
    profile: ReadingProfile,
    their: TheirDisclosures,
) -> Option<TheirMultiReading> {
    if !(profile.their_multi_reading && their.two_diamonds_multi) {
        return None;
    }
    let (opener_parity, overcall_index) =
        their_disclosed_overcall(auction, len, Bid::new(2, Strain::Diamonds))?;
    // Both halves ride the knob: widening the ladder is itself a reading
    // change, so leaving it ungated would move every anchor arm's base while
    // the A/B's own switch looked like it only added the positive claim.
    let advance = if profile.their_multi_advance_reading {
        multi_advance_ladder(auction, overcall_index, opener_parity)
    } else {
        advancer_artificial(auction, overcall_index, opener_parity)
    };
    Some(TheirMultiReading {
        overcall_index,
        advance,
    })
}

/// The advancer's first call from their Multi's **whole** pass-or-correct
/// ladder
///
/// [`advancer_artificial`] matches only `2♦`/`2♥`/`2♠` because it is shared
/// with the Landy reader, whose three-level advances *are* natural.  Over a
/// Multi they are not: every rung of `2♥ / 2♠ / 3♥ / 3♠ / 4♣ / 4♦ / 4♥ / 4♠`
/// is "bid your major", so the natural walk reading `3♥` as `♥ 6..13` or `4♦`
/// as `♦ 3..13` asserts a suit the advancer need not hold at all.
///
/// Measured, not assumed: `probe-bba-constraints --mode custom --seat 3
/// --calls "1NT 2♦ 2♠"` (6000 hands) puts the advancer's `3♥` at **♥ 2–5,
/// median 3** — so the natural walk's `♥ 6..13` is false on most of the band —
/// and its `4♦` at `♦` nothing, `♥ 3–5 / ♠ 3–6`.
///
/// **Suppression only, no positive claim.** A first build also published
/// `♥3+ & ♠3+` on the jump rungs, reasoning that an advancer choosing a
/// three- or four-level contract must be able to play either major.  The same
/// probe refutes it — `3♥` is `♠ 2–4`, and its 10th-percentile tail runs to a
/// singleton — and the A/B measured the cost: **negative in all eight cells**
/// over two seeds, with the worst boards showing our side talked out of a
/// correct `4♠` save by a spade length the advancer did not have (`♠1` on
/// `5.Q52.T543.KQJ82`, `♠2` on `A3.KQT8763.9.QT8`).  What is left is sound by
/// construction: it only ever *removes* a possibly-false length.
fn multi_advance_ladder(
    auction: &[Call],
    overcall_index: usize,
    opener_parity: usize,
) -> Option<usize> {
    const LADDER: [(u8, Strain); 9] = [
        (2, Strain::Diamonds),
        (2, Strain::Hearts),
        (2, Strain::Spades),
        (3, Strain::Hearts),
        (3, Strain::Spades),
        (4, Strain::Clubs),
        (4, Strain::Diamonds),
        (4, Strain::Hearts),
        (4, Strain::Spades),
    ];
    auction
        .iter()
        .enumerate()
        .skip(overcall_index + 1)
        // Stop at their advancer's first *bid*; jump over everything we do.
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => Some(
                LADDER
                    .iter()
                    .any(|&(level, strain)| bid == Bid::new(level, strain))
                    .then_some(index),
            ),
            _ => None,
        })
        .flatten()
}

impl TheirLandyReading {
    /// Whether the call at `index` names a suit its bidder need not hold
    fn suppresses(&self, index: usize) -> bool {
        self.overcall_index == index
            || self.advance == Some(index)
            || self.jump_advance == Some(index)
    }
}

/// The index of the advancer's first `2♦`/`2♥`/`2♠` response over a both-majors /
/// Multi overcall at `overcall_index` — a relay or a preference among partner's
/// suits, never own length, so its natural reading is suppressed
///
/// The scan jumps over *every* opponent call (pass, double, or a competing suit
/// bid), so a quiet advance and a doubled / contested runout are all covered: a
/// `2♦`/`2♥`/`2♠` is only legal as the *immediate* response (once the auction climbs
/// past `2♠` it can never recur), so the first such call we find is always the
/// preference, whatever the opponents did.  Suppression is sound regardless — it only
/// ever *removes* a possibly-false length, never asserts one.  The suppression then
/// lives for the whole `Inferences::read`.  `None` if our first response was instead
/// an ask (`2NT`) or a genuine raise.
fn advancer_artificial(
    auction: &[Call],
    overcall_index: usize,
    opener_parity: usize,
) -> Option<usize> {
    auction
        .iter()
        .enumerate()
        .skip(overcall_index + 1)
        // Stop at our first *bid* (decide there); jump over everything the opponents do.
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => Some(
                matches!(
                    bid,
                    b if b == Bid::new(2, Strain::Diamonds)
                        || b == Bid::new(2, Strain::Hearts)
                        || b == Bid::new(2, Strain::Spades)
                )
                .then_some(index),
            ),
            _ => None,
        })
        .flatten()
}

/// Our **Gladiator** advance of a 1NT overcall of their major
/// ([`nt_overcall_gladiator`][field@crate::bidding::inference::ReadingProfile::nt_overcall_gladiator])
///
/// The advancer's artificial calls under `(1M) 1NT - ?` — the `2♣` relay (and
/// its forced `2♦` completion), the cue of their major (Stayman for the unbid
/// major), the `3M` splinter, and the `4M` both-minor Leaping Michaels — are bids
/// of a suit the caller does *not* hold; the natural walk would floor a phantom
/// suit.  Their indices are suppressed and the real shape recorded post-walk.  The
/// natural advances (`2♦`/`2O`, the 3-level naturals, `4O`) read off the walk and
/// never enter here.
#[derive(Clone, Copy)]
pub(super) enum GladiatorAdvance {
    /// `2♣` relay (weak / invitational) — no sound per-suit floor.
    Relay,
    /// Cue of their major = Stayman: 4+ in the unbid major `o`, INV+.
    Cue { o: Suit },
    /// Delayed cue (`2♣` relay → forced `2♦` → cue of their major): exactly 3 in
    /// the unbid major `o`, INV+ — the 5-3-fit check.
    DelayedCue { o: Suit },
    /// `3M` splinter: 4+ `o`, 0–1 in their major `m`, GF.
    Splinter { o: Suit, m: Suit },
    /// `4M` Leaping Michaels: both minors 5+, GF.
    BothMinors,
    /// `4♣`/`4♦` Leaping Michaels: 5+ `o` + 5+ the named `minor`, GF.
    Minor { o: Suit, minor: Suit },
    /// `2NT`: a weak transfer to clubs (6+♣) — not a balanced notrump.
    ClubTransfer,
}

#[derive(Clone, Copy)]
pub(super) struct GladiatorReading {
    /// Index of the advancer's Gladiator call
    pub(super) index: usize,
    pub(super) advance: GladiatorAdvance,
    /// Bitset of indices whose natural suit reading the walk must skip
    pub(super) suppress: u64,
}

impl GladiatorReading {
    pub(super) const fn suppresses(self, index: usize) -> bool {
        index < 64 && self.suppress >> index & 1 != 0
    }
}

pub(super) fn gladiator_reading(
    auction: &[Call],
    profile: ReadingProfile,
) -> Option<GladiatorReading> {
    if !profile.nt_overcall_gladiator {
        return None;
    }
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[open] else {
        return None;
    };
    let m = opening.strain.suit()?;
    if opening.level.get() != 1 || !matches!(m, Suit::Hearts | Suit::Spades) {
        return None;
    }
    // Our 1NT overcall, then the advancer.  RHO usually passes; over RHO's (2♣)
    // systems-on overcall we mirror the book rebase — their 2♣ maps to a pass and
    // advancer's Double to the stolen 2♣ relay — and re-read, so every (2♣)
    // continuation (relay, delayed cue, cue-Stayman, club transfer) decodes
    // through the uncontested logic below with the same call indices.  Any other
    // RHO action leaves it to the natural walk.
    if auction.get(open + 1) != Some(&Call::Bid(Bid::new(1, Strain::Notrump))) {
        return None;
    }
    if auction.get(open + 2) == Some(&Call::Bid(Bid::new(2, Strain::Clubs))) {
        let mut stripped = auction.to_vec();
        stripped[open + 2] = Call::Pass;
        if auction.get(open + 3) == Some(&Call::Double) {
            stripped[open + 3] = Call::Bid(Bid::new(2, Strain::Clubs));
        }
        return gladiator_reading(&stripped, profile);
    }
    if auction.get(open + 2) != Some(&Call::Pass) {
        return None;
    }
    let index = open + 3;
    let Some(&Call::Bid(bid)) = auction.get(index) else {
        return None;
    };
    let o = if m == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };

    // `index ≤ 6` (at most three leading passes), so the shifts never overflow.
    let mut suppress = 0u64;
    let advance = if bid == Bid::new(2, Strain::Clubs) {
        suppress |= 1 << index;
        // The overcaller's forced 2♦ completion in `relay - 2♦` says nothing
        // of diamonds — suppress it too.
        let mut delayed = false;
        if auction.get(index + 2) == Some(&Call::Bid(Bid::new(2, Strain::Diamonds))) {
            suppress |= 1 << (index + 2);
            // The delayed cue at index+4 in
            // `relay - 2♦ - cue-of-their-major` is a phantom-suit call too
            // (advancer holds exactly 3 `o`, not `m`).
            if auction.get(index + 4) == Some(&Call::Bid(Bid::new(2, opening.strain))) {
                suppress |= 1 << (index + 4);
                delayed = true;
            }
        }
        if delayed {
            GladiatorAdvance::DelayedCue { o }
        } else {
            GladiatorAdvance::Relay
        }
    } else if bid == Bid::new(2, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::Cue { o }
    } else if bid == Bid::new(3, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::Splinter { o, m }
    } else if bid == Bid::new(4, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::BothMinors
    } else if bid == Bid::new(2, Strain::Notrump) {
        suppress |= 1 << index;
        // The overcaller's forced 3♣ transfer completion says nothing of clubs.
        if auction.get(index + 2) == Some(&Call::Bid(Bid::new(3, Strain::Clubs))) {
            suppress |= 1 << (index + 2);
        }
        GladiatorAdvance::ClubTransfer
    } else if bid == Bid::new(4, Strain::Clubs) {
        GladiatorAdvance::Minor {
            o,
            minor: Suit::Clubs,
        }
    } else if bid == Bid::new(4, Strain::Diamonds) {
        GladiatorAdvance::Minor {
            o,
            minor: Suit::Diamonds,
        }
    } else {
        return None;
    };

    Some(GladiatorReading {
        index,
        advance,
        suppress,
    })
}

/// The index of our natural **penalty** double of their 1NT (15+ HCP), or `None`
///
/// A double of 1NT names no suit, so the generic walk's takeout branch (which needs
/// a suit opening) reads it as nothing.  Returns the doubler's index so the post-walk
/// pass records the [`natural_double_floor`][field@crate::bidding::inference::ReadingProfile::natural_double_floor]
/// points floor.  Mirrors [`woolsey_x_reading`].
///
/// Fires only when a double of their 1NT actually *means* the natural penalty double:
/// the natural defense is on and no convention has repurposed the double (DONT = a
/// one-suiter, direct Landy / Woolsey = both majors — each has its own reading).  A
/// *passed* doubler cannot hold 15+, so their double is the both-majors passed-hand
/// call, not penalty; an unpassed doubler is identified by lane (a seat that passed
/// before the opening occupies a lane below `opening_index`).
pub(in crate::bidding) fn penalty_x_reading(auction: &[Call]) -> Option<usize> {
    penalty_x_reading_with_profile(auction, ReadingProfile::default())
}

fn penalty_x_reading_with_profile(auction: &[Call], profile: ReadingProfile) -> Option<usize> {
    // One `Cell<NotrumpDefense>` holds one system, so "Natural is active" is the
    // whole test: the four "…but not DONT/Meckwell/direct-Landy/Woolsey"
    // disjuncts this used to carry were the pre-fold precedence cascade, and
    // every one of them was unreachable once the enum landed.
    if !profile.natural_defense_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The double must be the defending side's FIRST action over their 1NT.
    let double_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some(index)),
            // The opener's side acted, or a defender overcalled — not the penalty double.
            _ => Some(None),
        })
        .flatten()?;

    // A passed doubler's double is the both-majors passed-hand call, never 15+ penalty.
    // Seats that passed before the opening fill lanes `0..opening_index` (all the calls
    // there are passes), so an unpassed doubler's lane is at or beyond `opening_index`.
    (double_index % 4 >= opening_index).then_some(double_index)
}

/// The index of responder's double of an opponent's overcall of *our* 1NT
/// (`1NT (2X) X`), or `None`
///
/// Every [`DoubleStyle`][crate::bidding::american::DoubleStyle] makes this double
/// show **8+ values** (takeout ≤3/8, penalty 4+/9, optional 2-3/8), so the post-walk
/// records that points floor — without it the double reads as nothing and opener
/// undercounts the partnership's strength.  Fires only for our own 1NT (the opener
/// shares the actor's parity); their responder's double of our overcall is their
/// convention, not ours.
///
/// **The Multi lane is the exception**, and it was a defect: over a declared
/// `(2♦)` Multi the double is not a `DoubleStyle` double at all but the N4
/// values call, authored `hcp(6..)` (`multi_2d_responder`), so the flat 8
/// asserted two points responder never promised.  With
/// [`ReadingProfile::their_multi_double_reading`] on, the floor follows the
/// lane's own rule.  Returns the index and the floor to publish.
pub(super) fn responder_overcall_double_reading(
    auction: &[Call],
    len: usize,
    profile: ReadingProfile,
    their: TheirDisclosures,
) -> Option<(usize, u8)> {
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump))
        || opening_index % 2 != len % 2
    {
        return None;
    }
    // The opponent's suit overcall, then our responder's immediate double of it.
    match auction.get(opening_index + 1) {
        Some(Call::Bid(bid)) if bid.strain.is_suit() => {}
        _ => return None,
    }
    if auction.get(opening_index + 2) != Some(&Call::Double) {
        return None;
    }
    let multi = profile.their_multi_double_reading
        && their.two_diamonds_multi
        && auction[opening_index + 1] == Call::Bid(Bid::new(2, Strain::Diamonds));
    Some((opening_index + 2, if multi { 6 } else { 8 }))
}

/// Our side's *subsequent* penalty doubles after the natural penalty X of their
/// 1NT — the latch's later doubles — each paired with the suit it doubles
///
/// The penalty latch ([`penalty_latch`][field@crate::bidding::inference::ReadingProfile::penalty_latch])
/// makes these via the trump-stack rule, so each promises four-plus cards in the
/// doubled suit.  Recording that length stops the sampler reading the double as
/// takeout — without it the advancer pulls a penalty double thinking partner is
/// short, the phantom-suit leak the [`penalty_x_reading`] doc names.  Empty unless
/// the latch is on, so it agrees with the floor on when a later double is penalty.
///
/// Once we penalty-double their 1NT the penalty stance holds for the rest of the
/// auction (mirrors `penalty_latched`) — a bid of our own does *not* un-latch it,
/// it only updates the suit a later penalty double refers to.
pub(super) fn penalty_latch_double_reading(
    auction: &[Call],
    profile: ReadingProfile,
) -> Vec<(usize, Suit)> {
    if !profile.penalty_latch {
        return Vec::new();
    }
    let Some(x_index) = penalty_x_reading_with_profile(auction, profile) else {
        return Vec::new();
    };
    let our_parity = x_index % 2;
    let mut out = Vec::new();
    let mut last_suit_bid: Option<(Suit, usize)> = None; // (suit, the bidder's parity)
    for (index, &call) in auction.iter().enumerate().skip(x_index + 1) {
        match call {
            // Our own bid does not un-latch the penalty stance; it just updates the
            // suit a later penalty double would refer to.
            Call::Bid(bid) => {
                last_suit_bid = bid.strain.suit().map(|suit| (suit, index % 2));
            }
            // Our double of their suit runout is penalty: four-plus in that suit.
            Call::Double if index % 2 == our_parity => {
                if let Some((suit, bidder_parity)) = last_suit_bid
                    && bidder_parity != our_parity
                {
                    out.push((index, suit));
                }
            }
            _ => {}
        }
    }
    out
}

/// Apply the meaning of the opening bid (the first non-pass call)
pub(super) fn apply_opening(inf: &mut Envelope, bid: Bid, seat: u8, profile: ReadingProfile) {
    // A one-level suit opening reads 10, not 12: `points(12..)` on the shipped
    // rule-of-N+8 scale is the Rule of 20, which admits sound 10-11 counts, and
    // the reading has to stay loose enough for a floor arm or an opponent whose
    // scale we do not control.  Third/fourth seat opens majors lighter still (9).
    let major_floor = if seat >= 3 { 9 } else { 10 };
    let minor_floor = 10;
    let majors_light = Range::new(major_floor, 21);
    match (bid.level.get(), bid.strain) {
        (1, Strain::Hearts) => {
            inf.narrow_length(Suit::Hearts, Range::at_least(5, LENGTH_CAP));
            inf.narrow_points(majors_light);
        }
        (1, Strain::Spades) => {
            inf.narrow_length(Suit::Spades, Range::at_least(5, LENGTH_CAP));
            inf.narrow_points(majors_light);
        }
        (1, Strain::Diamonds) => {
            inf.narrow_length(Suit::Diamonds, Range::at_least(3, LENGTH_CAP));
            inf.narrow_length(Suit::Hearts, Range::new(0, 4));
            inf.narrow_length(Suit::Spades, Range::new(0, 4));
            inf.narrow_points(Range::new(minor_floor, 21));
        }
        (1, Strain::Clubs) => {
            inf.narrow_length(Suit::Clubs, Range::at_least(3, LENGTH_CAP));
            inf.narrow_length(Suit::Hearts, Range::new(0, 4));
            inf.narrow_length(Suit::Spades, Range::new(0, 4));
            inf.narrow_points(Range::new(minor_floor, 21));
        }
        (1, Strain::Notrump) => {
            // Balanced, OR — since the shipped `Wide6322` shape also opens 1NT
            // on a 6322 with a six-card minor — a minor running to six.  Majors
            // stay 2–5 (a balanced 5332 major); minors widen to 2–6.  Set the
            // four suits directly: `narrow_length` only intersects, so clamping
            // via `balanced()` first would pin the minors back to five.
            inf.narrow_length(Suit::Spades, Range::new(2, 5));
            inf.narrow_length(Suit::Hearts, Range::new(2, 5));
            inf.narrow_length(Suit::Clubs, Range::new(2, 6));
            inf.narrow_length(Suit::Diamonds, Range::new(2, 6));
            // Plain HCP 15–17 gates the opening (fifths archived).  The plain
            // rule-of-N+8 opt-in scale reads a flat 4-3-3-3 one under its HCP
            // (the shipped floored scale doesn't) and a 5422/6322 one over
            // (9-card long suits − 8); the legacy upgrade scale adds at most
            // +1 the same way.  Sound band 15−slack..18 — the slack term
            // keeps every opt-in arm exact.  ponytail:
            // exact for the shipped plain-HCP gauge; the archived
            // `one_notrump_fifths` knob, if ever revived, would re-widen
            // this to 14–19.
            let slack = crate::bidding::constraint::flat_hcp_slack(profile.point_scale);
            inf.narrow_points(Range::new(15 - slack, 18));
            // The `hcp` gauge is crisp raw HCP — 15–17 gates the opening, with
            // no upgrade slack (notrump valuation, read behind Edit 2's knob).
            inf.narrow_hcp(Range::new(15, 17), profile.point_scale);
        }
        (2, Strain::Clubs) => {
            // Strong and artificial: 22+ points, but nothing about shape.
            inf.narrow_points(Range::at_least(20, POINTS_CAP));
        }
        (2, Strain::Notrump) => {
            if profile.two_notrump_wide {
                // Chop G0: the wide-minor 2NT (`ReadingProfile::two_notrump_wide`) caps
                // majors at four (5M(332) opens one-of-a-major) and runs minors
                // to six (5m422/6m322).  `narrow_length` only intersects, so set
                // the four suits directly rather than clamping via `balanced()`.
                inf.narrow_length(Suit::Spades, Range::new(2, 4));
                inf.narrow_length(Suit::Hearts, Range::new(2, 4));
                inf.narrow_length(Suit::Clubs, Range::new(2, 6));
                inf.narrow_length(Suit::Diamonds, Range::new(2, 6));
            } else {
                balanced(inf);
            }
            // As with 1NT: `fifths(20.0..22.0)` admits a quack-heavy 23-count
            // (fifths within 1.6 of raw HCP), so the sound point envelope is
            // 19–23, not 19–22 — and the plain rule-of-N+8 opt-in gives a
            // flat 4-3-3-3 floor another point back.
            let slack = crate::bidding::constraint::flat_hcp_slack(profile.point_scale);
            inf.narrow_points(Range::new(19 - slack, 23));
        }
        (2, strain) if strain.is_suit() => {
            inf.narrow_length(strain.suit().unwrap(), Range::new(6, 6));
            inf.narrow_points(Range::new(5, 10));
        }
        (3, strain) if strain.is_suit() => {
            inf.narrow_length(strain.suit().unwrap(), Range::at_least(7, LENGTH_CAP));
            inf.narrow_points(Range::new(0, 11));
        }
        _ => {}
    }
}

/// Narrow a balanced opener: two to five cards in every suit
fn balanced(inf: &mut Envelope) {
    for suit in Suit::ASC {
        inf.narrow_length(suit, Range::new(2, 5));
    }
}

/// The point floor a responder's first natural new suit shows, when uncontested
///
/// A one-level new suit promises six-plus points; a game-forcing 2/1 (a
/// two-level new suit over a one-of-a-major opening, or `1♦ - 2♣`) promises
/// thirteen-plus.
pub(super) fn apply_response_points(
    inf: &mut Envelope,
    response: Bid,
    opening: Bid,
    eligible: bool,
) {
    if !eligible {
        return;
    }
    match response.level.get() {
        1 => inf.narrow_points(Range::at_least(6, POINTS_CAP)),
        2 if is_american(opening, response) => {
            inf.narrow_points(Range::at_least(13, POINTS_CAP));
        }
        _ => {}
    }
}

/// Whether a two-level new suit is a game-forcing 2/1 over `opening`
fn is_american(opening: Bid, response: Bid) -> bool {
    response.level.get() == 2
        && match opening.strain {
            Strain::Hearts | Strain::Spades => true,
            Strain::Diamonds => response.strain == Strain::Clubs,
            _ => false,
        }
}

/// The hand-written convention readings of one auction, taken together
///
/// Each field is one reader's verdict, and `docs/reader-retirement.md` is the
/// ledger of which are still owed a chop (the Multi/Woolsey-X/DONT/Meckwell
/// four retired 2026-08-14 into their alerted rules' own projections).
/// Bundling them here makes a retirement a single-file edit: delete the
/// reader, its field, its arm in [`Readings::suppresses`], and its block in
/// [`Readings::apply`].
pub(super) struct Readings {
    rubens_suppress: [Option<usize>; 2],
    rubens_cue: Option<(usize, Suit)>,
    rubens_transfer: Option<(usize, Suit, u8)>,
    landy_relay: Option<usize>,
    their_landy: Option<TheirLandyReading>,
    their_multi: Option<TheirMultiReading>,
    penalty_x: Option<usize>,
    penalty_latch_doubles: Vec<(usize, Suit)>,
    overcall_double: Option<(usize, u8)>,
    gladiator: Option<GladiatorReading>,
}

impl Readings {
    /// Run every hand-written reader over `auction`
    pub(super) fn read(
        auction: &[Call],
        len: usize,
        profile: ReadingProfile,
        their: TheirDisclosures,
    ) -> Self {
        // Rubens advances name relay suits; identify them so the natural reading
        // skips them, and capture a cue-raise's strength to apply afterwards.
        let (rubens_suppress, rubens_cue, rubens_transfer) = rubens_reading(auction, profile);
        Self {
            rubens_suppress,
            rubens_cue,
            rubens_transfer,
            // The one suppression the projection cannot see: the advancer's 2♦ relay /
            // `2♥ - 2♠` preference over a Landy/Woolsey both-majors 2♣ names no length of its
            // own, so its rule projects nothing — suppress it by hand (the doc's stub).
            landy_relay: landy_advance_suppress(auction, profile),
            // The opponents' *disclosed* Landy 2♣ over our 1NT: their 2♣ is 4-4+
            // majors, not the natural walk's 5+ clubs and 8+, and their advances
            // are preferences.  Seat-gated on the disclosure, unlike the
            // symmetric readers around it.
            their_landy: their_landy_reading(auction, len, profile, their),
            // The disclosed foreign Multi has no authored opponent rule to
            // project, so preserve its `6+♥ | 6+♠` disjunction here.
            their_multi: their_multi_reading(auction, len, profile, their),
            // Our natural penalty double of their 1NT (15+): a double names no suit, so the
            // generic walk reads it as nothing — the points floor is recorded post-walk.
            penalty_x: penalty_x_reading_with_profile(auction, profile),
            // The latch's subsequent penalty doubles: each promises four-plus in the suit
            // it doubles, recorded post-walk so the sampler does not read them as takeout.
            penalty_latch_doubles: penalty_latch_double_reading(auction, profile),
            // Responder's double of an overcall of our 1NT shows 8+ (every DoubleStyle),
            // recorded post-walk so opener does not undercount the partnership's strength.
            overcall_double: responder_overcall_double_reading(auction, len, profile, their),
            // Our Gladiator advance of a 1NT overcall of their major: the 2♣ relay (and
            // its forced 2♦), the cue-Stayman, the 3M splinter, and the 4M both-minor
            // Leaping Michaels are bids of a suit the caller lacks — suppressed here,
            // real shape recorded post-walk.
            gladiator: gladiator_reading(auction, profile),
        }
    }

    /// Whether the call at `index` names a suit its bidder need not hold
    ///
    /// The natural walk must skip these: reading the named suit as length would
    /// assert a phantom holding.  What each call genuinely shows is recorded
    /// afterwards by [`Readings::apply`].
    pub(super) fn suppresses(&self, index: usize) -> bool {
        self.rubens_suppress.contains(&Some(index))
            || self.landy_relay == Some(index)
            || self.their_landy.is_some_and(|t| t.suppresses(index))
            || self.their_multi.is_some_and(|t| t.suppresses(index))
            || self.gladiator.is_some_and(|g| g.suppresses(index))
    }

    /// Record, after the walk, what the suppressed calls genuinely showed
    ///
    /// **The order of the blocks below is load-bearing.**  `Range::intersect`
    /// widens to the span on disjoint bounds rather than going empty, so it is
    /// not a meet and these narrowings do not commute.  The two Rubens blocks
    /// run *before* `overlay` is folded into `players`; the remaining readers
    /// run after.  `docs/reader-retirement.md` turns that ordering into the fifth
    /// condition of its subset escape — a reader ahead of the fold can widen an
    /// axis that the same narrowing applied after the fold would not.
    pub(super) fn apply(
        &self,
        players: &mut [Envelope; 4],
        overlay: &[Envelope; 4],
        overlay_unions: &mut [EnvelopeUnion; 4],
        agreement_unions: &mut [EnvelopeUnion; 4],
        len: usize,
        profile: ReadingProfile,
    ) {
        // A two-level cue-raise shows a limit-plus raise: three-plus cards in
        // partner's overcall and opening values.  Recorded after the walk (the
        // cue itself named the opponents' suit, suppressed above).
        if let Some((cue_index, overcall_suit)) = self.rubens_cue {
            let who = relative_of(len, cue_index) as usize;
            players[who].narrow_length(overcall_suit, Range::at_least(3, LENGTH_CAP));
            // Fit agreed (cue of partner's overcall), a support-scale promise;
            // the legacy axis takes only its sound image.
            let band = Range::at_least(10, POINTS_CAP);
            players[who].narrow_points(support_band_to_points(band));
            players[who].narrow_support_points(overcall_suit, band);
        }

        // A one-level Rubens transfer records its meaning likewise (see
        // `ReadingProfile::rubens_transfer`) — but only for the advancer's own
        // side: the transfer semantics are *our* agreement, and an opponent's
        // in-band advance may be a genuine suit (asserting length in the suit
        // above would poison the sampler).  Suppression above stays side-blind:
        // it only loses information, never asserts any.
        if let Some((transfer_index, suit, min_len)) = self.rubens_transfer {
            let who = relative_of(len, transfer_index);
            if matches!(who, Relative::Me | Relative::Partner) {
                let who = who as usize;
                players[who].narrow_length(suit, Range::at_least(min_len, LENGTH_CAP));
                players[who].narrow_points(Range::at_least(10, POINTS_CAP));
            }
        }

        // The declarative conventions (Jacoby transfers over our notrump,
        // Leaping Michaels, Landy's 2♣, and — since the 2026-08-14 chops — the
        // whole DONT/Woolsey/Multi/Meckwell 1NT-defense family, advances
        // included) are recorded from their authored rules' projections — the
        // `overlay` computed above — not hand-written decoders.  Post-FLIP the
        // envelope union carries each rule's disjunction itself (`Inferences::
        // admits` tests membership per box), so the per-suit floors the old
        // readers pinned by hand fall out of the very same rules, and the
        // strength bands come along where the readers never recorded them.
        for (seat, projected) in overlay.iter().enumerate() {
            players[seat] = players[seat].intersect(projected);
        }

        // Their disclosed Landy 2♣ over our 1NT: both majors, ≥ 4-4 (the natural
        // 5+ club and 8+ point readings were suppressed above).  No strength
        // claim and no club cap — their band and style are undeclared, and the
        // 5-4/4-5 split is a disjunction the per-suit framework cannot pin, so
        // the residual carries it (the same loose handling our own Landy uses).
        // The advances stay suppression-only: a preference can be a doubleton.
        if let Some(their_landy) = self.their_landy {
            let who = relative_of(len, their_landy.overcall_index) as usize;
            players[who].narrow_length(Suit::Hearts, Range::at_least(4, LENGTH_CAP));
            players[who].narrow_length(Suit::Spades, Range::at_least(4, LENGTH_CAP));
        }

        // Their disclosed Multi: one true union, not the hull that would admit
        // a 5-4 hand.  The natural diamond and pass-or-correct readings were
        // suppressed above; no strength or side-suit claim is added.
        if let Some(their_multi) = self.their_multi {
            let who = relative_of(len, their_multi.overcall_index) as usize;
            let mut hearts = Envelope::unknown();
            hearts.narrow_length(Suit::Hearts, Range::at_least(6, LENGTH_CAP));
            let mut spades = Envelope::unknown();
            spades.narrow_length(Suit::Spades, Range::at_least(6, LENGTH_CAP));
            // This disclosure is inherently disjunctive, not an optional
            // projection refinement: preserve both boxes even under the
            // legacy single-envelope projection setting.
            let shown = EnvelopeUnion::from(hearts).union(EnvelopeUnion::from(spades));
            players[who] = players[who].intersect(&shown.hull());
            overlay_unions[who].intersect_assign(&shown, profile);
            agreement_unions[who].intersect_assign(&shown, profile);
        }

        // Our Gladiator advance: record the real shape the suppressed call hid.
        // Guarded to our own side (the advance is our agreement) — an opponent's
        // in-band call must never be narrowed to the phantom suit.
        if let Some(gladiator) = self.gladiator {
            let who = relative_of(len, gladiator.index);
            if matches!(who, Relative::Me | Relative::Partner) {
                let who = who as usize;
                match gladiator.advance {
                    // No band: the relay is a *three-way* disjunction — a weak
                    // ♦/`o` takeout, any invitational hand, **or a game-forcing
                    // balanced hand with exactly three `o`** heading for the
                    // delayed cue (`gladiator_advances`, the 2♣ rule; its
                    // continuation authors the delayed cue `points(inv..)`,
                    // unbounded).  A `0..=9` cap here was intersected into the
                    // projection's game-forcing box and emptied it — a wrong
                    // box, not a loose one.  The strength reading is the
                    // authored rule's own union of boxes, and the suit stays
                    // unread (the XYZ-style rebid over 2♦ reveals it, read
                    // naturally).
                    GladiatorAdvance::Relay => {}
                    GladiatorAdvance::Cue { o } => {
                        players[who].narrow_length(o, Range::at_least(4, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                    }
                    // Delayed cue: exactly 3 in the unbid major, INV+ (checks the
                    // 5-3 fit an exactly-5-major overcall can hold).
                    GladiatorAdvance::DelayedCue { o } => {
                        players[who].narrow_length(o, Range::new(3, 3));
                        players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                    }
                    GladiatorAdvance::Splinter { o, m } => {
                        players[who].narrow_length(o, Range::at_least(4, LENGTH_CAP));
                        players[who].narrow_length(m, Range::new(0, 1));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    GladiatorAdvance::BothMinors => {
                        players[who].narrow_length(Suit::Clubs, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_length(Suit::Diamonds, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    GladiatorAdvance::Minor { o, minor } => {
                        players[who].narrow_length(o, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_length(minor, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    // 2NT = weak transfer to clubs: 6+ clubs, sub-invitational.
                    GladiatorAdvance::ClubTransfer => {
                        players[who].narrow_length(Suit::Clubs, Range::at_least(6, LENGTH_CAP));
                        players[who].narrow_points(Range::new(0, 7));
                    }
                }
            }
        }

        // Our natural penalty double of their 1NT.  The shape gate only widens *which*
        // 15+ hands double, so only the points floor is a sound per-call fact; recording
        // it stops the floor sampling the doubler as a random weak hand and the advancer
        // pulling a phantom suit (cf. the Woolsey double, which records points alone too).
        if let Some(double_index) = self.penalty_x {
            let who = relative_of(len, double_index) as usize;
            let floor = profile.natural_double_floor;
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }

        // The latch's later penalty doubles: four-plus in the doubled suit (the
        // floor makes them only on a trump stack), so partner reads them as penalty.
        for &(double_index, suit) in &self.penalty_latch_doubles {
            let who = relative_of(len, double_index) as usize;
            players[who].narrow_length(suit, Range::at_least(4, LENGTH_CAP));
        }

        // Responder's double of an overcall of our 1NT: 8+ values (every
        // DoubleStyle), or the Multi lane's own `hcp(6..)` under its knob.
        if let Some((double_index, floor)) = self.overcall_double {
            let who = relative_of(len, double_index) as usize;
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }
    }
}

#[cfg(test)]
mod tests;
