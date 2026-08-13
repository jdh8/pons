//! Integration tests for the **European** 1NT minor scheme
//! ([`notrump_minors`][field@pons::bidding::inference::ReadingProfile::notrump_minors]
//! set to [`EUROPEAN`]): `2♠` = clubs (transfer), `2NT` = a
//! balanced invite / size ask, `3♣` = diamonds (transfer); no Puppet Stayman, so a
//! game-forcing balanced hand with only a three-card major bids 3NT and a 4-3 game
//! force takes Stayman.  Mirrors `american_minor_transfers.rs` (the Puppet default).
//!
//! [`EUROPEAN`]: pons::american::EUROPEAN

mod common;
use common::*;

use contract_bridge::Suit;
use pons::american::EUROPEAN;

/// The American 2/1 partnership with the **European** minor scheme selected
///
/// Each call builds a partnership whose agreements select the European book.
fn partnership() -> Partnership {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.decision.reading.notrump_minors = EUROPEAN;
    american(&agreements).bind()
}

const P: Call = Call::Pass;

/// `1NT -` plus the given tail of our-side calls (RHO passes interleaved)
fn after_1nt(tail: &[Call]) -> Vec<Call> {
    let mut auction = vec![call(1, Strain::Notrump), P];
    for &c in tail {
        auction.push(c);
        auction.push(P);
    }
    auction
}

// --- 2♠ = transfer to clubs -------------------------------------------------

#[test]
fn two_spades_is_a_transfer_to_clubs() {
    let system = partnership();
    // Six clubs, sub-game: the European club transfer, 2♠ (not the natural-spades
    // bid, and not Stayman — there is no four-card major).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "xxx.xxx.x.KQxxxx"),
        call(2, Strain::Spades),
    );
}

#[test]
fn opener_completes_the_club_transfer() {
    let system = partnership();
    let auction = after_1nt(&[call(2, Strain::Spades)]);
    // Opener always completes the transfer to 3♣ (no super-accept).
    assert_eq!(
        best_call(&system, &auction, "AQx.KJx.Kxx.Axxx"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn weak_clubs_pass_the_completion() {
    let system = partnership();
    let auction = after_1nt(&[call(2, Strain::Spades), call(3, Strain::Clubs)]);
    // Weak six-card club one-suiter: pass the club partscore.
    assert_eq!(best_call(&system, &auction, "xxx.xxx.x.KQxxxx"), P);
}

#[test]
fn game_going_clubs_raise_to_3nt() {
    let system = partnership();
    let auction = after_1nt(&[call(2, Strain::Spades), call(3, Strain::Clubs)]);
    // Six clubs, game values: 3NT over the completion — EPBot's whole 8–15 bucket.
    assert_eq!(
        best_call(&system, &auction, "x.Kxx.Kxx.AQxxxx"),
        call(3, Strain::Notrump),
    );
}

/// The club-lane twin of [`no_three_level_splinter_over_the_diamond_completion`].
/// `--mode nt-2s-3c` (9929 hands reaching the node) has **no `3♦`/`3♥`/`3♠`
/// bucket at all** — indeed no three-level call but `3NT`.  These rungs were
/// inherited from Puppet's two-way `2♠` on no evidence and were the *last*
/// unprobed copy-paste in the scheme.
#[test]
fn no_three_level_splinter_over_the_club_completion() {
    let system = partnership();
    let auction = after_1nt(&[call(2, Strain::Spades), call(3, Strain::Clubs)]);
    // Six clubs, game values, a stiff spade — the Puppet twin's splinter hand.
    let got = best_call(&system, &auction, "x.Kxx.Kxx.AQxxxx");
    assert_ne!(got, call(3, Strain::Diamonds));
    assert_ne!(got, call(3, Strain::Hearts));
    assert_ne!(got, call(3, Strain::Spades));
}

// --- 2NT = balanced invitational (size ask) ---------------------------------

#[test]
fn two_nt_is_a_balanced_invite() {
    let system = partnership();
    // Balanced 8, no four-card major, *not* a flat 4-3-3-3 (a 4-4 in the minors):
    // the European size ask, 2NT (the Puppet default would route this hand through
    // the two-way 2♠ instead).  A flat 4-3-3-3 eight would pass 1NT, not invite.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "Kx.Qxx.Jxxx.Qxxx"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn opener_accepts_the_invite_with_a_maximum() {
    let system = partnership();
    let auction = after_1nt(&[call(2, Strain::Notrump)]);
    // Maximum (17): accept game, 3NT.
    assert_eq!(
        best_call(&system, &auction, "AQx.KJx.Kxx.Axxx"),
        call(3, Strain::Notrump),
    );
    // Minimum (15): decline, pass and play 2NT.
    assert_eq!(best_call(&system, &auction, "KQx.KJx.Qxx.Axxx"), P);
}

// --- 3♣ = transfer to diamonds ----------------------------------------------

#[test]
fn three_clubs_is_a_transfer_to_diamonds() {
    let system = partnership();
    // Six diamonds, sub-game: the European diamond transfer, 3♣ (no Puppet Stayman
    // claims 3♣ here).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "xx.xxx.KQxxxx.xx"),
        call(3, Strain::Clubs),
    );
}

/// The fidelity pin the scheme went years without: EPBot's `3♣` bucket is
/// **diamonds 6–7, hard min/max** (`--mode nt-resp --trim 0.0`, n=1042), so a
/// 5♦4♣ two-suiter never transfers.  It used to, on a class pasted over from
/// Puppet's `2NT` — see docs/ai-bidder/bba-1nt-minors.md.
#[test]
fn five_diamond_four_club_two_suiter_does_not_transfer() {
    let system = partnership();
    assert_ne!(
        best_call(&system, &after_1nt(&[]), "xx.xx.KQxxx.Qxxx"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn opener_completes_the_diamond_transfer() {
    let system = partnership();
    let auction = after_1nt(&[call(3, Strain::Clubs)]);
    // Opener always completes the diamond transfer to 3♦.
    assert_eq!(
        best_call(&system, &auction, "AQx.KJx.Kxx.Axxx"),
        call(3, Strain::Diamonds),
    );
}

#[test]
fn weak_diamonds_pass_the_completion() {
    let system = partnership();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Diamonds)]);
    // Six diamonds, sub-game: pass the 3♦ partscore.
    assert_eq!(best_call(&system, &auction, "xx.xxx.KQxxxx.xx"), P);
}

#[test]
fn game_going_diamonds_raise_to_3nt() {
    let system = partnership();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Diamonds)]);
    // Six diamonds, game values: bid 3NT over the completion.
    assert_eq!(
        best_call(&system, &auction, "xx.Axx.KQJxxx.xx"),
        call(3, Strain::Notrump),
    );
}

/// The Puppet lane splinters `3♥`/`3♠` after its own diamond transfer; European
/// must not copy it.  `--mode nt-3c-3d` (10260 hands reaching the node) has **no
/// `3♥`/`3♠` bucket at all** — EPBot shows shortness only as a void, and only at
/// `4♠`/`5♥`/`5♣`.  Pinned so a future mirror of the Puppet twin has to argue
/// with the probe first.
#[test]
fn no_three_level_splinter_over_the_diamond_completion() {
    let system = partnership();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Diamonds)]);
    // Six diamonds, game values, a stiff spade — the Puppet twin's splinter hand.
    let got = best_call(&system, &auction, "x.Axx.KQJxxx.xxx");
    assert_ne!(got, call(3, Strain::Spades));
    assert_ne!(got, call(3, Strain::Hearts));
}

// --- No Puppet: the GF balanced / 4-3 hands route elsewhere ------------------

#[test]
fn game_force_three_card_major_bids_3nt() {
    let system = partnership();
    // 3-3 majors, balanced 11: the hand Puppet routes through 3♣ has no home in
    // the European scheme (3♣ is diamonds) — it simply bids 3NT.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "K32.Q43.KJ4.Q932"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn game_force_four_three_takes_stayman() {
    let system = partnership();
    // 4♠-3♥-4♦-2♣ game force (non-flat): with no Puppet, the 4-3 hand uses plain
    // Stayman (2♣).  A flat 4-3-3-3 would instead bid 3NT (see below).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.Q32.K432.Q9"),
        call(2, Strain::Clubs),
    );
}

// --- Reading a European *opponent* ------------------------------------------

/// Our own (Puppet) partnership, told the opponents play European
///
/// `Partnership::with_opponents` is the reading half of a declared opponent: our
/// own calls keep resolving in our books, theirs decode in this one.  Until this
/// existed on the European axis, every setter of `notrump_minors` was our-side,
/// so the scheme — an *opponent model* — had no coverage on the one path it is
/// for.
fn vs_european() -> Partnership {
    partnership_us().with_opponents(&partnership())
}

/// Our shipped Puppet partnership, as the reader
fn partnership_us() -> Partnership {
    american(&pons::bidding::agreements::Agreements::default()).bind()
}

/// `(1NT) - (2♠) -` with us to act fourth: the opponents' auction, read from our seat
fn their_auction(responder: Call) -> Vec<Call> {
    vec![call(1, Strain::Notrump), P, responder]
}

#[test]
fn a_declared_european_opponent_shows_clubs_on_two_spades() {
    use pons::bidding::inference::Relative;

    let auction = their_auction(call(2, Strain::Spades));
    let vul = RelativeVulnerability::NONE;
    // Declared European: `2♠` is the club transfer — six of them, guaranteed.
    assert!(
        vs_european()
            .infer(vul, &auction)
            .get(Relative::Rho)
            .length(Suit::Clubs)
            .min
            >= 6,
        "a declared European opponent's 2♠ must read as six-plus clubs",
    );
    // Undeclared, we model them as playing our Puppet two-way `2♠`, which the
    // balanced invite also makes — so no six-card club promise.
    assert!(
        partnership_us()
            .infer(vul, &auction)
            .get(Relative::Rho)
            .length(Suit::Clubs)
            .min
            < 6,
        "our own two-way 2♠ promises no such thing; the declaration is what moves it",
    );
}

#[test]
fn a_declared_european_opponent_shows_diamonds_on_three_clubs() {
    use pons::bidding::inference::Relative;

    let auction = their_auction(call(3, Strain::Clubs));
    let vul = RelativeVulnerability::NONE;
    // Declared European: `3♣` is the diamond transfer, a six-card one-suiter.
    assert!(
        vs_european()
            .infer(vul, &auction)
            .get(Relative::Rho)
            .length(Suit::Diamonds)
            .min
            >= 6,
        "a declared European opponent's 3♣ must read as six-plus diamonds",
    );
    // Ours is Puppet Stayman there — artificial, and no diamond claim at all.
    assert!(
        partnership_us()
            .infer(vul, &auction)
            .get(Relative::Rho)
            .length(Suit::Diamonds)
            .min
            < 6,
        "our own 3♣ is Puppet Stayman; the declaration is what moves it",
    );
}

/// The `read.rs` site that reads responder's `2NT` off the minor scheme is gated
/// by `is_opening_side` — parity relative to the *opener*, not to us — so before
/// `side_profile` it answered the opponents' auction out of our knob.
#[test]
fn a_declared_european_opponent_reads_two_notrump_as_the_size_ask() {
    use pons::bidding::inference::Relative;

    let auction = their_auction(call(2, Strain::Notrump));
    let vul = RelativeVulnerability::NONE;
    // European's `2NT` is a balanced invite; it says nothing about diamonds.
    assert!(
        vs_european()
            .infer(vul, &auction)
            .get(Relative::Rho)
            .length(Suit::Diamonds)
            .min
            < 5,
        "a declared European opponent's 2NT is the size ask, not a diamond transfer",
    );
    // Ours is the Puppet diamond transfer: five-plus diamonds.
    assert!(
        partnership_us()
            .infer(vul, &auction)
            .get(Relative::Rho)
            .length(Suit::Diamonds)
            .min
            >= 5,
        "our own 2NT is the diamond transfer; the declaration is what moves it",
    );
}

#[test]
fn flat_four_three_three_three_game_force_bids_3nt() {
    let system = partnership();
    // Flat 4-3-3-3 (four spades) game force: no Stayman with a flat hand — it plays
    // 3NT, not the 4-4 fit (European has no Puppet either, so it simply bids 3NT).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.Q32.K43.Q92"),
        call(3, Strain::Notrump),
    );
}
