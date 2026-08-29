//! Deterministic safety shell over the distilled neural floor — AI-bidder M1.3.
//!
//! [`neural::classify_bba_v6`] is a
//! bare MLP: it emits a finite logit for every one of the 38 calls, with no
//! built-in respect for the laws or for the floor's non-negotiable
//! forced-situation rails.
//! [`ConfiguredFloorV6`][crate::bidding::neural_floor::ConfiguredFloorV6]
//! wraps it so it is safe to
//! attach as the floor, exactly where [`instinct()`]
//! attaches (see [`american`][super::american::american]).
//!
//! The shell has two paths:
//!
//! - **Forced** — when `instinct::forced` reports an
//!   *auction-determined* forced situation (partner's live takeout double, a
//!   prior call committing us to game, partner's just-made transfer over our
//!   strong notrump, or a live keycard conversation — partner's 4NT, their
//!   1430 answer to ours, or their placement over our answer), it returns the
//!   deterministic [`instinct()`] answer verbatim.  The net is never trusted
//!   on the rails; delegating reproduces the already-tested behavior exactly.
//!   The keycard rail exists because a convention in motion is not judgement:
//!   left to itself the net passed out asks, played 1430 answers, and
//!   redoubled doubled answers (the reading-drift A/B's worst boards).
//! - **Judgement** — otherwise it returns the net's logits, legality-masked: any
//!   call the laws forbid is set to `-∞`, while `Pass` (always legal) stays
//!   finite so a distribution always exists.  This is the vast middle the net is
//!   here to learn.  One optional stage follows the mask: `competitive_gate`
//!   prices the contested game-level node against the score table when
//!   `InstinctProfile::competitive_accountant` is on.  It only ever demotes, so
//!   knob-off is byte-identical and the net keeps its monopoly on introducing
//!   calls.
//!
//! One optional stage runs *before* the net instead of after it: the **PDI
//! dialect translation** (`pdi_swap`).  The net was distilled from BBA's book,
//! so where one of our authored rules means something that book reads
//! differently — a rule tagged [`Rules::pdi`][super::Rules::pdi] — the auction it
//! is shown is rewritten into the picture BBA would have had to reach the same
//! agreement.  Features come off the translated picture; the legality mask and
//! the accountant gate stay on the **real** one, so the shell can never
//! introduce an illegal call.  Off by default
//! ([`pdi_translate`][super::instinct::InstinctProfile::pdi_translate]); see
//! `docs/pdi.md`.
//!
//! Hand-conditioned game forces (a strong-notrump responder who *holds* game
//! values) are deliberately left to the net — that is judgement, measured in
//! aggregate by the A/B examples, not guarded here.
//!
//! Both paths are book-independent, so this shell is also what
//! [`american_floor`] stands on with no
//! authored book at all.

use super::Rules;
use super::array::Logits;
use super::context::Context;
use super::features::{CompactConfig, Config};
use super::inference::our_side_mask;
use super::instinct::{competitive_gate, forced};
use super::trie::Classifier;
use super::{features, neural};
use contract_bridge::Hand;
use contract_bridge::auction::{Auction, Call};
use std::sync::Arc;

/// The **configured** BBA-distilled floor — the card-input (v4) floor
///
/// The shipped default until the compact v5 floor won its 2026-08-08 gate A/B.
/// It stays reachable through
/// [`american_with_config`][super::american::american_with_config] and
/// [`dutch_with_config`][super::dutch::dutch_with_config].
///
/// A [`Classifier`] drop-in for [`instinct()`][super::instinct::instinct]: the
/// learned net in the judgement middle, the deterministic rails preserved by
/// delegation.  Distilled from the vendored **EPBot 2/1** oracle
/// ([`neural::classify_bba_v4`]) — a stronger learned prior than the
/// deterministic ladder it floors (EPBot clears our instinct floor by ~1.9
/// IMPs/board).  See the [module docs][self].
///
/// It is fed [`features_v4`][super::features::features_v4], whose last 280
/// floats are **both partnerships' convention cards**.  So the regime is an
/// *input* rather than a choice of artifact, and an A/B arm differs by a card
/// row instead of by a separately trained net — the confound
/// `docs/ai-bidder/configured-net.md` exists to remove.  It replaced the v3
/// twin scheme on gate 1's verdict: +0.1933/+0.2469 plain DD and
/// +0.5256/+0.5358 PD at 2,000,000 fresh boards per cell.
///
/// The [`Config`] is captured **once, when the floor is built**, from whatever
/// the knobs said at that moment (see
/// [`american`][super::american::american]).  A partnership is
/// built per A/B arm and a card is an agreement, not a per-call decision, so
/// this is the right granularity — and it keeps the per-decision path from
/// reading ambient state that could silently change what a feature vector
/// means.
///
/// The deterministic ladder the forced path delegates to is captured the same
/// way, as a constructor argument rather than a process-wide `LazyLock`:
/// [`instinct()`][super::instinct::instinct] reads the pinned profile at build
/// time too (its RKCB fields pick the kickback ladder over the plain one), so
/// a process that builds two differently-knobbed systems must not share one ladder
/// frozen at whichever came first.
/// `common::with_floor` builds it once per system and gives the same `Arc` to the
/// constructive book.
#[derive(Clone, Debug)]
pub struct ConfiguredFloorBba(Config, Arc<Rules>);

impl ConfiguredFloorBba {
    /// Attach the floor to one configuration cell — what each side is declared
    /// to play — over one deterministic ladder for the forced rails
    #[must_use]
    pub const fn new(config: Config, ladder: Arc<Rules>) -> Self {
        Self(config, ladder)
    }
}

impl Classifier for ConfiguredFloorBba {
    fn classify(&self, hand: Hand, context: &Context<'_>) -> Logits {
        if forced(context) {
            // Rails: trust the deterministic floor, never the net.
            return self.1.classify(hand, context);
        }
        // The context arrives from the trie without a config — only this floor
        // knows the cell — so attach ours for the extractor to read.  The clone
        // inside `dialect_context` copies scalars and borrows; the auction and
        // prefixes are not copied.
        let picture = pdi_picture(context);
        let configured = dialect_context(context, picture.as_ref()).with_config(&self.0);
        let mut logits = neural::classify_bba_v4(&features::features_v4(hand, &configured));
        if picture.is_some_and(|picture| picture.swap_output) {
            swap_pass_double(&mut logits);
        }
        mask_illegal(&mut logits, context.auction());
        competitive_gate(&mut logits, hand, context);
        logits
    }
}

/// The shipped compact-config floor retrained on the live authored reading.
#[derive(Clone, Debug)]
pub struct ConfiguredFloorV6(CompactConfig, Arc<Rules>, fn(&[f32]) -> Logits);

impl ConfiguredFloorV6 {
    /// Attach the v6 floor to one compact configuration cell and rail ladder.
    #[must_use]
    pub const fn new(compact: CompactConfig, ladder: Arc<Rules>) -> Self {
        Self(compact, ladder, neural::classify_bba_v6)
    }

    /// Attach the experimental twin trained on BBA's disclosed readings.
    #[must_use]
    pub(in crate::bidding) const fn new_their(compact: CompactConfig, ladder: Arc<Rules>) -> Self {
        Self(compact, ladder, neural::classify_bba_v6_their)
    }
}

impl Classifier for ConfiguredFloorV6 {
    fn classify(&self, hand: Hand, context: &Context<'_>) -> Logits {
        if forced(context) {
            return self.1.classify(hand, context);
        }
        let picture = pdi_picture(context);
        let configured = dialect_context(context, picture.as_ref()).with_compact(&self.0);
        let mut logits = (self.2)(&features::features_v6(hand, &configured));
        if picture.is_some_and(|picture| picture.swap_output) {
            swap_pass_double(&mut logits);
        }
        mask_illegal(&mut logits, context.auction());
        competitive_gate(&mut logits, hand, context);
        logits
    }
}

/// One PDI **dialect translation**: the auction to serve, and whether the
/// answer needs unswapping
///
/// See [`pdi_swap`] for the rules S1–S5 that build it.
struct PdiPicture {
    /// The auction rewritten into the floor's dialect — same length, same
    /// legality, our tagged calls restated
    swapped: Vec<Call>,
    /// The real and swapped pictures disagree on whether our double *stands*,
    /// so `Pass` and `Double` must trade logits before the legality mask (S5)
    swap_output: bool,
}

/// Whether index `i` carries a divergence tag in `flips`
///
/// Positions past 63 carry no bit — the shared `CallMasks` limitation, which
/// this makes total rather than a shift overflow.
const fn tagged(flips: u64, i: usize) -> bool {
    i < 64 && flips & (1 << i) != 0
}

/// Restate our PDI-divergent calls in the floor's own dialect
///
/// `flips` are the auction indices of **our** tagged calls (the caller scopes
/// the reading's table-wide mask with [`our_side_mask`]).  The rewrite is five
/// rules:
///
/// - **S1** — a tagged `X` becomes `P`.  Our tagged doubles are penalty; the
///   teacher's book reads a double in that seat as takeout, and the nearest
///   thing in its dialect to "nothing to ask for, happy to defend" is a pass.
/// - **S2** — our sit over a tagged `X` (the `[X, P, P]` window, tag on the
///   `X`) becomes `X`.  The pair's aggregate is preserved: we did double and we
///   did elect to defend, said in the order the teacher would say it.  A wider
///   window is impossible — a third pass would have ended the real auction.
/// - **S3** — nothing moved, no picture; serve the real auction.
/// - **S4** — replay the rewrite through [`Auction::try_extend`]; if it is
///   illegal, *or has ended*, there is no picture.  One check subsumes several
///   families at once: a tagged `X` in the pass-out seat (where `X → P` ends the
///   auction — this is what makes the §N1l preference legs untranslatable),
///   their immediate `XX` of our tagged `X` (a redouble with no double under
///   it), and anything else the rewrite cannot express.
/// - **S5** — if the auction ends `X P` with the tag on that `X`, the two
///   pictures disagree about whether our double is still standing, which is the
///   one place the *answer* needs translating back too.
///
/// Note what S2 does **not** get right: it moves the double one seat along, so
/// the side aggregate is faithful but the seat attribution of the four trumps is
/// not.  A knowing v1 approximation, priced by the A/B rather than papered over.
fn pdi_swap(auction: &[Call], flips: u64) -> Option<PdiPicture> {
    if flips == 0 {
        return None;
    }
    let mut swapped = auction.to_vec();
    let mut moved = false;
    for (index, &call) in auction.iter().enumerate() {
        if call != Call::Double || !tagged(flips, index) {
            continue;
        }
        swapped[index] = Call::Pass; // S1
        moved = true;
        if auction.get(index + 1) == Some(&Call::Pass)
            && auction.get(index + 2) == Some(&Call::Pass)
        {
            swapped[index + 2] = Call::Double; // S2
        }
    }
    if !moved {
        return None; // S3
    }
    // S4: the picture has to be a real auction that is still going.
    let mut replay = Auction::new();
    replay.try_extend(swapped.iter().copied()).ok()?;
    if replay.has_ended() {
        return None;
    }
    // S5
    let swap_output = auction.last() == Some(&Call::Pass)
        && auction.len() >= 2
        && auction[auction.len() - 2] == Call::Double
        && tagged(flips, auction.len() - 2);
    Some(PdiPicture {
        swapped,
        swap_output,
    })
}

/// The dialect translation in force for this decision, if any
///
/// The guard chain, in cost order: the knob, then a length the mask can address,
/// then an attached system (a bare diagnostic context has no trie prefixes, so
/// the swapped auction would read as almost nothing — serve it untranslated),
/// then the reading's tag mask scoped to the side to act.  `flips == 0` — every
/// decision in the shipped default, where nothing is tagged — costs one field
/// read off the already-cached reading.
fn pdi_picture(context: &Context<'_>) -> Option<PdiPicture> {
    if !context.decision_profile().instinct.pdi_translate {
        return None;
    }
    let auction = context.auction();
    if auction.len() > 64 || context.own_system().is_none() {
        return None;
    }
    pdi_swap(
        auction,
        context.inferences().pdi_flip() & our_side_mask(auction.len()),
    )
}

/// The context whose features the net is served: the translated picture where
/// the shell fires, the real one otherwise
///
/// `prefixed_context` rather than a bare [`Context::new`], because a keyless
/// context carries no trie prefixes — the swapped auction would lose the Landy
/// decode and read as a natural nothing, which is the opposite of the point.
/// The profile is re-pinned because attaching a system takes that system's own
/// pin, which is not necessarily this decision's.
fn dialect_context<'a>(context: &Context<'a>, picture: Option<&'a PdiPicture>) -> Context<'a> {
    match picture {
        Some(picture) => context
            .own_system()
            .expect("pdi_picture returns None without an attached system")
            .prefixed_context(context.vul(), &picture.swapped)
            .with_profile(context.decision_profile()),
        None => context.clone(),
    }
}

/// Trade the `Pass` and `Double` logits (S5)
fn swap_pass_double(logits: &mut Logits) {
    let pass = logits[Call::Pass];
    logits[Call::Pass] = logits[Call::Double];
    logits[Call::Double] = pass;
}

/// Set every call the laws forbid to `-∞`, leaving the rest as the net set them
///
/// Reuses [`Auction::can_push`] — the very predicate the driver filters with —
/// so the mask can never drift from the laws.  `Pass` is always legal, so it
/// stays finite and a distribution always exists (invariant §0.2).
pub(crate) fn mask_illegal(logits: &mut Logits, auction: &[Call]) {
    let mut played = Auction::new();
    // The slice is a real prior auction, so every call in it is legal.
    played
        .try_extend(auction.iter().copied())
        .expect("a prior table auction is legal");
    for (call, slot) in logits.iter_mut() {
        if played.can_push(call).is_err() {
            *slot = f32::NEG_INFINITY;
        }
    }
}

#[cfg(test)]
mod tests;
