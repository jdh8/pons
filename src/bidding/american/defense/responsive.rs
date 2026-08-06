//! Responsive doubles — when partner doubles (or overcalls) and they raise
//!
//! Their raise removes the room to bid two suits, so the second double is
//! takeout of the remaining ones.  [`set_responsive_takeout`] covers the
//! doubled auction, [`set_responsive_overcall`] the overcalled one.
use super::*;

thread_local! {
    /// Whether the responsive double after partner's **takeout double** + their
    /// raise (`[1t, X, raise]`) is authored; see [`set_responsive_takeout`].
    static RESPONSIVE_TAKEOUT: Cell<bool> = const { Cell::new(true) };
    /// Whether the responsive double after partner's **overcall** + their raise
    /// (`[1t, overcall, raise]`) is authored; see [`set_responsive_overcall`].
    static RESPONSIVE_OVERCALL: Cell<bool> = const { Cell::new(false) };
}

/// Toggle the responsive double after partner's **takeout double** and their
/// raise (`(1t)–X–(2t)–?`) for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// **On by default** (the shipped behavior): advancer's double of the raise shows
/// the two unbid suits with 8+. Turn it off to drop the node to the instinct
/// floor — the A/B knob for `examples/responsive-ab --conv takeout`. This is the
/// canonical "responsive double" (BBA's single `Responsive double` toggle, on in
/// `21GF.bbsa`); see `docs/ai-bidder/21gf-ledger.md`.
pub fn set_responsive_takeout(on: bool) {
    RESPONSIVE_TAKEOUT.with(|cell| cell.set(on));
}

/// Whether the takeout-double responsive double is currently authored
pub(crate) fn responsive_takeout_enabled() -> bool {
    RESPONSIVE_TAKEOUT.with(Cell::get)
}

/// Toggle the responsive double after partner's **overcall** and their raise
/// (`(1t)–overcall–(2t)–?`) for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// **Off by default** (the auction falls to the instinct floor). When on, advancer's
/// double of the raise shows the two suits unbid by opener and partner with 8+ — a
/// non-standard extension of our own (BBA's `Responsive double` is only the takeout
/// version; the nearest overcall toggle, `Snapdragon Double`, is off in `21GF.bbsa`
/// and over a *new suit*, not a raise). The A/B knob for
/// `examples/responsive-ab --conv overcall`; see `docs/ai-bidder/21gf-ledger.md`.
pub fn set_responsive_overcall(on: bool) {
    RESPONSIVE_OVERCALL.with(|cell| cell.set(on));
}

/// Whether the overcall responsive double is currently authored
fn responsive_overcall_enabled() -> bool {
    RESPONSIVE_OVERCALL.with(Cell::get)
}

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
/// `open`, and they raised (`(1t)–overcall–(2t)–?`)
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
        gate: responsive_takeout_enabled,
        entries: || {
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
/// (`--conv overcall`) turns it on; see [`set_responsive_overcall`].
pub(super) fn responsive_overcall_package() -> Package {
    Package {
        name: "responsive-double-over-overcall",
        gate: responsive_overcall_enabled,
        entries: || {
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
