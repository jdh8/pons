//! Responsive doubles — when partner doubles (or overcalls) and they raise
//!
//! Their raise removes the room to bid two suits, so the second double is
//! takeout of the remaining ones.  `agreements.defense.responsive_takeout_enabled` covers the
//! doubled auction, `agreements.defense.responsive_overcall_enabled` the overcalled one.
use super::*;

/// Advancer's action when partner made a takeout double and they raised `t` to `raise_lvl`
///
/// Responsive double: both suits of the rank opposite the opened suit (minor/major).
/// Natural bids at the minimum legal level (2–3) for suits other than `t`, 5-card, 8+ HCP.
fn responsive_doubles(t: Suit, _raise_lvl: u8) -> Rules {
    // Responsive double shows the two unbid suits of the same rank (minor or major).
    let mut rules = if matches!(t, Suit::Hearts | Suit::Spades) {
        // t major → both minors
        Rules::new()
            .rule(
                Call::Double,
                150,
                len(Suit::Clubs, 4..) & len(Suit::Diamonds, 4..) & points(8..),
            )
            .alert(RESPONSIVE)
    } else {
        // t minor → both majors
        Rules::new()
            .rule(
                Call::Double,
                150,
                len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & points(8..),
            )
            .alert(RESPONSIVE)
    };

    rules = rules.rule(Call::Pass, 0, hcp(0..));

    // Natural bids for suits ≠ t at levels 2 and 3.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == t {
            continue;
        }
        let strain = Strain::from(suit);
        for bid_lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(bid_lvl, strain),
                100,
                min_level_is(bid_lvl, strain) & len(suit, 5..) & points(8..),
            );
        }
    }
    rules
}

/// Advancer's responsive double after partner *overcalled* `overcall` over their
/// `open`, and they raised (`(1t) overcall (2t) ?`)
///
/// A single-rule node: a `Call::Double` showing the two suits unbid by opener and
/// partner (all four minus `{open, overcall}`), 4+ in each, 8+ points.  By design it
/// has **no** catch-all — a hand that does not qualify gets all `-∞` logits and falls
/// through to the instinct floor's natural advances (mass-aware shadowing,
/// [`Trie::classify_floored`]), so this *layers* a responsive double onto the floor
/// rather than replacing it.  `Double` is always legal here (the opponents have a live
/// contract), so the lone rule cannot trip the silent-pass trap.
//
// ponytail: faithful reconstruction of the never-committed "8+ floor double" (ledger
// #100); off by default, the A/B knob for `examples/responsive-ab --conv overcall`.
fn responsive_overcall_doubles(open: Suit, overcall: Suit, _raise_lvl: u8) -> Rules {
    let mut unbid = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .filter(|&s| s != open && s != overcall);
    let s1 = unbid.next().expect("two suits remain unbid");
    let s2 = unbid.next().expect("two suits remain unbid");
    Rules::new()
        .rule(Call::Double, 150, len(s1, 4..) & len(s2, 4..) & points(8..))
        .alert(RESPONSIVE)
}

/// Responsive doubles: partner doubled for takeout, they raised
///
/// On by default; the A/B knob (`--conv takeout`) turns it off to compare the
/// shipped node against the bare floor.
pub(super) fn responsive_double_package() -> Package {
    Package {
        name: "responsive-double",
        gate: |agreements| agreements.defense.responsive_takeout_enabled,
        entries: |_| {
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                for raise_lvl in [2u8, 3] {
                    let raise = Bid::new(raise_lvl, theirs);
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* ({opening}) X ({raise})")),
                        responsive_doubles(suit, raise_lvl),
                    ));
                }
            }
            entries
        },
    }
}

/// Responsive double after partner's *overcall* and their raise
///
/// Off by default: the auction is otherwise floored.  The A/B knob
/// (`--conv overcall`) turns it on; see `agreements.defense.responsive_overcall_enabled`.
pub(super) fn responsive_overcall_package() -> Package {
    Package {
        name: "responsive-double-over-overcall",
        gate: |agreements| agreements.defense.responsive_overcall_enabled,
        entries: |_| {
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                for over in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if over == suit {
                        continue;
                    }
                    // Partner's natural overcall of `over` at its minimum level
                    // over 1t: the 1-level if it outranks their suit, else the 2.
                    let over_lvl = if over > suit { 1 } else { 2 };
                    let overcall = Bid::new(over_lvl, Strain::from(over));
                    for raise_lvl in [2u8, 3] {
                        let raise = Bid::new(raise_lvl, theirs);
                        entries.extend(rows_of(
                            Pattern::node(&format!("P* ({opening}) {overcall} ({raise})")),
                            responsive_overcall_doubles(suit, over, raise_lvl),
                        ));
                    }
                }
            }
            entries
        },
    }
}
