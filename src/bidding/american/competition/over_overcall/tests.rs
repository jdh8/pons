use super::super::tests::call;
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};

/// Every overcall the retired `(≤2♠)` guard admitted over `1opening`:
/// the suit bids above the opening through 2♠, plus their 1NT.
fn admitted_overcalls(opening: Strain) -> Vec<Bid> {
    let open = Bid::new(1, opening);
    let mut bids: Vec<Bid> = (1..=2u8)
        .flat_map(|level| {
            [
                Strain::Clubs,
                Strain::Diamonds,
                Strain::Hearts,
                Strain::Spades,
            ]
            .into_iter()
            .map(move |strain| Bid::new(level, strain))
        })
        .filter(|&bid| bid > open)
        .collect();
    bids.push(Bid::new(1, Strain::Notrump));
    bids
}

/// The per-overcall exact tables evaluate identically to the retired
/// guarded table across the knob grid: the same logits for every hand,
/// in every column the guard admitted.
#[test]
fn per_overcall_tables_match_legacy() {
    use super::super::free_bids::FreeBidStyle;
    use super::super::negative_double::NegativeDoubleShape;
    use super::over_their_overcall;
    use super::over_their_overcall_legacy;
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::Suit;

    let hands: Vec<Hand> = [
        "QJ9862.43.752.83", // weak six spades
        "83.QJ9862.752.43", // weak six hearts
        "83.43.QJ9862.752", // weak six diamonds
        "83.43.752.QJ9862", // weak six clubs
        "KQ72.QJ84.652.83", // both majors, 8 HCP
        "KJ72.QT84.652.83", // both majors, 6 HCP
        "K53.Q42.J932.T87", // flat 7
        "K5.Q4.J9532.KT87", // five diamonds, 10
        "K5.AQJ96.532.T87", // five hearts, 10
        "KJ8.QT7.AJ94.986", // balanced 11, wide stoppers
        "AKQ2.KQ5.AQJ4.92", // 21 balanced
        "AQ2.K53.QJ42.T92", // 12 flat
        "2.98653.QJ742.92", // weak two-suiter
        "KQ842.75.652.J83", // five spades, 6
        "KQ84.752.652.J83", // four spades, 6
        "AQJ83.K4.KT7.J93", // five spades, 14
    ]
    .iter()
    .map(|hand| hand.parse().expect("valid probe hand"))
    .collect();

    for shape in [
        NegativeDoubleShape::BothMajors,
        NegativeDoubleShape::Modern,
        NegativeDoubleShape::Cachalot,
        NegativeDoubleShape::Sputnik,
    ] {
        for style in [
            FreeBidStyle::Forcing,
            FreeBidStyle::Negative,
            FreeBidStyle::Transfer,
        ] {
            for engaged in [false, true] {
                for quality in [false, true] {
                    let mut agreements = Agreements::current();
                    agreements.competition.negative_double_shape = shape;
                    agreements.competition.free_bid_style = style;
                    agreements.competition.free_bids = engaged;
                    agreements.competition.free_bid_quality = quality;
                    for opening in Suit::ASC {
                        let legacy = over_their_overcall_legacy(opening, &agreements);
                        for overcall in admitted_overcalls(Strain::from(opening)) {
                            let table = over_their_overcall(opening, overcall, &agreements);
                            let auction = [call(1, Strain::from(opening)), Call::Bid(overcall)];
                            for vul in [RelativeVulnerability::NONE, RelativeVulnerability::ALL] {
                                let context = Context::new(vul, &auction);
                                for &hand in &hands {
                                    assert_eq!(
                                        table.classify(hand, &context),
                                        legacy.classify(hand, &context),
                                        "shape {shape:?}, style {style:?}, free bids \
                                         {engaged}, quality {quality}: 1{opening} \
                                         ({overcall}), {hand}",
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
