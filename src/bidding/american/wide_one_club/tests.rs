use crate::bidding::Bidder;
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

const P: Call = Call::Pass;

/// The wide 1♣ on the 5542 host the plugin is written for
fn watermelon() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.opening.wide_one_club = true;
    agreements.opening.five_five_four_two = true;
    agreements
}

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// Our call after an undisturbed `auction`, under `agreements`
fn calls(agreements: &Agreements, auction: &[Call], hand: &str) -> Call {
    let partnership = super::super::american(agreements).bind();
    let hand = hand.parse().unwrap();
    let logits = partnership
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(call, _)| call)
        .unwrap()
}

fn responds(auction: &[Call], hand: &str) -> Call {
    calls(&watermelon(), auction, hand)
}

/// The overlay preserves the declarative row invariants (with the knob on;
/// off, the package is empty by its gate).
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(&watermelon(), &[super::package()]);
}

/// Both knobs off is the shipped opening table, byte for byte.
#[test]
fn knobs_off_is_american() {
    let shipped = super::super::openings::openings(&Agreements::default());
    let mut off = watermelon();
    off.opening.wide_one_club = false;
    off.opening.five_five_four_two = false;
    assert_eq!(
        format!("{:?}", super::super::openings::openings(&off).rules()),
        format!("{:?}", shipped.rules())
    );
}

/// The 5542 partition alone: `1♦` is four-plus, `1♣` two-plus, (xx)45 canapés.
#[test]
fn five_five_four_two_partition() {
    let mut a = Agreements::default();
    a.opening.five_five_four_two = true;
    // 4=4=3=2 opens a doubleton 1♣ (better minor opens 1♦).
    assert_eq!(calls(&a, &[], "KQ32.KJ32.K32.32"), bid(1, Strain::Clubs));
    assert_eq!(
        calls(&Agreements::default(), &[], "KQ32.KJ32.K32.32"),
        bid(1, Strain::Diamonds)
    );
    // (xx)45 opens 1♦ — the canapé.
    assert_eq!(
        calls(&a, &[], "K32.A2.KJ32.Q9832"),
        bid(1, Strain::Diamonds)
    );
    // 4-4 minors open 1♦ under both.
    assert_eq!(calls(&a, &[], "K32.A2.KJ32.Q983"), bid(1, Strain::Diamonds));
    // 3-3 minors open 1♣ under both.
    assert_eq!(calls(&a, &[], "KQ32.KJ32.K32.Q32"), bid(1, Strain::Clubs));
}

/// The wide-1♣ opening partition: the load-bearing cases.
#[test]
fn wide_opening_partition() {
    let opens = |hand| calls(&watermelon(), &[], hand);
    // The wide 1♣ hosts a strong balanced 23-count (american opens it 2♣).
    assert_eq!(opens("AKQ2.KQ3.KQ3.A32"), bid(1, Strain::Clubs));
    assert_eq!(
        calls(&Agreements::default(), &[], "AKQ2.KQ3.KQ3.A32"),
        bid(2, Strain::Clubs)
    );
    // A 22-count with four diamonds opens the wide 1♦.
    assert_eq!(opens("AKQ2.KQ3.KQ32.A3"), bid(1, Strain::Diamonds));
    // 21–23 with a five-card major is the strong, artificial 2♣.
    assert_eq!(opens("AKQ32.AK3.AQ2.32"), bid(2, Strain::Clubs));
    // 21–23 with a six-card minor is the strong 2♣ too.
    assert_eq!(opens("AK.AK3.Q2.KQJ432"), bid(2, Strain::Clubs));
    // 24+ anything is 2♣.
    assert_eq!(opens("AKQ2.AKQ3.KQ2.A3"), bid(2, Strain::Clubs));
    // A balanced 16 still opens 1NT; a 21 balanced still opens 2NT — and a
    // 22 balanced, past 2NT's fifths ceiling, is the wide 1♣'s.
    assert_eq!(opens("AQ32.K53.QJ4.A92"), bid(1, Strain::Notrump));
    assert_eq!(opens("AKQ2.KQ3.KQ3.Q32"), bid(2, Strain::Notrump));
    assert_eq!(opens("AKQ2.KQ3.KQ3.K32"), bid(1, Strain::Clubs));
    // A five-card major at 20 is still 1♠; at 21 it is the strong 2♣.
    assert_eq!(opens("AKQJ2.AKJ.Q432.2"), bid(1, Strain::Spades));
    assert_eq!(opens("AKQJ2.AKJ.K432.2"), bid(2, Strain::Clubs));
}

/// Responder's first call over the wide 1♣.
#[test]
fn wide_one_club_responses() {
    let one_club = [bid(1, Strain::Clubs), P];
    // Weak, club-tolerant (3 HCP, 4♣): content to play 1♣.
    assert_eq!(responds(&one_club, "xxx.xxx.xxx.Kxxx"), P);
    // Weak and short in clubs: relay out.
    assert_eq!(
        responds(&one_club, "Kxxx.Qxx.xxxx.xx"),
        bid(1, Strain::Diamonds)
    );
    // 8 HCP, no four-card major: the host's 1NT rung takes it before the relay.
    assert_eq!(
        responds(&one_club, "Kxx.Qxx.Kxxxx.xx"),
        bid(1, Strain::Notrump)
    );
    // 8 HCP with a four-card major: the natural major, as ever.
    assert_eq!(
        responds(&one_club, "Kxxx.Qxx.xxx.Kxx"),
        bid(1, Strain::Spades)
    );
    // 16 HCP balanced, no four-card major: past 3NT's cap — the relay.
    assert_eq!(
        responds(&one_club, "AQx.KJx.KQxx.Jxx"),
        bid(1, Strain::Diamonds)
    );
    // 14 balanced, no major: 3NT to play.
    assert_eq!(
        responds(&one_club, "AQx.KJx.Qxxx.Jxx"),
        bid(3, Strain::Notrump)
    );
    // 16 HCP, 5+♦, no four-card major: natural game force.
    assert_eq!(
        responds(&one_club, "Axx.Kx.AQxxx.Kxx"),
        bid(2, Strain::Diamonds)
    );
    // 11 HCP, 5+♣, no major: natural invite+.
    assert_eq!(
        responds(&one_club, "Axx.Kx.xxx.KJxxx"),
        bid(2, Strain::Clubs)
    );
    // Four-card majors still go up the line.
    assert_eq!(
        responds(&one_club, "KQxx.Axx.xxx.xxx"),
        bid(1, Strain::Spades)
    );
}

/// Opener's rebid after the relay — the book's seven rows.
#[test]
fn opener_after_relay() {
    let relay = [bid(1, Strain::Clubs), P, bid(1, Strain::Diamonds), P];
    // 19 HCP balanced: the 18–20 notrump rebid.
    assert_eq!(
        responds(&relay, "AQx.KJx.KQx.Axxx"),
        bid(1, Strain::Notrump)
    );
    // 18 HCP, four hearts and five clubs: 2♥.
    assert_eq!(responds(&relay, "Ax.KJxx.Kx.AKxxx"), bid(2, Strain::Hearts));
    // 18 HCP, six clubs: 2NT!.
    assert_eq!(
        responds(&relay, "Ax.KJx.Kx.AKxxxx"),
        bid(2, Strain::Notrump)
    );
    // 15 HCP, six clubs: 3♣.
    assert_eq!(responds(&relay, "Ax.Kxx.xx.AKJxxx"), bid(3, Strain::Clubs));
    // 13 HCP, five clubs and four hearts: clubs first (partner has no four-card major).
    assert_eq!(responds(&relay, "xx.KJxx.Kx.AQxxx"), bid(2, Strain::Clubs));
    // 13 HCP balanced 4=3=3=3: the four-card major.
    assert_eq!(responds(&relay, "KQxx.Kxx.Qxx.Axx"), bid(1, Strain::Spades));
    // 13 HCP balanced 3=3=3=4: the three-card major, up the line.
    assert_eq!(responds(&relay, "KQx.Kxx.Qxx.Axxx"), bid(1, Strain::Hearts));
    // 23 HCP, no 5-card major / 6-card minor: the artificial 2♦.
    assert_eq!(
        responds(&relay, "AKQ.Kx.AQxx.AJxx"),
        bid(2, Strain::Diamonds)
    );
}

/// Responder's second call after opener's minimum relay rebid.
#[test]
fn after_relay_continuations() {
    let c = bid(1, Strain::Clubs);
    let d = bid(1, Strain::Diamonds);
    // After `1♣ - 1♦ - 1♥`: both minors (5-4), 10 pts: 2♠ (the other major, repurposed).
    let after_1h = [c, P, d, P, bid(1, Strain::Hearts), P];
    assert_eq!(
        responds(&after_1h, "x.xx.KQxxx.AJxx"),
        bid(2, Strain::Spades)
    );
    // 16 balanced: 2NT, rightsiding the notrump.
    assert_eq!(
        responds(&after_1h, "AQx.Kxx.KJxx.Kxx"),
        bid(2, Strain::Notrump)
    );
    // Weak balanced: 1NT.
    assert_eq!(
        responds(&after_1h, "Qxx.xxx.Kxxx.Jxx"),
        bid(1, Strain::Notrump)
    );
    // After `1♣ - 1♦ - 1♠`: both minors is 2♥.
    let after_1s = [c, P, d, P, bid(1, Strain::Spades), P];
    assert_eq!(
        responds(&after_1s, "x.xx.KQxxx.AJxx"),
        bid(2, Strain::Hearts)
    );
    // After `1♣ - 1♦ - 2♣`: an invitational club raise is 2♠!, a minimum one 3♣.
    let after_2c = [c, P, d, P, bid(2, Strain::Clubs), P];
    assert_eq!(
        responds(&after_2c, "Qxx.x.Qxxx.AQxx"),
        bid(2, Strain::Spades)
    );
    assert_eq!(
        responds(&after_2c, "xxx.xx.Qxxx.KQxx"),
        bid(3, Strain::Clubs)
    );
}

/// Opener's rebid after responder's game-forcing 2♦.
#[test]
fn opener_after_two_diamonds() {
    let a = [bid(1, Strain::Clubs), P, bid(2, Strain::Diamonds), P];
    assert_eq!(responds(&a, "Axx.Kx.KJxx.Qxx"), bid(3, Strain::Diamonds));
    assert_eq!(responds(&a, "Ax.Kx.xxx.AQxxx"), bid(3, Strain::Clubs));
    assert_eq!(responds(&a, "AQx.KQx.Qxx.Kxxx"), bid(3, Strain::Notrump));
    assert_eq!(responds(&a, "xxx.AQx.Kxx.Kxxx"), bid(2, Strain::Hearts));
    assert_eq!(responds(&a, "AQx.xxx.Kxx.Kxxx"), bid(2, Strain::Spades));
    assert_eq!(responds(&a, "xxx.xxx.KQx.AQxx"), bid(2, Strain::Notrump));
}

/// Opener's rebid after responder's invitational-or-better 2♣.
#[test]
fn opener_after_two_clubs() {
    let a = [bid(1, Strain::Clubs), P, bid(2, Strain::Clubs), P];
    assert_eq!(responds(&a, "AQx.KQx.Qxx.Kxxx"), bid(3, Strain::Notrump));
    assert_eq!(responds(&a, "Axx.xxx.AKx.AKxx"), bid(3, Strain::Notrump));
    assert_eq!(responds(&a, "AQx.KQx.Qxxx.xx"), bid(2, Strain::Notrump));
    assert_eq!(responds(&a, "AQx.Kxx.xx.KJxx"), bid(3, Strain::Clubs));
}

/// Responder honours the force and caps at the right game.
#[test]
fn responder_continues_after_opener_rebid() {
    let c = bid(1, Strain::Clubs);
    let d2 = bid(2, Strain::Diamonds);
    let c2 = bid(2, Strain::Clubs);
    let gf = "AQx.Kx.KQxxx.xx";
    for rebid in [
        bid(3, Strain::Diamonds),
        bid(2, Strain::Hearts),
        bid(2, Strain::Notrump),
    ] {
        let auction = [c, P, d2, P, rebid, P];
        assert_eq!(responds(&auction, gf), bid(3, Strain::Notrump));
    }
    let gf_3nt = [c, P, d2, P, bid(3, Strain::Notrump), P];
    assert_eq!(responds(&gf_3nt, gf), P);
    let inv_3c = [c, P, c2, P, bid(3, Strain::Clubs), P];
    assert_eq!(
        responds(&inv_3c, "AQx.Kx.Kx.KQxxx"),
        bid(3, Strain::Notrump)
    );
    assert_eq!(responds(&inv_3c, "Jxx.Qx.Qx.KQxxx"), P);
    let inv_3nt = [c, P, c2, P, bid(3, Strain::Notrump), P];
    assert_eq!(responds(&inv_3nt, "Jxx.Qx.Qx.KQxxx"), P);
}
