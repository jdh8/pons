//! The hand-written convention readers still awaiting retirement
//!
//! Each of these predates envelope unions and encodes a convention's meaning a
//! second time, beside its authoring rules.  `docs/reader-retirement.md` holds
//! the ledger; every function here is a chop candidate, and the tail helpers
//! below serve the natural walk in [`super::read`].

use super::envelope::{Envelope, Range};
use super::knobs::rubens_transfer_reading;
use super::{LENGTH_CAP, POINTS_CAP};
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
/// both ten-plus points ([`set_rubens_transfer_reading`], recorded post-walk
/// for the advancer's *own side only* — an opponent's in-band advance may be
/// natural).
///
/// The shown values are what let the overcaller judge game — and the completion
/// is a forced relay, still never read as length (soundness over tightness, as
/// with transfers over our own notrump).
#[allow(clippy::type_complexity)]
pub(super) fn rubens_reading(
    auction: &[Call],
) -> (
    [Option<usize>; 2],
    Option<(usize, Suit)>,
    Option<(usize, Suit, u8)>,
) {
    let none = ([None, None], None, None);
    // The bidder's knob governs the reading too: with Rubens advances off, an
    // advance in the band is a genuine suit and must be read naturally.
    if !crate::bidding::instinct::rubens_advances_enabled() {
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
    let transfer = rubens_transfer_reading().then_some(if target_suit == y {
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
pub(super) fn landy_advance_suppress(auction: &[Call]) -> Option<usize> {
    let on = crate::bidding::american::landy_range().is_some()
        || crate::bidding::american::woolsey_enabled();
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

/// Which Woolsey **Multi-family** overcall the defending side made over their 1NT
#[derive(Clone, Copy)]
pub(super) enum MultiKind {
    /// `2♦` Multi — a single 6+ major (unknown which), nothing else long.  Names a
    /// diamond suit it does not hold, so its natural reading must be suppressed.
    Major,
    /// `2♥`/`2♠` Muiderberg — exactly 5 in the named major, ≤ 3 in the other major
    /// (and a 4+ minor, captured by the residual).  A real major: no suppression.
    Muiderberg(Suit),
}

/// A Woolsey Multi-family overcall and which call it was
#[derive(Clone, Copy)]
pub(super) struct MultiReading {
    pub(super) overcall_index: usize,
    pub(super) kind: MultiKind,
    /// The advancer's `2♥`/`2♠` pass-or-correct over the Multi `2♦` (a preference
    /// among partner's unknown major — not own length), suppressed if present.
    pub(super) advance_suppress: Option<usize>,
}

impl MultiReading {
    /// Whether the call at `index` is artificial: the `2♦` Multi naming diamonds it
    /// does not hold, or the advancer's `2♥`/`2♠` pass-or-correct (a preference, not
    /// own length).  The Muiderberg `2♥`/`2♠` overcall names a real 5-card major, so
    /// its natural reading is kept.
    pub(super) fn suppresses(&self, index: usize) -> bool {
        (matches!(self.kind, MultiKind::Major) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a Woolsey **Multi-family** overcall of their 1NT: the `2♦` Multi (a single
/// 6+ major) or the `2♥`/`2♠` Muiderberg (exactly 5 in the major + a 4+ minor)
///
/// Gated on [`woolsey_enabled`][crate::bidding::american::woolsey_enabled] and the
/// auction being `1NT` then the defending side's first action being that bid.  The
/// both-majors `2♣` is read off its authored rule by the projection pass folded
/// into [`Inferences::read`] (Woolsey = Landy 2♣ + this family).
///
/// ponytail: kept separate so this Multi reading is reusable for a future Multi `2♦`
/// *opening* (an unknown-major weak two) — same shape, no 1NT prefix.
pub(super) fn multi_reading(auction: &[Call]) -> Option<MultiReading> {
    if !crate::bidding::american::woolsey_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The defending side's FIRST action — a 2♦/2♥/2♠ Multi-family overcall.
    let reading = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Diamonds) {
                    Some(MultiKind::Major)
                } else if bid == Bid::new(2, Strain::Hearts) {
                    Some(MultiKind::Muiderberg(Suit::Hearts))
                } else if bid == Bid::new(2, Strain::Spades) {
                    Some(MultiKind::Muiderberg(Suit::Spades))
                } else {
                    None
                };
                Some(kind.map(|kind| MultiReading {
                    overcall_index: index,
                    kind,
                    advance_suppress: None,
                }))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // Over the Multi 2♦, the advancer's 2♥/2♠ pass-or-correct picks one of partner's
    // unknown majors — a preference, not own length — so suppress it too (including a
    // doubled runout; the shared helper handles both).
    let advance_suppress = matches!(reading.kind, MultiKind::Major)
        .then(|| advancer_artificial(auction, reading.overcall_index, opener_parity))
        .flatten();

    Some(MultiReading {
        advance_suppress,
        ..reading
    })
}

/// Our **Gladiator** advance of a 1NT overcall of their major
/// ([`set_nt_overcall_gladiator`][crate::bidding::american::set_nt_overcall_gladiator])
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

pub(super) fn gladiator_reading(auction: &[Call]) -> Option<GladiatorReading> {
    if !crate::bidding::american::nt_overcall_gladiator() {
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
        return gladiator_reading(&stripped);
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

/// Our Woolsey takeout **double** of their 1NT and the advancer's `2♣` minor relay
///
/// The double shows a 4-card major plus a 5-6 card minor with the
/// [`woolsey_double_floor`][crate::bidding::american::woolsey_double_floor] points
/// floor.  The shape is a *double* disjunction (either major, either minor) the
/// per-suit framework cannot pin, so only the points floor is recorded post-walk —
/// but that alone matters: a double of 1NT names no suit, so the generic walk reads
/// it as *nothing* (the takeout-of-a-suit branch needs a suit opening), leaving the
/// floor to sample the doubler as a random hand.
///
/// The advancer's `2♣` over the double is a "name your minor" relay, not own clubs,
/// so its natural reading is suppressed.  Our own `2♥`/`2♠` advances are natural
/// majors and `2NT` is the notrump game-ask, so neither needs suppression.
#[derive(Clone, Copy)]
pub(super) struct WoolseyXReading {
    pub(super) double_index: usize,
    pub(super) relay_suppress: Option<usize>,
}

impl WoolseyXReading {
    pub(super) fn suppresses(&self, index: usize) -> bool {
        self.relay_suppress == Some(index)
    }
}

pub(super) fn woolsey_x_reading(auction: &[Call]) -> Option<WoolseyXReading> {
    if !crate::bidding::american::woolsey_enabled() {
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
            // The opener's side acted, or a defender did something else (an overcall)
            // — not our takeout double.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's first bid; suppress it only if it is the 2♣ minor relay.  Jump
    // over every opponent call so a contested relay is covered too (the 2♣ relay is
    // only legal as the immediate response, so the first such call is always it).
    let relay_suppress = auction
        .iter()
        .enumerate()
        .skip(double_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => {
                Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
            }
            _ => None,
        })
        .flatten();

    Some(WoolseyXReading {
        double_index,
        relay_suppress,
    })
}

/// The index of our natural **penalty** double of their 1NT (15+ HCP), or `None`
///
/// A double of 1NT names no suit, so the generic walk's takeout branch (which needs
/// a suit opening) reads it as nothing.  Returns the doubler's index so the post-walk
/// pass records the [`natural_double_floor`][crate::bidding::american::natural_double_floor]
/// points floor.  Mirrors [`woolsey_x_reading`].
///
/// Fires only when a double of their 1NT actually *means* the natural penalty double:
/// the natural defense is on and no convention has repurposed the double (DONT = a
/// one-suiter, direct Landy / Woolsey = both majors — each has its own reading).  A
/// *passed* doubler cannot hold 15+, so their double is the both-majors passed-hand
/// call, not penalty; an unpassed doubler is identified by lane (a seat that passed
/// before the opening occupies a lane below `opening_index`).
pub(in crate::bidding) fn penalty_x_reading(auction: &[Call]) -> Option<usize> {
    use crate::bidding::american as a;
    // One `Cell<NotrumpDefense>` holds one system, so "Natural is active" is the
    // whole test: the four "…but not DONT/Meckwell/direct-Landy/Woolsey"
    // disjuncts this used to carry were the pre-fold precedence cascade, and
    // every one of them was unreachable once the enum landed.
    if !a::natural_defense_enabled() {
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
pub(super) fn responder_overcall_double_reading(auction: &[Call], len: usize) -> Option<usize> {
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
    (auction.get(opening_index + 2) == Some(&Call::Double)).then_some(opening_index + 2)
}

/// Our side's *subsequent* penalty doubles after the natural penalty X of their
/// 1NT — the latch's later doubles — each paired with the suit it doubles
///
/// The penalty latch ([`set_penalty_latch`][crate::bidding::instinct::set_penalty_latch])
/// makes these via the trump-stack rule, so each promises four-plus cards in the
/// doubled suit.  Recording that length stops the sampler reading the double as
/// takeout — without it the advancer pulls a penalty double thinking partner is
/// short, the phantom-suit leak the [`penalty_x_reading`] doc names.  Empty unless
/// the latch is on, so it agrees with the floor on when a later double is penalty.
///
/// Once we penalty-double their 1NT the penalty stance holds for the rest of the
/// auction (mirrors `penalty_latched`) — a bid of our own does *not* un-latch it,
/// it only updates the suit a later penalty double refers to.
pub(super) fn penalty_latch_double_reading(auction: &[Call]) -> Vec<(usize, Suit)> {
    if !crate::bidding::instinct::penalty_latch_enabled() {
        return Vec::new();
    }
    let Some(x_index) = penalty_x_reading(auction) else {
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

/// Which DONT defense call the defending side made over their 1NT
#[derive(Clone, Copy)]
pub(super) enum DontKind {
    /// `X` — a one-suiter in ♣/♦/♥ (a spade one-suiter bids the natural `2♠`), so
    /// spades are short.  The long suit is a triple disjunction the per-suit
    /// framework cannot pin; only `spades ≤ 3` is a sound per-suit fact.
    OneSuiter,
    /// `2♣` — clubs (real, ≥ 4) + an unknown higher major.  Names a real club suit,
    /// but the natural ≥ 5 reading is unsound (the 4-major-5-club hand has 4 clubs).
    ClubsMajor,
    /// `2♦` — diamonds (real, ≥ 4) + an unknown major.  As `ClubsMajor` for diamonds.
    DiamondsMajor,
    /// `2♥` — both majors, ≥ 5-4.  Exactly a Landy two-suiter on the `2♥` bid.
    BothMajors,
}

/// A DONT overcall of their 1NT (`X`/`2♣`/`2♦`/`2♥`) and the advancer's relay
///
/// DONT's calls name suits the hand may not hold (`X` names none; `2♣`/`2♦`/`2♥` can
/// be only 4 cards in the named suit) or are relays, so the generic walk misreads
/// them — leaving the floor to raise a phantom suit or sample a random hand.  The
/// natural `2♠` is a genuine spade suit and needs no reading.  Mirrors
/// [`multi_reading`] / [`woolsey_x_reading`].
#[derive(Clone, Copy)]
pub(super) struct DontReading {
    pub(super) overcall_index: usize,
    pub(super) kind: DontKind,
    pub(super) floor: u8,
    /// The advancer's relay — `2♣` over the `X`, or the `2♦`/`2♥`/`2♠` pass-or-correct
    /// over `2♣`/`2♦`/`2♥` (a preference among partner's suits, not own length).
    pub(super) advance_suppress: Option<usize>,
}

impl DontReading {
    /// Whether the call at `index` is artificial.  The `X` (a double) names no suit,
    /// so only the `2♣`/`2♦`/`2♥` overcalls suppress their own natural reading; the
    /// advancer's relay is always suppressed.
    pub(super) fn suppresses(&self, index: usize) -> bool {
        (!matches!(self.kind, DontKind::OneSuiter) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a DONT overcall of their 1NT, gated on
/// [`direct_dont_enabled`][crate::bidding::american::direct_dont_enabled] and the
/// auction being `1NT` then the defending side's first action being a DONT call
pub(super) fn dont_reading(auction: &[Call]) -> Option<DontReading> {
    if !crate::bidding::american::direct_dont_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;
    let floor = crate::bidding::american::natural_overcall_points().0;

    // The defending side's FIRST action — a DONT `X`/`2♣`/`2♦`/`2♥` (the natural `2♠`
    // and anything else fall through to the generic reading).
    let (overcall_index, kind) = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some((index, DontKind::OneSuiter))),
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Clubs) {
                    Some(DontKind::ClubsMajor)
                } else if bid == Bid::new(2, Strain::Diamonds) {
                    Some(DontKind::DiamondsMajor)
                } else if bid == Bid::new(2, Strain::Hearts) {
                    Some(DontKind::BothMajors)
                } else {
                    None
                };
                Some(kind.map(|kind| (index, kind)))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's relay: `2♣` over the `X` (it names a minor, not own clubs), or the
    // `2♦`/`2♥`/`2♠` preference over a two-suiter (one of partner's suits, not own
    // length).  Both scans jump over every opponent call so a contested relay is
    // covered (the relay is only legal as the immediate response).
    let advance_suppress = match kind {
        DontKind::OneSuiter => auction
            .iter()
            .enumerate()
            .skip(overcall_index + 1)
            .find_map(|(index, &call)| match call {
                Call::Bid(bid) if index % 2 != opener_parity => {
                    Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
                }
                _ => None,
            })
            .flatten(),
        _ => advancer_artificial(auction, overcall_index, opener_parity),
    };

    Some(DontReading {
        overcall_index,
        kind,
        floor,
        advance_suppress,
    })
}

/// Which Meckwell defense call the defending side made over their 1NT
#[derive(Clone, Copy)]
pub(super) enum MeckwellKind {
    /// `X` — a single 6+ minor OR both majors.  A double naming no suit, and a
    /// disjunction (short majors OR long majors) the per-suit framework cannot pin, so
    /// only the points floor is a sound fact (as the Woolsey / penalty double).
    TwoWayDouble,
    /// `2♣` — clubs (real, ≥ 4) + an unknown major.  As DONT's `ClubsMajor`.
    ClubsMajor,
    /// `2♦` — diamonds (real, ≥ 4) + an unknown major.  As DONT's `DiamondsMajor`.
    DiamondsMajor,
}

/// A Meckwell overcall of their 1NT (`X`/`2♣`/`2♦`) and the advancer's relay
///
/// Meckwell's natural `2♥`/`2♠` single-suiters name real suits (read by the generic
/// walk) and the `2NT` both-minors is the Unusual overlay, so only the two-way `X` and
/// the `2♣`/`2♦` minor + major are decoded here.  Mirrors [`dont_reading`].
#[derive(Clone, Copy)]
pub(super) struct MeckwellReading {
    pub(super) overcall_index: usize,
    pub(super) kind: MeckwellKind,
    pub(super) floor: u8,
    /// The advancer's relay — `2♣` over the `X`, or the `2♦`/`2♥`/`2♠` pass-or-correct
    /// over `2♣`/`2♦` (a preference among partner's suits, not own length).
    pub(super) advance_suppress: Option<usize>,
}

impl MeckwellReading {
    /// The `X` (a double) names no suit, so only the `2♣`/`2♦` overcalls suppress
    /// their own natural reading; the advancer's relay is always suppressed.
    pub(super) fn suppresses(&self, index: usize) -> bool {
        (!matches!(self.kind, MeckwellKind::TwoWayDouble) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a Meckwell overcall of their 1NT, gated on
/// [`meckwell_enabled`][crate::bidding::american::meckwell_enabled] and the auction
/// being `1NT` then the defending side's first action being a Meckwell call
pub(super) fn meckwell_reading(auction: &[Call]) -> Option<MeckwellReading> {
    if !crate::bidding::american::meckwell_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;
    let floor = crate::bidding::american::natural_overcall_points().0;

    // The defending side's FIRST action — a Meckwell `X`/`2♣`/`2♦` (natural `2♥`/`2♠`
    // and anything else fall through to the generic reading).
    let (overcall_index, kind) = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => {
                Some(Some((index, MeckwellKind::TwoWayDouble)))
            }
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Clubs) {
                    Some(MeckwellKind::ClubsMajor)
                } else if bid == Bid::new(2, Strain::Diamonds) {
                    Some(MeckwellKind::DiamondsMajor)
                } else {
                    None
                };
                Some(kind.map(|kind| (index, kind)))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's relay: `2♣` over the `X` (names a minor, not own clubs), or the
    // `2♦`/`2♥`/`2♠` preference over a two-suiter.  Both scans jump over every opponent
    // call so a contested relay is covered (the relay is only legal as the immediate
    // response).
    let advance_suppress = match kind {
        MeckwellKind::TwoWayDouble => auction
            .iter()
            .enumerate()
            .skip(overcall_index + 1)
            .find_map(|(index, &call)| match call {
                Call::Bid(bid) if index % 2 != opener_parity => {
                    Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
                }
                _ => None,
            })
            .flatten(),
        _ => advancer_artificial(auction, overcall_index, opener_parity),
    };

    Some(MeckwellReading {
        overcall_index,
        kind,
        floor,
        advance_suppress,
    })
}

/// Apply the meaning of the opening bid (the first non-pass call)
pub(super) fn apply_opening(inf: &mut Envelope, bid: Bid, seat: u8) {
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
            // `set_one_notrump_fifths` knob, if ever revived, would re-widen
            // this to 14–19.
            let slack = crate::bidding::constraint::flat_hcp_slack();
            inf.narrow_points(Range::new(15 - slack, 18));
            // The `hcp` gauge is crisp raw HCP — 15–17 gates the opening, with
            // no upgrade slack (notrump valuation, read behind Edit 2's knob).
            inf.narrow_hcp(Range::new(15, 17));
        }
        (2, Strain::Clubs) => {
            // Strong and artificial: 22+ points, but nothing about shape.
            inf.narrow_points(Range::at_least(20, POINTS_CAP));
        }
        (2, Strain::Notrump) => {
            if crate::bidding::american::two_notrump_wide() {
                // Chop G0: the wide-minor 2NT (`set_two_notrump_wide`) caps
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
            let slack = crate::bidding::constraint::flat_hcp_slack();
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

#[cfg(test)]
mod tests;
