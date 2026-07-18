//! The Dutch system — a natural 2/1 built around a wide, non-forcing 1♣
//!
//! Dutch naturalises the Polish 1♣: a "lawyer's Polish Club" that keeps Polish
//! constructiveness while staying natural and less restricted.  The 1♣ opening
//! is non-forcing, 2+♣, 11–23 HCP, and hosts every strong hand that lacks the
//! strong-2♣ shape (the `1♣–1♦` relay sorts them out).  Otherwise it mirrors
//! `american()`: five-card majors, a 15–17 1NT, 2/1 game-forcing continuations.
//!
//! This is a **champion candidate**, built by copying `american()` and applying
//! the Dutch diff one measurable phase at a time.  Until it measures stronger,
//! it lives here as a sibling factory under the standard A/B discipline; see
//! `docs/dutch-system.md` for the campaign ledger.

mod openings;
mod responses;

use super::Pair;
use super::american::{bare_american, insert_uncontested, with_instinct_floor};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// A bid as a [`Call`], for trie keys
const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// Build the Dutch system as one side's [`Pair`]
///
/// Bind it against the opponents' [`Family`][super::Family] with
/// [`Pair::against`] and seat two pairs with [`Table::of_pairs`][super::Table::of_pairs],
/// exactly like `american()`.
///
/// ```
/// use pons::dutch;
/// use pons::bidding::{Family, System};
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = dutch().against(Family::NATURAL);
/// let hand = "AQ32.K53.QJ4.A92".parse().unwrap(); // 16 HCP, balanced
/// let logits = stance
///     .classify(hand, RelativeVulnerability::NONE, &[])
///     .expect("an opening decision");
/// let best = (&logits.0)
///     .into_iter()
///     .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
///     .map(|(call, _)| call)
///     .unwrap();
/// assert_eq!(best, Call::Bid(Bid::new(1, Strain::Notrump)));
/// ```
#[must_use]
pub fn dutch() -> Pair {
    with_instinct_floor(bare_dutch())
}

/// The Dutch pair without the instinct floor — the authored books
///
/// Takes a full [`bare_american`] pair and overwrites the **divergent nodes**
/// (`Trie::insert_arc` replaces the classifier at each key); every other
/// american continuation is reused verbatim.  Phase 1 overwrote the opening
/// table ([`openings::dutch_openings`]); Phase 2.1 overwrites the wide-1♣
/// response node and opener's rebid after the `1♦` relay; Phase 2.2 adds
/// responder's second call over opener's minimum rebids (`1♣-1♦-1M`,
/// `1♣-1♦-2♣`).  The rare 18–20 `1NT` / 21–23 `2♦!` continuations stay
/// american's — projection discloses their strength; see `docs/dutch-system.md`.
fn bare_dutch() -> Pair {
    let mut pair = bare_american();
    let book = &mut pair.constructive.0;
    // `insert_uncontested` re-keys at the undisturbed auction for every seat,
    // and `Trie::insert_arc` replaces the classifier there — a clean overwrite.
    insert_uncontested(book, &[], openings::dutch_openings());
    let one_club = call(1, Strain::Clubs);
    let relay = call(1, Strain::Diamonds);
    insert_uncontested(book, &[one_club], responses::one_club_responses());
    insert_uncontested(
        book,
        &[one_club, relay],
        responses::opener_rebids_after_relay(),
    );
    // Phase 2.2 increment 1 — responder's second call after opener's *minimum*
    // relay rebids (11–17), the high-frequency landing spots.  Deeper opener
    // rebids (18–20 `1NT`, 21–23 `2♦!`) still fall to the floor, which reads
    // their self-disclosed strength off the alerted rule; see `docs/dutch-system.md`.
    insert_uncontested(
        book,
        &[one_club, relay, call(1, Strain::Hearts)],
        responses::relay_responses_after_major(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_club, relay, call(1, Strain::Spades)],
        responses::relay_responses_after_major(Suit::Spades),
    );
    insert_uncontested(
        book,
        &[one_club, relay, call(2, Strain::Clubs)],
        responses::relay_responses_after_club(),
    );
    // Phase 2.2 increment 2 — opener's rebid after responder's natural minor
    // two-level responses.  These overwrite american's inverted-raise (`2♣`) and
    // weak-jump-shift (`2♦`) continuations, which misread the Dutch meanings
    // (invite+ 5+♣ / game-forcing 5+♦); see `docs/dutch-system.md`.
    insert_uncontested(
        book,
        &[one_club, call(2, Strain::Diamonds)],
        responses::opener_rebids_after_two_diamonds(),
    );
    insert_uncontested(
        book,
        &[one_club, call(2, Strain::Clubs)],
        responses::opener_rebids_after_two_clubs(),
    );
    pair
}

#[cfg(test)]
mod tests {
    use super::dutch;
    use crate::bidding::{Family, System};
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain};

    /// The Dutch opening for a first-seat hand.
    fn opens(hand: &str) -> Call {
        let stance = dutch().against(Family::NATURAL);
        let hand = hand.parse().unwrap();
        let logits = stance
            .classify(hand, RelativeVulnerability::NONE, &[])
            .expect("an opening decision");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(call, _)| call)
            .unwrap()
    }

    fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// The wide-1♣ opening partition (Phase 1): the load-bearing cases.
    #[test]
    fn opening_partition() {
        // The wide 1♣ hosts a strong balanced 23-count (american opens it 2♣).
        assert_eq!(opens("AKQ2.KQ3.KQ3.A32"), bid(1, Strain::Clubs));
        // Four-diamond hands open 1♣ — every one but the 4=4=4=1.
        assert_eq!(opens("KQ32.K32.KJ32.32"), bid(1, Strain::Clubs));
        // The singleton-club 4=4=4=1 is the one four-diamond hand that opens 1♦.
        assert_eq!(opens("KQ32.KQ32.Q432.2"), bid(1, Strain::Diamonds));
        // A real five-card diamond suit opens 1♦.
        assert_eq!(opens("A32.3.KQ432.K432"), bid(1, Strain::Diamonds));
        // 21–23 with a five-card major is the strong, artificial 2♣.
        assert_eq!(opens("AKQ32.AK3.AQ2.32"), bid(2, Strain::Clubs));
        // A balanced 16 opens 1NT, and — american's wide shape — so does a 5422
        // or 6322 with a long *minor* (was the wide 1♣ before the widening).
        assert_eq!(opens("AQ32.K53.QJ4.A92"), bid(1, Strain::Notrump));
        assert_eq!(opens("Q432.KQ.K2.AK432"), bid(1, Strain::Notrump)); // 5422, 5♣
        assert_eq!(opens("Q2.K3.AQ4.KQ8765"), bid(1, Strain::Notrump)); // 6322, 6♣
        // A 5422 with the five-card suit a *major* stays a suit opening (1♠).
        assert_eq!(opens("AK432.KQ.Q432.K2"), bid(1, Strain::Spades));
        // Rule of 20 gates the light end: a flat 12-count passes.
        assert_eq!(opens("KJ32.K32.K32.Q32"), Call::Pass);
    }

    /// The Dutch call after an undisturbed `auction`.
    fn responds(auction: &[Call], hand: &str) -> Call {
        let stance = dutch().against(Family::NATURAL);
        let hand = hand.parse().unwrap();
        let logits = stance
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("a decision");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(call, _)| call)
            .unwrap()
    }

    /// Responder's first call over the wide 1♣ (Phase 2.1).
    #[test]
    fn wide_1c_responses() {
        const P: Call = Call::Pass;
        let one_club = [bid(1, Strain::Clubs), P];
        // Weak, club-tolerant (3 HCP, 4♣): content to play 1♣.
        assert_eq!(responds(&one_club, "xxx.xxx.xxx.Kxxx"), P);
        // 5 HCP 4-4 majors, too weak for a 7+ major: the artificial relay.
        assert_eq!(
            responds(&one_club, "Kxxx.Qxxx.xxx.xx"),
            bid(1, Strain::Diamonds)
        );
        // 16 HCP, 5+♦, no four-card major: natural game force.
        assert_eq!(
            responds(&one_club, "Axx.Kx.AQxxx.Kxx"),
            bid(2, Strain::Diamonds)
        );
    }

    /// Opener's rebid after the 1♣-1♦ relay (Phase 2.1).
    #[test]
    fn opener_rebids() {
        const P: Call = Call::Pass;
        let relay = [bid(1, Strain::Clubs), P, bid(1, Strain::Diamonds), P];
        // 19 HCP balanced: the 18–20 notrump rebid.
        assert_eq!(
            responds(&relay, "AQx.KJx.KQx.Axxx"),
            bid(1, Strain::Notrump)
        );
        // 21 HCP, no 5-card major / 6-card minor / 5-5 minors: the artificial 2♦.
        assert_eq!(
            responds(&relay, "AKQ.x.AQxx.AQxxx"),
            bid(2, Strain::Diamonds)
        );
    }

    /// Responder's second call after opener's minimum relay rebid (Phase 2.2).
    #[test]
    fn relay_deep_continuations() {
        const P: Call = Call::Pass;
        let c = bid(1, Strain::Clubs);
        let d = bid(1, Strain::Diamonds);
        // After 1♣-1♦-1♥: the 5♠/4♥ two-suiter (8 pts) is Reverse Flannery — a
        // raise to 2♥, not a natural spade bid.
        let after_1h = [c, P, d, P, bid(1, Strain::Hearts), P];
        assert_eq!(
            responds(&after_1h, "KQxxx.Kxxx.xx.xx"),
            bid(2, Strain::Hearts)
        );
        // Both minors (5-4), 10 pts: 2♠ (the other major, repurposed).
        assert_eq!(
            responds(&after_1h, "x.xx.KQxxx.AJxx"),
            bid(2, Strain::Spades)
        );
        // After 1♣-1♦-1♠: the same two-suiter raises spades (2♠); both-minors is 2♥.
        let after_1s = [c, P, d, P, bid(1, Strain::Spades), P];
        assert_eq!(
            responds(&after_1s, "KQxxx.Kxxx.xx.xx"),
            bid(2, Strain::Spades)
        );
        assert_eq!(
            responds(&after_1s, "x.xx.KQxxx.AJxx"),
            bid(2, Strain::Hearts)
        );
        // After 1♣-1♦-2♣: the two-suiter shows as 2♥; an invitational club raise is 2♠.
        let after_2c = [c, P, d, P, bid(2, Strain::Clubs), P];
        assert_eq!(
            responds(&after_2c, "KQxxx.Kxxx.xx.xx"),
            bid(2, Strain::Hearts)
        );
        assert_eq!(
            responds(&after_2c, "Qxx.x.Qxxx.AQxx"),
            bid(2, Strain::Spades)
        );
    }

    /// Opener's rebid after responder's game-forcing 2♦ (Phase 2.2 increment 2).
    #[test]
    fn opener_rebids_after_two_diamonds() {
        const P: Call = Call::Pass;
        let a = [bid(1, Strain::Clubs), P, bid(2, Strain::Diamonds), P];
        // Four-card diamond support — raise the known nine-card fit.
        assert_eq!(responds(&a, "Axx.Kx.KJxx.Qxx"), bid(3, Strain::Diamonds));
        // Five clubs, short diamonds — the real second suit.
        assert_eq!(responds(&a, "Ax.Kx.xxx.AQxxx"), bid(3, Strain::Clubs));
        // Balanced 16, both majors stopped — to play.
        assert_eq!(responds(&a, "AQx.KQx.Qxx.Kxxx"), bid(3, Strain::Notrump));
        // Heart stopper only — shown up the line toward 3NT.
        assert_eq!(responds(&a, "xxx.AQx.Kxx.Kxxx"), bid(2, Strain::Hearts));
        // Spade stopper only — the other up-the-line stopper show.
        assert_eq!(responds(&a, "AQx.xxx.Kxx.Kxxx"), bid(2, Strain::Spades));
        // Minimum, no major stopper — the notrump catch-all (never Pass).
        assert_eq!(responds(&a, "xxx.xxx.KQx.AQxx"), bid(2, Strain::Notrump));
    }

    /// Opener's rebid after responder's invitational-or-better 2♣ (Phase 2.2 inc.2).
    #[test]
    fn opener_rebids_after_two_clubs() {
        const P: Call = Call::Pass;
        let a = [bid(1, Strain::Clubs), P, bid(2, Strain::Clubs), P];
        // Balanced 16, both majors stopped — accept to game.
        assert_eq!(responds(&a, "AQx.KQx.Qxx.Kxxx"), bid(3, Strain::Notrump));
        // 18 balanced but a major unstopped — a maximum still forces game.
        assert_eq!(responds(&a, "Axx.xxx.AKx.AKxx"), bid(3, Strain::Notrump));
        // Balanced 13, only two clubs — the non-forcing 2NT decline.
        assert_eq!(responds(&a, "AQx.KQx.Qxxx.xx"), bid(2, Strain::Notrump));
        // Minimum with club support — the non-forcing 3♣ decline.
        assert_eq!(responds(&a, "AQx.Kxx.xx.KJxx"), bid(3, Strain::Clubs));
    }

    /// Responder places the contract off the **floor** after opener's rebid —
    /// no authored responder node this increment (measured to bid correctly).
    /// These lock in the reliance: a floor change that breaks them flags for
    /// re-review (and would be the cue to author responder re-rebids).
    #[test]
    fn responder_places_contract_off_floor() {
        const P: Call = Call::Pass;
        let c = bid(1, Strain::Clubs);
        let d2 = bid(2, Strain::Diamonds);
        let c2 = bid(2, Strain::Clubs);
        // GF 2♦, opener raises diamonds: balanced 15, both stopped → 3NT.
        let gf_3d = [c, P, d2, P, bid(3, Strain::Diamonds), P];
        assert_eq!(responds(&gf_3d, "KJx.KQ.AQxxx.xx"), bid(3, Strain::Notrump));
        // GF 2♦, opener shows a heart stopper: responder has spades → 3NT.
        let gf_2h = [c, P, d2, P, bid(2, Strain::Hearts), P];
        assert_eq!(responds(&gf_2h, "Kxx.xx.AKxxx.Qx"), bid(3, Strain::Notrump));
        // Invite+ 2♣, opener declines 3♣: a dead minimum passes …
        let inv_3c = [c, P, c2, P, bid(3, Strain::Clubs), P];
        assert_eq!(responds(&inv_3c, "Kx.Qx.Qxx.KJxxxx"), P);
        // … and a game force drives to 3NT.
        assert_eq!(
            responds(&inv_3c, "Ax.Kx.Qxx.KQxxxx"),
            bid(3, Strain::Notrump)
        );
        // Invite+ 2♣, opener accepts 3NT: responder passes the game.
        let inv_3nt = [c, P, c2, P, bid(3, Strain::Notrump), P];
        assert_eq!(responds(&inv_3nt, "Kx.Qx.Qxx.KJxxxx"), P);
    }
}
