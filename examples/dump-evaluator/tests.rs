use super::*;

/// Ben-arm column offsets within a suit's [`LEN_SUIT_BITS`] block: `#spots`
/// and the five honour flags, i.e. everything but `len` and `suit_hcp`.
const BEN_SUIT: [usize; 6] = [1, 3, 4, 5, 6, 7];

/// Ben-arm offsets within a `(min, max, width)` range triple: the width the
/// `bits` encoding added is dropped again.
const BEN_TRIPLE: [usize; 2] = [0, 1];

/// The shared fixture: a void, an honourless suit, and two mixed holdings,
/// so `#spots` is exercised as "length minus honours held" and not as a
/// constant, under an auction that actually shows something.
fn fixture() -> (Hand, Inferences) {
    let hand: Hand = "AT2.KQ98.J76543.".parse().expect("valid test hand");
    let auction: Vec<Call> = ["1S", "P", "2H"]
        .iter()
        .map(|c| c.parse().expect("valid test call"))
        .collect();
    let stance = american(&pons::bidding::agreements::Agreements::default()).against();
    let vul = relative(AbsoluteVulnerability::NONE, Seat::North);
    (hand, stance.infer(vul, &auction))
}

/// The `bits` columns the trainer's `--arm ben` leaves live, re-derived from
/// its offset table rather than transcribed as a 54-element literal.
fn ben_live_columns() -> Vec<usize> {
    let mut cols = Vec::new();
    for suit in 0..4 {
        cols.extend(BEN_SUIT.map(|o| suit * LEN_SUIT_BITS + o));
    }
    // Columns 32 (`hcp/40`) and 33 (`upgrade/2`) are the globals the arm
    // drops, so the range triples follow immediately.
    for triple in 0..LEN_RANGES / 2 {
        cols.extend(BEN_TRIPLE.map(|o| LEN_HAND_BITS + 3 * triple + o));
    }
    cols
}

#[test]
fn bits_row_is_self_consistent() {
    let (hand, inferences) = fixture();

    let mut row = vec![0f32; LEN_HAND_BITS + LEN_RANGES / 2 * 3];
    assert_eq!(row.len(), 79);
    encode(&mut row, hand, &inferences, &[], Encoding::Bits);

    let (hand_block, triples) = row.split_at(LEN_HAND_BITS);
    for block in hand_block[..4 * LEN_SUIT_BITS]
        .as_chunks::<LEN_SUIT_BITS>()
        .0
    {
        let (spots, honours) = (block[1] * 8.0, &block[3..]);
        // Span identity: len = #spots + A + K + Q + J + T.  Both sides
        // divide by 13 rather than multiplying out, so the compare is exact.
        assert_eq!(block[0], (spots + honours.iter().sum::<f32>()) / 13.0);
        // Suit HCP is exactly what the honour bits say: 4A + 3K + 2Q + J.
        let hcp = 4.0 * honours[0] + 3.0 * honours[1] + 2.0 * honours[2] + honours[3];
        assert_eq!(block[2], hcp / 10.0);
    }

    // Every range pair survives verbatim and gains its width beside it.
    let feats = features_eval(hand, &inferences);
    assert_eq!(triples.len(), 45);
    for (triple, pair) in triples
        .as_chunks::<3>()
        .0
        .iter()
        .zip(feats[LEN_HAND_EVAL..].as_chunks::<2>().0)
    {
        assert_eq!(triple[..2], *pair);
        assert_eq!(triple[2], triple[1] - triple[0]);
    }
}

/// `features_eval` is now exactly the `ben` arm of the `bits` superset: the
/// same 24 honour columns and the same 30 range bounds, in the same order.
/// Nothing else checks that coupling, and it is silent when it breaks — the
/// trainer would fit a net on one column order while the crate serves it
/// another, permuted, with no width mismatch to trip over.  So gather the
/// `bits` row at the arm's live columns and demand the `summary` row back.
///
/// Exact float equality is right here: both sides are copies of the very
/// same computed floats, not two roundings of one quantity.
/// The oracle counts every ace as a keycard, adds the trump king per strain,
/// and flags the trump queen — in `Suit::ASC` (♣♦♥♠) order, keycards then
/// queens. `AKQJ.AK2.Q32.432` has two aces (♠♥), so ♥/♠ read 3 keycards
/// (own trump king) and ♣/♦ read 2, with the queen only under ♦ and ♠.
#[test]
fn oracle_counts_partner_keycards_and_trump_queen() {
    let partner: Hand = "AKQJ.AK2.Q32.432".parse().expect("valid test hand");
    let mut out = [0f32; ORACLE_LEN];
    write_oracle(&mut out, partner);
    assert_eq!(out, [0.4, 0.4, 0.6, 0.6, 0.0, 1.0, 0.0, 1.0]);
}

/// The survey oracle, pinned cell by cell: keycard head verbatim (same
/// fixture hand as [`oracle_counts_partner_keycards_and_trump_queen`],
/// placed at South's partner), then each axis block for the hidden seats
/// [LHO, partner, RHO] = [West, North, East] in ♣♦♥♠ order.
#[test]
fn oracle_all_layout_is_axis_major() {
    let deal: FullDeal = "W:T9876.QJT9.J54.A AKQJ.AK2.Q32.432 5432.87.AKT9.KQJ \
                          .6543.876.T98765"
        .parse()
        .expect("valid test deal");
    let mut out = [0f32; ORACLE_ALL_LEN];
    write_oracle_all(&mut out, &deal, Seat::South);

    // Keycard head: partner (North) holds AKQJ.AK2.Q32.432.
    assert_eq!(out[..8], [0.4, 0.4, 0.6, 0.6, 0.0, 1.0, 0.0, 1.0]);
    // Quality: per-suit HCP / 10.
    let quality = [
        [0.4, 0.1, 0.3, 0.0], // West: ♣A, ♦J54, ♥QJT9, ♠T9876
        [0.0, 0.2, 0.7, 1.0], // North: ♣432, ♦Q32, ♥AK2, ♠AKQJ
        [0.6, 0.7, 0.0, 0.0], // East: ♣KQJ, ♦AKT9, ♥87, ♠5432
    ];
    assert_eq!(out[8..20], quality.concat()[..]);
    // Shortness: only West's singleton ♣A qualifies.
    let mut shortness = [0.0; 12];
    shortness[0] = 1.0;
    assert_eq!(out[20..32], shortness);
    // Controls: (ace, king) per suit.
    let controls = [
        [1., 0., 0., 0., 0., 0., 0., 0.], // West: ♣A only
        [0., 0., 0., 0., 1., 1., 1., 1.], // North: ♥AK, ♠AKQJ
        [0., 1., 1., 1., 0., 0., 0., 0.], // East: ♣KQJ, ♦AKT9
    ];
    assert_eq!(out[32..56], controls.concat()[..]);
    // Stoppers: A / Kx / Qxx / Jxxx — West's ♦J54 is a J with only three.
    let stoppers = [
        [1.0, 0.0, 1.0, 0.0], // West: ♣A, ♥QJT9
        [0.0, 1.0, 1.0, 1.0], // North: ♦Qxx, ♥AK2, ♠AKQJ
        [1.0, 1.0, 0.0, 0.0], // East: ♣KQJ, ♦AKT9
    ];
    assert_eq!(out[56..68], stoppers.concat()[..]);
}

#[test]
fn summary_is_the_ben_gather_of_bits() {
    let (hand, inferences) = fixture();

    let mut summary = vec![0f32; FEATURES_LEN_EVAL];
    encode(&mut summary, hand, &inferences, &[], Encoding::Summary);
    let mut bits = vec![0f32; LEN_HAND_BITS + LEN_RANGES / 2 * 3];
    encode(&mut bits, hand, &inferences, &[], Encoding::Bits);

    let live = ben_live_columns();
    assert_eq!(live.len(), 54, "the ben arm's documented live width");
    assert_eq!(live.len(), summary.len());
    for (i, (&col, &want)) in live.iter().zip(&summary).enumerate() {
        assert_eq!(bits[col], want, "summary[{i}] should be bits[{col}]");
    }
}

/// The `--auction` block, pinned on `1NT - 2♣` (our Stayman): the
/// most-recent slot carries the 2♣ bid encoding, the `STAY` tag bit, the
/// alerted bit, and exactly one hash bucket; the Pass slot is `is_pass`
/// with no bid present; slots past the auction's start stay all-zero.
#[test]
fn auction_block_encodes_stayman() {
    let auction: Vec<Call> = ["1N", "P", "2C"]
        .iter()
        .map(|c| c.parse().expect("valid test call"))
        .collect();
    // Invitational with both four-card majors — a live Stayman hand, so
    // the alerted 2♣ rule gives it a finite logit and wins attribution.
    let hand: Hand = "AQ32.KJ54.876.54".parse().expect("valid test hand");
    let stance = american(&pons::bidding::agreements::Agreements::default()).against();
    let vul = relative(AbsoluteVulnerability::NONE, Seat::South);
    let alert = stance
        .explain_call(hand, vul, &auction[..2], auction[2])
        .and_then(|(_, rule)| rule)
        .and_then(|rule| rule.alert);
    assert_eq!(alert, Some("stayman"), "the Stayman rule's alert");

    let alerts = vec![None, None, alert];
    let mut block = [0f32; AUCTION_LEN];
    write_auction_block(&mut block, &auction, &alerts);

    // Slot 0, most recent = 2♣: present, level 2, clubs first in ASC.
    let (recent, rest) = block.split_at(LEN_CALL_SLOT);
    assert_eq!(recent[..7], [1.0, 2.0 / 7.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(recent[7..10], [0.0; 3]);
    let stay = TAGS.iter().position(|&t| t == "STAY").expect("STAY tag");
    let tags: Vec<usize> = (0..TAGS.len()).filter(|&i| recent[10 + i] != 0.0).collect();
    assert_eq!(tags, [stay], "exactly the STAY tag bit");
    assert_eq!(recent[10 + TAGS.len()], 1.0, "alerted bit");
    let buckets = &recent[11 + TAGS.len()..];
    assert_eq!(buckets.len(), 8);
    assert!(buckets.iter().all(|&b| b == 0.0 || b == 1.0));
    assert_eq!(buckets.iter().sum::<f32>(), 1.0, "exactly one hash bucket");

    // Slot 1 = the Pass: no bid present, only the is_pass call-kind bit.
    let pass = &rest[..LEN_CALL_SLOT];
    assert_eq!(pass[..8], [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
    assert_eq!(pass[8..10], [0.0; 2]);

    // Slot 3 is beyond the 3-call auction: all-zero, unlike a real Pass.
    assert!(block[3 * LEN_CALL_SLOT..].iter().all(|&x| x == 0.0));
}
