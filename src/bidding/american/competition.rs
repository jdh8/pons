//! The competitive package over our openings
//!
//! This module builds the [`Competitive`] book that covers contested auctions
//! after our one-level openings.  It is the **index**: the per-call [`Alert`]
//! constants that the whole book shares, and [`competition()`], which folds
//! each agreement's package into the book.  Every agreement lives in its own
//! submodule:
//!
//! | Module | Agreement |
//! | --- | --- |
//! | [`over_overcall`] | responder's direct-seat action over their overcall |
//! | [`penalty_double`] | and responder's `X`/`Pass` options within it |
//! | [`free_bids`], [`negative_double`], [`cue_raise`] | and opener's answer to each |
//! | [`support_double`] | opener's three-card-support `X`/`XX` |
//! | [`over_their_double`] | Jordan/Truscott, and our doubled splinter |
//! | [`high_overcall`] | their jump and three-level overcalls |
//! | [`nt_high_overcall`] | their three-level overcall of our `1NT` |
//! | [`two_suiters`] | Michaels / unusual `2NT` over our `1M` |
//! | [`our_preempts`] | our contested weak twos and strong `2♣` |
//! | [`lebensohl`], [`rubensohl`], [`uvu`] | over their overcall of our `1NT` |
//! | [`over_our_stayman`] | when they compete over our `2♣` Stayman |
//! | [`over_our_jacoby`] | when they compete over our Jacoby transfer |
//! | [`over_our_minor_transfer`] | when they compete over our two-way `2♠` minor response |
//! | [`over_our_diamond_transfer`] | when they compete over our `2NT` diamond transfer |

use super::super::agreements::Agreements;
use super::super::constraint::{
    Cons, Constraint, at_least_as_long, balanced, described, has_stopper, hcp, len, longer_suit,
    longest_unbid, min_level_is, partner_suit_is, points, stopper_in, stopper_in_their_suits,
    suit_hcp, support, they_bid, top_honors, vulnerable,
};
use super::super::context::Context;
use super::super::fallback::{ReplaceNext, described_guard, described_rewrite, guard, rewriter};
use super::super::rows::{
    Bindings, Entry, Package, Pattern, classified, compile_into, expand, rebase, row, rows_of,
};
use super::super::trie::{Classifier, classifier};
use super::super::{Alert, Competitive, Rules};
use super::notrump::{
    PUPPET, TEXAS, complete_texas, complete_transfer, direct_4m_max, notrump_responses,
    slam_try_answer, smolen_at_three, smolen_completion, stayman_answers, texas_slam_drive_rebid,
};
use super::weak_twos;
use super::{COMPLETION, call};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

mod cue_raise;
mod free_bids;
mod high_overcall;
mod lebensohl;
mod negative_double;
mod nt_high_overcall;
mod our_preempts;
mod over_our_diamond_transfer;
mod over_our_jacoby;
mod over_our_minor_transfer;
mod over_our_stayman;
mod over_overcall;
mod over_their_double;
mod penalty_double;
mod rubensohl;
mod support_double;
mod two_suiters;
mod uvu;

use cue_raise::{cue_minor_raise_answer_package, cue_raise_answer_package};
use free_bids::{free_bid_answer_package, transfer_free_bid_package};
use high_overcall::high_overcall_package;
use lebensohl::lebensohl_package;
use negative_double::{
    answer_negative_double_package, cachalot_package, sputnik_residual_answer_package,
};
use nt_high_overcall::nt_high_overcall_package;
use our_preempts::{strong_two_competition_package, weak_two_competition_package};
use over_our_diamond_transfer::competition_over_diamond_transfer_package;
use over_our_jacoby::competition_over_transfer_package;
use over_our_minor_transfer::competition_over_minor_transfer_package;
use over_our_stayman::competition_over_stayman_package;
use over_overcall::direct_seat_package;
use over_their_double::{jordan_truscott_package, splinter_doubled_package};
use support_double::support_double_package;
use two_suiters::{uvu_over_majors_package, uvu_over_minors_package};
use uvu::uvu_package;

pub use free_bids::FreeBidStyle;
pub use lebensohl::LebensohlStyle;
pub(super) use lebensohl::{complete_lebensohl_relay, lebensohl_relay_rebid, lebensohl_responder};
pub use negative_double::NegativeDoubleShape;
pub use penalty_double::DoubleStyle;
pub use rubensohl::{Competitive4333, MultiStopperAsk};
pub(super) use rubensohl::{
    clubs_transfer_completion, cue_stayman_answer, cue_stayman_answer_no_stopper,
    lm_2d_both_majors_advance, lm_2d_clubs_ask, lm_2d_clubs_major, stayman_2d_answer,
    stayman_2d_fit_rebid, transfer_completion, transfer_lebensohl_responder,
    transfer_stayman_2d_responder, transfer_target,
};

// Per-call alerts for the competitive book's artificial calls.  An [`Alert`] marks
// a call as *conventional*: the inference reader decodes it as the convention
// rather than as a natural suit.  Natural raises, natural suit rebids, natural
// notrump, penalty passes, and the catch-all `Pass` stay unalerted.

/// Cue-bid raise — a cue of the opponents' suit as a limit-plus raise of partner's
/// opening (not natural).
const CUE_RAISE: Alert = Alert("comp:cue-raise");
/// Negative double — responder's takeout double showing the unbid suit(s) after
/// partner opens and RHO overcalls.
const NEGATIVE_DOUBLE: Alert = Alert("comp:negative-double");
/// Support double / redouble — opener's `X`/`XX` showing exactly three-card support.
const SUPPORT_DOUBLE: Alert = Alert("comp:support-double");
/// Diamond penalty double — `X` of their `(2♦)` overcall of our 1NT under
/// `competition.two_diamond_double`: a real diamond suit with values, not the
/// cooperative double it replaces.  Alerted because the *reading* rides on it —
/// `project_authored` decodes alerted calls only, so without this the length and
/// suit-quality the gate promises never reach opener or the floor, and opener
/// competes over their runout blind to the diamonds it was told about.
const TWO_DIAMOND_PENALTY: Alert = Alert("comp:two-diamond-penalty");
/// Multi values double — `X` of their `(2♦)` Multi (one unknown six-card
/// major, `their.two_diamonds_multi`): values (`hcp 6+`, BBA's own band),
/// no diamond claim, describing once they name the major.
/// Alerted so it *reads*: an unalerted double is not decoded at all, and the
/// cooperative diamond double it replaces would otherwise be what opener and
/// the floor believe was bid.
const MULTI_VALUES: Alert = Alert("comp:multi-values");
/// Multi penalty double — opener's `X` of the advancer's pass-or-correct
/// `2♥`/`2♠` after responder's values double: four-plus trumps.  Nominally
/// penalty; when the overcaller's major is the other one they correct, and
/// the double has told partner where our trumps are.  Alerted for the same
/// reason as [`MULTI_VALUES`] — the length is the whole message.  Shared with
/// the Kokish–Kraft variant's *repeated* double, which is the same claim one
/// round later (`competition.multi_kokish_kraft`).
const MULTI_PENALTY: Alert = Alert("comp:multi-penalty");
/// Multi takeout double — responder's second `X` once their pass-or-correct
/// has resolved the major (`1NT (2♦) X (2M) - - X`): four of the *other*
/// major and 1–2 of theirs, BBA's "reopening double".  Alerted so opener
/// reads the other major, not a penalty holding.  Under the Kokish–Kraft
/// variant the same claim moves one branch over, to the double that follows
/// responder's *neutral pass* — the delayed-double split every exact-object
/// source in the survey makes.
const MULTI_TAKEOUT: Alert = Alert("comp:multi-takeout");
/// Multi stopper ask — responder's `3♠` after the opponents correct their
/// disclosed Multi to spades.  It denies a spade stopper and asks opener to
/// bid `3NT` with one or place the contract in a side suit.
const MULTI_STOPPER_ASK: Alert = Alert("comp:multi-stopper-ask");
/// Kokish–Kraft values double — `X` of their `(2♦)` Multi under
/// `competition.multi_kokish_kraft`: invitational-plus values (`hcp 8+`) with
/// **no shape promise at all**, the waiting call of that variant's table.
/// Alerted for [`MULTI_VALUES`]'s reason (an unalerted double is not decoded,
/// and the cooperative diamond double it replaces is what opener would
/// otherwise believe was bid); a separate slug because the band differs — the
/// 6–7 hands this one refuses take the designed neutral pass instead.
///
/// Under
/// [`CompetitionKnobs::multi_px_split`][crate::bidding::agreements::CompetitionKnobs::multi_px_split]
/// the "no shape promise at all" stops being true of the whole band: the call
/// is then game values *or* an invitation with a four-card major, so the 8–9
/// half promises one.  Same slug — the hull an opponent has to be told is still
/// `hcp 8+`, and the split is a matter of what our own partner infers.
const KK_VALUES: Alert = Alert("comp:kk-values");
/// Kokish–Kraft minor transfer — `2NT`→♣ and `3♣`→♦ over their `(2♦)` Multi:
/// a six-card minor with **no point floor**, so it is both the preempt of
/// their unknown major and the start of a game force.  Their `2♦` holds no
/// diamonds, so both minors are ours to transfer into.
const KK_MINOR_TRANSFER: Alert = Alert("comp:kk-minor-transfer");
/// Kokish–Kraft two-suiter rebid — responder's second call over a completed
/// minor transfer, naming a four-card second suit at a step the source fixes
/// rather than by rank (after `3♣`: `3♦` = ♥, `3♥` = ♠, `3♠` = ♦).  Game
/// forcing; the alert is what stops the step reading as its own suit.
const KK_TWO_SUITER: Alert = Alert("comp:kk-two-suiter");
/// Kokish–Kraft both minors — `3♠` over their `(2♦)` Multi: game-forcing with
/// at least 5-4 in the minors, naming a major nobody claims.
const KK_MINORS: Alert = Alert("comp:kk-minors");
/// Landy values double — `X` of their `(2♣)` Landy, values (8+) willing to
/// defend whichever major they run to.  Not the stolen Stayman it replaces:
/// against a both-majors overcall there is no major left to ask for.
///
/// The slug covers both widths.  By default the ordering caps the double at
/// nine points, because the table's ungated `3NT`@168 outranks it; under
/// `landy_notrump_no_major` (§N1p) `3NT` denies a four-card major and the
/// double picks up every game hand with length in a suit they showed.  The
/// meaning widens, the tag does not — `reading.bid_exclusion` republishes the
/// wider reading off the rule's own siblings, so there is nothing here to
/// re-tag and no `.bbsa` row to re-bless.
const LANDY_VALUES: Alert = Alert("comp:landy-values");
/// Landy cue — `2♥`/`2♠` over their `(2♣)` Landy: a cue of a shown major
/// naming the corresponding unshown minor (`2♥` = clubs, `2♠` = diamonds),
/// invitational or better with a 5+ suit.  Also the stopper-ask and re-cue
/// rungs above it, which name a major nobody holds.
const LANDY_CUE: Alert = Alert("comp:landy-cue");
/// Landy club transfer — `2NT` over their `(2♣)` Landy under the N1c re-rung
/// minors: a weak six-card club escape, transferred so the `1NT` opener
/// declares.  Their `2♣` is artificial, so clubs are ours to transfer into.
const LANDY_TRANSFER: Alert = Alert("comp:landy-transfer");
/// Landy both-minors takeout — `2♥`/`2♠` over their `(2♣)` Landy under the
/// N1j BBA ladder: game-forcing with 4+ in both minors and exactly a doubleton
/// in the bid major (2-2 bids `2♥`, so `2♠` promises three hearts).  Opener
/// answers in notrump with that major stopped or no four-card minor, else
/// picks a minor.
const LANDY_TKO: Alert = Alert("comp:landy-tko");
/// Landy both-minors splinter — `3♥`/`3♠` over their `(2♣)` Landy under the
/// N1j BBA ladder: the takeout hand with 0-1 in the bid major.
const LANDY_SPL: Alert = Alert("comp:landy-spl");
/// Landy penalty double — our side's `X` of the major their advance has named,
/// at either of the two seats that can make it: **opener's**, immediately over
/// the advance (`1NT (2♣) X (2♥)`, `competition.landy_opener_px`), and the
/// **doubler's** second `X` one round later (`1NT (2♣) X (2♥) - - X` and its
/// siblings, `competition.landy_doubler_rebids` and its flip arms).  One claim
/// at both: **length or honour strength in their major** — four-plus at the
/// top rung, exactly three under the §N1-lia cells
/// (`landy_doubler_three_honors` / `_three_small`, default-on 2026-08-30),
/// whose top-honor split the rules carry and the projection publishes.  The
/// floor's shorter values double lives under the same claim now that the
/// catch-all is gone.  Re-worded from "four-plus"
/// 2026-08-30 so the tag covers every cell that can fire under it — the arms
/// differ only in the rule, never in disclosure — which is what unblocked the
/// `landy_doubler_catchall=false` arm (the floor's short values double at the
/// same seat no longer contradicts the tag).
///
/// The two seats share the slug because they publish the same thing.  They
/// differ only in who is still to speak — opener doubles with partner able to
/// pull, the doubler's is the last word — and that is a matter for the
/// continuation tables, not for disclosure.
///
/// The polarity is this lane's house rule and it is the whole reason the alert
/// exists.  A double after our own double is penalty; a double after our
/// *pass* is takeout and stays the floor's.  Nothing mechanises that split
/// here — `inference::readers::penalty_x_reading_with_profile` requires *their*
/// 1NT opening, so `penalty_latch` cannot fire in this lane — which leaves the
/// alert and the `.penalty()` tag carrying the whole meaning.  An unalerted
/// second double reads as the takeout it is not, a phantom four-card holding in
/// the major nobody has left, so the alert has to publish **length in their
/// suit**, not values.  [`MULTI_PENALTY`] is the same claim one lane over, but
/// their `2♦` resolves to a major the overcaller may still correct; here the
/// preference is final, so this double never becomes a correction hint.
const LANDY_PENALTY: Alert = Alert("comp:landy-penalty");
/// Lia's stopper ask — opener's `2♠` over the §N1-lia takeout (`1NT (2♣) 2♥ -
/// 2♠`): no four-card minor and no spade stopper, by exclusion under the
/// reversed answer priority (minors first, then the `2NT` stopper rung).  A
/// cue of a suit *they* showed, constrained `hcp(0..)` so it projects nothing
/// — the alert is by hand, as the invariant's witness cannot see a vacuous
/// constraint, and it is what stops the walk reading opener for spades.
const LANDY_ASK: Alert = Alert("comp:landy-ask");
/// Lia's length answer — opener's reply to the §N1-lia minor rungs (`2♠` = 5+♣
/// weak-or-GF, `2NT` = long diamonds): the cheap raise shows **three-card**
/// support (`3♣` over `2♠`, `3♦` over `2NT`), the step below it a doubleton
/// (`2NT` over `2♠`, a contract; `3♣` over `2NT` — opener is balanced, so two
/// diamonds implies 3+ clubs, a safe landing).  Alerted so the reader decodes
/// the rule's exact `len` bands instead of the natural walk's four-card raise
/// floor — a three-card raise read as four is unsound, and the doubleton
/// `3♣` names a suit the rule says nothing about.
const LANDY_LENGTH: Alert = Alert("comp:landy-length");
/// Lebensohl `2NT` — the weak relay to `3♣` over their overcall of our `1NT`.
const LEBENSOHL_RELAY: Alert = Alert("comp:lebensohl-relay");
/// Opener's forced `3♣` completion of the Lebensohl relay — a puppet, not
/// clubs.  Constrained `hcp(0..)`, so its projection claims nothing: the
/// alert exists to *suppress* the natural walk, which would otherwise read
/// the completion as a club holding (the invariant's artificiality witness
/// cannot see a vacuous constraint, so this alert is by hand).
const LEBENSOHL_COMPLETION: Alert = Alert("comp:lebensohl-completion");
/// Lebensohl cue — a cue of their suit as game-forcing Stayman.
const LEBENSOHL_CUE: Alert = Alert("comp:lebensohl-cue");
/// Transfer-Lebensohl 3-level transfer — bids the next suit up *through* the
/// adverse suit (INV+).
const LEBENSOHL_TRANSFER: Alert = Alert("comp:lebensohl-transfer");
/// Stayman over `(2♦)` — `3♣` as game-forcing Stayman (with Smolen after the
/// `3♦` denial).
const STAYMAN: Alert = Alert("comp:stayman");
/// Smolen — showing a 5-card major right-sided after the Stayman denial.
const SMOLEN: Alert = Alert("comp:smolen");
/// Leaping Michaels — `4♣`/`4♦` jumps naming a 5-5 game-forcing two-suiter.
const LEAPING_MICHAELS: Alert = Alert("comp:leaping-michaels");
/// Unusual-vs-Unusual cue — `3♣`/`3♦` cues finding a major fit over their
/// both-minors `2NT`.
const UVU_CUE: Alert = Alert("comp:uvu-cue");
/// Unusual-vs-Unusual splinter — `4♣`/`4♦` as a FG+ 5-5-majors splinter into the
/// short minor.
const UVU_SPLINTER: Alert = Alert("comp:uvu-splinter");
/// Stayman re-ask — responder's `XX` after the opponents doubled our 2♣ Stayman
/// and opener passed to deny a club stopper: re-asks the major (forcing).
const STAYMAN_REDOUBLE: Alert = Alert("comp:stayman-redouble");
/// Transfer re-ask — responder's `XX` after the opponents doubled our Jacoby
/// transfer and opener passed to decline: forces opener to complete (forcing).
const TRANSFER_REDOUBLE: Alert = Alert("comp:transfer-redouble");
/// Unusual-vs-unusual over our 1M — the cheaper cue of the two-suiter's suits
/// (`3♣` over their both-minors `(2NT)`, the other-major cue over their
/// Michaels) as a limit-plus raise of our major.
const UVU_MAJOR_RAISE: Alert = Alert("comp:uvu-major-raise");
/// The second cue over their both-minors `(2NT)` — `3♦` as a game force with
/// 5+ cards in the other major.
const UVU_MAJOR_FOURTH: Alert = Alert("comp:uvu-major-fourth");
/// Unusual-vs-unusual over our 1m and their both-majors Michaels cue — `2♥`
/// (their lower suit) as a limit-plus raise of our minor.
const UVU_MINOR_RAISE: Alert = Alert("comp:uvu-minor-raise");
/// The second cue over their both-majors Michaels of our 1m — `2♠` as a game
/// force with 5+ cards in the unbid minor.
const UVU_MINOR_FOURTH: Alert = Alert("comp:uvu-minor-fourth");
/// Business redouble of their takeout double of our weak two — 13+ values
/// (redoubles are natural-by-default; the alert buys the points-floor decode).
const WEAK_TWO_XX: Alert = Alert("comp:weak-two-xx");
/// Ogust survives their overcall of our weak two — the contested `2NT` still
/// asks (2+ card support, 14+), alerted so the fit and strength project.
const CONTESTED_OGUST: Alert = Alert("comp:ogust");
/// Cachalot rotated double — 4+ cards in the *adjacent* major (hearts over
/// `(1♦)`, spades over `(1♥)`), not a classic unbid-majors negative double.
const CACHALOT_X: Alert = Alert("comp:cachalot-x");
/// Cachalot transfer — `1♥` over `(1♦)` showing 4+ **spades**.
const CACHALOT_TRANSFER: Alert = Alert("comp:cachalot-transfer");
/// Cachalot residual — `1♠` over `(1♦)`/`(1♥)` as the takeout hand, ≤3 in
/// each major the rotation could have shown.
const CACHALOT_TAKEOUT: Alert = Alert("comp:cachalot-takeout");
/// 2-level free-bid transfer (`FreeBidStyle::Transfer`) — a non-jump 2-level
/// new suit over their overcall showing the *other* unbid suit when exactly
/// two unbid suits sit at the two level; opener completes and declares.
const FREE_TRANSFER: Alert = Alert("comp:free-transfer");
/// Cachalot completion — opener's 1-level completion of the transfer shows
/// **exactly three** trumps (forcing one round; the raise shows four).
const CACHALOT_THREE: Alert = Alert("comp:cachalot-three");
/// Jordan/Truscott `2NT` over their takeout double — a limit-plus raise of
/// the opening (4+ support for a major, 5+ for a minor), not natural.
const JORDAN: Alert = Alert("comp:jordan");
/// Value redouble over their takeout double — 10+ without the Jordan fit
/// (redoubles are natural-by-default; the alert buys the points-floor decode).
const VALUE_REDOUBLE: Alert = Alert("comp:value-redouble");

/// The `X (bid) …` systems-on rebase, shared by the four
/// competition-over-our-own-convention packages
///
/// They doubled our artificial call and we answered with a bid; from there
/// responder's rebids are the *uncontested* tree, so strip the double to a
/// Pass and re-key.  `bid` is one answer the guard admits — the sample the
/// row layer seat-checks and probes with.
///
/// The guard is a two-call prefix with a free tail, which no named [`Pattern`]
/// construct spells: `Pattern::first("…", "X")` would also swallow the
/// `X - -` re-ask whose own table is declared just below it, rebasing the
/// re-ask instead of classifying it.  So it rides in verbatim through
/// [`Pattern::guarded`].
fn systems_on_over_double(key: &str, bid: &str) -> Entry {
    rebase(
        Pattern::guarded(
            key,
            &format!("(X) {bid}"),
            described_guard(
                "X (bid) …",
                guard(|_: &Context<'_>, s: &[Call]| {
                    s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
                }),
            ),
        ),
        described_rewrite(
            "systems on: their X is stripped to a pass",
            rewriter(|auction: &[Call], depth: usize| {
                if auction.get(depth) != Some(&Call::Double) {
                    return None;
                }
                let mut rewritten = auction.to_vec();
                rewritten[depth] = Call::Pass; // strip the X → systems on
                Some(rewritten)
            }),
        ),
    )
}

/// The competitive package over our openings: cue-bid raises, preemptive raises,
/// negative doubles for all four openings, support doubles/redoubles, and
/// opener's answers to negative doubles of minor overcalls
///
/// Standalone, the system-on rebase has nothing to land on; bind through
/// [`System::bind`][crate::bidding::System::bind] (as [`american`][super::american] is meant to be
/// used) so it resolves into the uncontested core.
#[must_use]
pub fn competition(agreements: &Agreements) -> Competitive {
    let mut book = Competitive::new();

    // Section 1 & 2: over all four openings, attach direct-seat response rules
    // and system-on over their double.
    compile_into(&mut book, agreements, &[direct_seat_package()]);

    // Section 2b: systems-on over their double of our splinter.
    // Section 3: support doubles and redoubles for each (opening, major) pair.
    compile_into(
        &mut book,
        agreements,
        &[splinter_doubled_package(), support_double_package()],
    );

    // Section 4: opener answers partner's negative double of a two-level minor.
    // Section 4b/4c: opener answers partner's cue-raise of the opening suit.
    compile_into(
        &mut book,
        agreements,
        &[
            answer_negative_double_package(),
            cue_raise_answer_package(),
            cue_minor_raise_answer_package(),
        ],
    );

    // Section 4d/4d′/4d″/4d‴: opener answers responder's natural free bid,
    // and the Negative style's capped-free-bid continuations.
    compile_into(&mut book, agreements, &[free_bid_answer_package()]);

    // Section 4f (`FreeBidStyle::Transfer`): opener completes the 2-level
    // free-bid transfer and responder clarifies. The swap contexts are a
    // closed enumeration — (opening, their overcall, lower slot → shown,
    // wrap slot → shown, completing a level higher on the wrap):
    compile_into(&mut book, agreements, &[transfer_free_bid_package()]);

    // Section 6: their two-suiters over our 1M — and the opt-in minor twin.
    compile_into(&mut book, agreements, &[uvu_over_majors_package()]);
    compile_into(&mut book, agreements, &[uvu_over_minors_package()]);

    // Section 11: over their takeout double (`agreements.competition.jordan_truscott`, default
    // on). Responder's first call at the deeper `1x (X)` key — it wins over
    // the `1x` FirstIs(X) systems-on rebase structurally, and the rebase
    // survives untouched below it for every deeper suffix the package's
    // exact-suffix guards don't claim.
    compile_into(&mut book, agreements, &[jordan_truscott_package()]);

    // Section 10: their jump / 3-level suit overcalls
    // (`agreements.competition.high_overcall_responses`, default off). A guarded entry at `1x` —
    // its bid range (2NT, 3♠] sits above the shipped per-overcall exact nodes
    // (which stop at 2♠), so nothing races it. Their (2NT) and their 3-level
    // cue of our own suit are excluded (the first is a two-suiter, the second
    // is rare enough for the floor).
    compile_into(&mut book, agreements, &[high_overcall_package()]);

    // Section 12: their three-level overcall of our 1NT
    // (`agreements.competition.nt_high_overcall_responses`, default on since
    // 2026-08-18).
    // Exact `1NT (3x)` nodes — the Lebensohl package stops at the two level and
    // the `(2NT)` Unusual-vs-Unusual node is a different key, so nothing races
    // them.  Responder's one call and opener's one answer; everything deeper is
    // the floor's.
    compile_into(&mut book, agreements, &[nt_high_overcall_package()]);

    // Section 9: opener's Cachalot answers (`NegativeDoubleShape::Cachalot`
    // only). Section 9b: opener's answers to the Sputnik residual double.
    compile_into(
        &mut book,
        agreements,
        &[cachalot_package(), sputnik_residual_answer_package()],
    );

    // Section 7: our contested weak twos (`agreements.competition.weak_two_competition`, default
    // off). Their double: responder's first call at the deeper `2M (X)` node
    // (business XX riding on the uncontested responses), everything deeper
    // systems-on. Their overcall (≤ 3♠): responder's direct action, and a
    // targeted rebase so an Ogust 2NT bid over the overcall still gets
    // opener's undisturbed five-rung answer.
    compile_into(&mut book, agreements, &[weak_two_competition_package()]);

    // Section 8: our contested strong 2♣ (`agreements.competition.strong_two_competition`,
    // default on). Their double steals no room → systems on wholesale; their
    // overcall gets responder's natural-GF / values-X / waiting-pass table,
    // backed by opener's forced reopening in the pass-out seat.
    compile_into(&mut book, agreements, &[strong_two_competition_package()]);

    // Section 5 / 5b / 5c: Lebensohl after our 1NT is overcalled at the 2
    // level. Purely additive — nothing else lands at `1NT` in the competitive
    // book. Plain or Transfer Lebensohl per [`LebensohlStyle`]; both keep the
    // weak 2NT relay, and (2♣) gets the systems-on rebase instead.
    compile_into(&mut book, agreements, &[lebensohl_package()]);

    // Competition over our own conventions: the opponents double or overcall
    // our Stayman, our Jacoby transfer, our two-way 2♠, or our 2NT diamond
    // transfer.  Each is keyed under the uncontested `1NT - <our call>`
    // node — a distinct trie path from the systems-on blocks where their call
    // sits at depth 1 — and each shares the `X (bid) …` systems-on rebase.
    compile_into(
        &mut book,
        agreements,
        &[
            competition_over_stayman_package(),
            competition_over_transfer_package(),
            competition_over_minor_transfer_package(),
            competition_over_diamond_transfer_package(),
        ],
    );

    // Section 5d: Unusual vs Unusual over their (2NT) overcall of our 1NT.
    compile_into(&mut book, agreements, &[uvu_package()]);

    book
}

#[cfg(test)]
mod tests;
