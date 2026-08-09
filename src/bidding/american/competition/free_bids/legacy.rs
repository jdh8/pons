use super::*;

/// The retired guarded wiring of [`free_bid_answer_package`]'s free-bid and
/// negative-free-bid tables (4d″/4d‴ ride along verbatim), kept as the
/// resolution-equivalence oracle for `converted_packages_match_legacy`
#[cfg(test)]
pub(crate) fn free_bid_answer_package_legacy() -> Package {
    Package {
        name: "free-bid-answer",
        gate: |_| free_bids_engaged(),
        entries: |_| {
            let cachalot = negative_double_shape() == NegativeDoubleShape::Cachalot;
            let negative = free_bid_style() == FreeBidStyle::Negative;
            let transfer = free_bid_style() == FreeBidStyle::Transfer;
            let mut entries = Vec::new();
            for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let o_strain = Strain::from(opening);
                let rotated = cachalot && matches!(opening, Suit::Clubs | Suit::Diamonds);
                let key = format!("P* 1{o_strain}");
                // A reachable free bid.  Only a minor opening leaves room for
                // the 1-level rung — over `1♥`/`1♠` every legal overcall is
                // high enough that the cheapest new suit already sits at the
                // two level — and Cachalot's rotation claims the minors' too.
                let free_sample = match (opening, rotated) {
                    (Suit::Clubs, false) => "(1♦) 1♥ -",
                    (Suit::Diamonds, false) => "(1♥) 1♠ -",
                    (Suit::Diamonds | Suit::Hearts, _) => "(2♣) 2♠ -",
                    _ => "(2♦) 2♥ -",
                };
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        free_sample,
                        described_guard(
                            if rotated {
                                "(overcall ≤2♠) 2-level free-suit -"
                            } else {
                                "(overcall ≤2♠) free-suit -"
                            },
                            guard(move |_: &Context<'_>, suffix: &[Call]| {
                                matches!(
                                    suffix,
                                    [Call::Bid(ovc), Call::Bid(free), Call::Pass]
                                        if *ovc <= Bid::new(2, Strain::Spades)
                                            && ovc.strain != o_strain
                                            && free.strain != Strain::Notrump
                                            && free.strain != ovc.strain
                                            && free.strain != o_strain
                                            && free.level.get() < 3
                                            && free.level.get()
                                                == ovc.level.get()
                                                    + u8::from(free.strain < ovc.strain)
                                            && !(rotated && free.level.get() == 1)
                                            && !(negative && free.level.get() == 2)
                                            && !(transfer
                                                && free.level.get() == 2
                                                && two_level_slots(o_strain, *ovc) == 2)
                                )
                            }),
                        ),
                    ),
                    answer_free_bid(opening),
                ));

                if !negative {
                    continue;
                }

                // Section 4d′: the capped, non-forcing level-2 frees get
                // answers WITH a Pass catch-all.  The sample must be a level-2
                // free, so it rides an overcall the free bid can out-rank
                // without stepping up a level.
                let negative_sample = match opening {
                    Suit::Clubs => "(2♦) 2♥ -",
                    Suit::Hearts => "(2♣) 2♠ -",
                    _ => "(2♣) 2♥ -",
                };
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        negative_sample,
                        described_guard(
                            "(overcall ≤2♠) negative free-suit -",
                            guard(move |_: &Context<'_>, suffix: &[Call]| {
                                matches!(
                                    suffix,
                                    [Call::Bid(ovc), Call::Bid(free), Call::Pass]
                                        if *ovc <= Bid::new(2, Strain::Spades)
                                            && ovc.strain != o_strain
                                            && free.strain != Strain::Notrump
                                            && free.strain != ovc.strain
                                            && free.strain != o_strain
                                            && free.level.get() == 2
                                            && free.level.get()
                                                == ovc.level.get()
                                                    + u8::from(free.strain < ovc.strain)
                                )
                            }),
                        ),
                    ),
                    answer_negative_free_bid(opening),
                ));

                // Section 4d″: the doubler's rebid over opener's answer — a new
                // suit is the strong hand the capped free bid could not carry,
                // forcing to game.  This node also claims the ordinary
                // doubler's second turn (previously floored — bucket
                // X-then-Pass vs X-then-suit in the forensics).
                let over = if opening == Suit::Spades {
                    "(2♥)"
                } else {
                    "(2♠)"
                };
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        &format!("{over} X - 3♣ -"),
                        described_guard(
                            "(overcall ≤2♠) X - answer -",
                            guard(move |_: &Context<'_>, suffix: &[Call]| {
                                matches!(
                                    suffix,
                                    [
                                        Call::Bid(ovc),
                                        Call::Double,
                                        Call::Pass,
                                        Call::Bid(_),
                                        Call::Pass
                                    ] if *ovc <= Bid::new(2, Strain::Spades)
                                        && ovc.strain != o_strain
                                )
                            }),
                        ),
                    ),
                    negative_doubler_rebid(opening),
                ));

                // Section 4d‴: opener answers the game-forcing rebid with the
                // ordinary forcing-answer table; the guard's `< 3 of the
                // opening suit` scope keeps that table's catch-all legal.
                let fg_sample = match opening {
                    Suit::Clubs => "(1♥) X - 1♠ - 2♦ -",
                    Suit::Diamonds => "(1♥) X - 1♠ - 2♣ -",
                    Suit::Hearts => "(1♠) X - 2♣ - 2♦ -",
                    Suit::Spades => "(1♥) X - 2♣ - 2♦ -",
                };
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        fg_sample,
                        described_guard(
                            "(overcall ≤2♠) X - answer - FG-suit -",
                            guard(move |_: &Context<'_>, suffix: &[Call]| {
                                matches!(
                                    suffix,
                                    [
                                        Call::Bid(ovc),
                                        Call::Double,
                                        Call::Pass,
                                        Call::Bid(ans),
                                        Call::Pass,
                                        Call::Bid(new),
                                        Call::Pass
                                    ] if *ovc <= Bid::new(2, Strain::Spades)
                                        && ovc.strain != o_strain
                                        && new.strain != Strain::Notrump
                                        && new.strain != ovc.strain
                                        && new.strain != o_strain
                                        && new.strain != ans.strain
                                        && *new < Bid::new(3, o_strain)
                                )
                            }),
                        ),
                    ),
                    answer_free_bid(opening),
                ));
            }
            entries
        },
    }
}
