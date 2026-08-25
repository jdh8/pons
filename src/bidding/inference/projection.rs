//! Rule projection: decoding an alerted call back into boxes
//!
//! Where the natural walk in [`super::read`] guesses a call's meaning from its
//! shape, this module replays the *authoring rule's own* `project` fold, so a
//! convention reads exactly what its rules promise.  [`AuthoringStepCache`] is
//! the per-auction memo that keeps the replay affordable.

#[cfg(test)]
use super::envelope::Envelope;
use super::envelope::{EnvelopeUnion, relative_of};
use super::knobs::*;
use crate::bidding::constraint::ProjectionKind;
use crate::bidding::context::Context;
use crate::bidding::rules::{CompiledRules, FaceMemo, ProjectedUnion, RuleIndex, Rules};
#[cfg(test)]
use contract_bridge::Strain;
use contract_bridge::Suit;
use contract_bridge::auction::Call;

/// How many boxes one call's exclusion fold may leave behind
///
/// A complement is a *disjunction* (`!(A & B) = !A | !B`), so folding several
/// multiplies terms.  Sixteen is comfortably above what the shipped book
/// produces and comfortably below `intersect_owned`'s own `debug_assert!(<
/// 64)`.  Dropping a complement costs precision, never soundness.
///
// ponytail: one flat cap for every table.  If some table's fold genuinely
// needs more, the upgrade path is a per-table budget or a precision-ranked
// fold order — not a bigger constant.
const EXCLUSION_BOX_BUDGET: usize = 16;

/// One rule's projection fold, through the compiled constant pool when there
/// is one and off the authored rule when there is not
fn rule_union<'a>(
    rules: &Rules,
    compiled: Option<&'a CompiledRules>,
    index: RuleIndex,
    ctx: &Context<'_>,
    kind: ProjectionKind,
) -> ProjectedUnion<'a> {
    if let Some(compiled) = compiled {
        return match kind {
            ProjectionKind::Forward => compiled.project_union_matched(rules, index, ctx),
            ProjectionKind::Band => compiled.project_band_union_matched(rules, index, ctx),
            ProjectionKind::Complement => {
                compiled.project_complement_union_matched(rules, index, ctx)
            }
            ProjectionKind::Announcement => compiled.announce_union_matched(rules, index, ctx),
        };
    }
    let rule = &rules.rules()[index as usize];
    ProjectedUnion::Owned(match kind {
        ProjectionKind::Forward => rule.project_union(ctx),
        ProjectionKind::Band => rule.project_band_union(ctx),
        ProjectionKind::Complement => rule.project_complement_union(ctx),
        ProjectionKind::Announcement => rule.announce_union(ctx),
    })
}

/// A call's face-live rules with their weights, in authored order
///
/// `alerted` restricts to the rules carrying an [`Alert`][crate::bidding::rules::Alert]
/// — the announcement overlay's half.
fn live_rules(
    rules: &Rules,
    compiled: Option<&CompiledRules>,
    ctx: &Context<'_>,
    made: Call,
    alerted: bool,
    memo: &mut FaceMemo,
) -> Vec<(RuleIndex, i16)> {
    if let Some(compiled) = compiled {
        let indices = if alerted {
            compiled.alerted_rule_indices(made)
        } else {
            compiled.rule_indices(made)
        };
        return indices
            .iter()
            .filter(|&&index| compiled.face_live_memoized(rules, index, ctx, memo))
            .map(|&index| (index, rules.rules()[index as usize].weight()))
            .collect();
    }
    rules
        .rules()
        .iter()
        .enumerate()
        .filter(|(_, rule)| {
            rule.call() == made && (!alerted || rule.alert().is_some()) && rule.face_live(ctx)
        })
        .map(|(index, rule)| {
            (
                RuleIndex::try_from(index).expect("a rule table fits in u32 indices"),
                rule.weight(),
            )
        })
        .collect()
}

/// The complements of the gates a bidder could have chosen instead of `made`,
/// each paired with the weight it must be compared against
///
/// Selection is argmax over legal calls with finite logits and a book logit is
/// `weight/100` or −∞, so a rule on another call whose weight strictly beats
/// the one that produced `made` must have been **false** — otherwise it, not
/// `made`, would have won.  A sibling counts only when it could really have
/// won: face-live ([`Rules::face`][crate::bidding::rules::Rules::face]) and
/// legal at this turn ([`Context::allows`]).
///
/// `floor` is the *lowest* threshold any caller will apply — the widest set —
/// so one gather serves every rule of the call, each filtering by its own
/// weight in [`exclude_siblings`].  ⊤ complements are dropped here, and the
/// rest sorted cheapest-first once: a one-box complement never grows the
/// product, so folding it before a disjunctive one buys the most exclusions
/// per box of budget.  The sort is stable, so authored order breaks ties.
fn sibling_complements<'a>(
    rules: &Rules,
    compiled: Option<&'a CompiledRules>,
    ctx: &Context<'_>,
    made: Call,
    floor: i16,
    memo: &mut FaceMemo,
) -> Vec<(i16, ProjectedUnion<'a>)> {
    let mut complements: Vec<(i16, ProjectedUnion<'a>)> = Vec::new();
    let mut push = |index: RuleIndex, memo: &mut FaceMemo| {
        let rule = &rules.rules()[index as usize];
        if !ctx.allows(rule.call()) {
            return;
        }
        let live = match compiled {
            Some(compiled) => compiled.face_live_memoized(rules, index, ctx, memo),
            None => rule.face_live(ctx),
        };
        if !live {
            return;
        }
        let complement = rule_union(rules, compiled, index, ctx, ProjectionKind::Complement);
        if complement.as_union() != &EnvelopeUnion::unknown() {
            complements.push((rule.weight(), complement));
        }
    };
    if let Some(compiled) = compiled {
        for index in compiled.stronger_siblings(made, floor).collect::<Vec<_>>() {
            push(index, memo);
        }
    } else {
        for (index, rule) in rules.rules().iter().enumerate() {
            if rule.call() != made && rule.weight() > floor {
                push(
                    RuleIndex::try_from(index).expect("a rule table fits in u32 indices"),
                    memo,
                );
            }
        }
    }
    complements.sort_by_key(|(_, complement)| complement.as_union().boxes().len());
    complements
}

/// Intersect one rule's reading with what its strictly-heavier siblings deny
///
/// A complement is skipped when its transient product would reach
/// `intersect_owned`'s 64-box wall, or when the tidied result would exceed
/// [`EXCLUSION_BOX_BUDGET`].
fn exclude_siblings(
    positive: EnvelopeUnion,
    complements: &[(i16, ProjectedUnion<'_>)],
    threshold: i16,
    profile: ReadingProfile,
) -> EnvelopeUnion {
    let mut acc = positive;
    for (weight, complement) in complements {
        if *weight <= threshold {
            continue;
        }
        let complement = complement.as_union();
        let product = acc.boxes().len() * complement.boxes().len();
        if product >= 64 {
            continue;
        }
        // `intersect_owned` only ever drops or merges boxes, so a product
        // within budget cannot leave a result above it — no trial fold needed.
        if product <= EXCLUSION_BOX_BUDGET {
            acc = acc.intersect_owned(complement, profile);
            continue;
        }
        let next = acc.clone().intersect_owned(complement, profile);
        if next.boxes().len() <= EXCLUSION_BOX_BUDGET {
            acc = next;
        }
    }
    acc
}

/// One call's reading, each live rule intersected with what its own
/// strictly-heavier siblings deny, paired with whether the *positive* reading
/// (the pre-fold union, what the knob-off projection would have been) was top
///
/// The flag is what keeps the natural walk alive under the fold: a `hcp(0..)`
/// catch-all projects ⊤ and hands its call to the walk, but `⊤ ∩ ¬(heavier
/// siblings)` is not ⊤, so keying `substitutes_natural` off the *folded* union
/// would suppress that walk and lose its length floors along with it.
///
/// [`None`] when the call has no live rule — the caller's "this table says
/// nothing here" signal.
fn excluded_union(
    rules: &Rules,
    compiled: Option<&CompiledRules>,
    ctx: &Context<'_>,
    made: Call,
    live: &[(RuleIndex, i16)],
    kind: ProjectionKind,
    memo: &mut FaceMemo,
) -> Option<(EnvelopeUnion, bool)> {
    let profile = ctx.reading_profile();
    let floor = live.iter().map(|&(_, weight)| weight).min()?;
    let complements = sibling_complements(rules, compiled, ctx, made, floor, memo);
    // A disjunction containing ⊤ *is* ⊤, so one top rule tops the union.
    let mut positive_top = false;
    let folded = live
        .iter()
        .map(|&(index, weight)| {
            let positive = rule_union(rules, compiled, index, ctx, kind).into_owned();
            positive_top |= positive == EnvelopeUnion::unknown();
            exclude_siblings(positive, &complements, weight, profile)
        })
        .reduce(|a, b| a.disjoin_with(b, profile))?;
    Some((folded, positive_top))
}

/// One table's pass reading: the union of its Pass rules' bands.
///
/// The Pass band deliberately unions every authored Pass rule, face-gated
/// ones included — a wider band is sound.
/// [`None`] when the table authors no Pass rule at all (the projection pass
/// then records nothing, as before).
#[inline]
pub(super) fn project_pass<'a>(
    rules: &Rules,
    compiled: Option<&'a CompiledRules>,
    ctx: &Context<'_>,
    profile: ReadingProfile,
) -> Option<ProjectedUnion<'a>> {
    let band = if let Some(compiled) = compiled {
        compiled
            .pass_rule_indices()
            .iter()
            .map(|&index| compiled.project_band_union_matched(rules, index, ctx))
            .reduce(|a, b| a.disjoin(b, profile))?
    } else {
        ProjectedUnion::Owned(
            rules
                .rules()
                .iter()
                .filter(|rule| rule.call() == Call::Pass)
                .map(|rule| rule.project_band_union(ctx))
                .reduce(|a, b| a.disjoin_with(b, profile))?,
        )
    };
    Some(band)
}

/// The per-call bits an authored reading leaves for the walk, indexed by
/// position in the auction (positions past 63 are unrepresentable and simply
/// carry no bits — the walk then reads them naturally)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CallMasks {
    /// Calls whose natural-walk meaning must be skipped: an informative
    /// authored projection, or an alerted call whose projection is top.
    pub(super) suppressed: u64,
    /// Non-pass calls resolved to a live authored rule, including projections
    /// that are top and must fall back to the walk.  Such a call is natural by
    /// the `artificial_calls_are_alerted` invariant, so the notrump-structure
    /// blanket — a guess for calls nothing is known about — must not erase it.
    pub(super) authored: u64,
    /// Calls whose informative authored projection substitutes for the walk.
    pub(super) substituted: u64,
    /// Calls a live authored rule alerts — the artificial ones.  A substituted
    /// call still records the suit it *named* in the lane's mechanical
    /// bid-history, so a later rebid, raise or cue still recognises it; an
    /// artificial one must not, or a transfer's face suit becomes a phantom
    /// holding.
    pub(super) artificial: u64,
    /// Per-suit call masks whose projection promises at least three through
    /// six cards: `length_floor[i]` holds the calls promising `i + 3`-plus.
    /// The walk consumes these chronologically to retain fit, rebid and cue
    /// bookkeeping without deriving meaning from the call's face.
    pub(super) length_floor: [[u64; 4]; 4],
    /// Calls whose *positive* rule promised nothing and whose reading is the
    /// exclusion fold alone: the walk reads their shape as it always did, and
    /// the strength it guesses on the way is rolled back afterwards, leaving
    /// the fold — which knows what the heavier siblings deny — to say it.
    pub(super) walk_shape: u64,
}

impl CallMasks {
    fn record(&mut self, index: usize, effect: &AuthoredEffect<'_>) {
        if index >= 64 {
            return;
        }
        let bit = 1 << index;
        if effect.authored {
            self.authored |= bit;
        }
        if effect.substitutes_natural {
            self.substituted |= bit;
            let projected = effect.projection.as_union().hull();
            for suit in Suit::ASC {
                let length = usize::from(projected.length(suit).min);
                for floor in self.length_floor.iter_mut().take(length.saturating_sub(2)) {
                    floor[suit as usize] |= bit;
                }
            }
        }
        if effect.artificial {
            self.artificial |= bit;
        }
        if effect.walk_shape {
            self.walk_shape |= bit;
        }
        if effect.suppresses_natural {
            self.suppressed |= bit;
        }
    }

    fn merge(&mut self, other: Self) {
        self.suppressed |= other.suppressed;
        self.authored |= other.authored;
        self.substituted |= other.substituted;
        self.artificial |= other.artificial;
        self.walk_shape |= other.walk_shape;
        for (mine, theirs) in self.length_floor.iter_mut().zip(other.length_floor) {
            for (mine, theirs) in mine.iter_mut().zip(theirs) {
                *mine |= theirs;
            }
        }
    }

    /// The length floor this call promises in `suit`, or zero
    pub(super) fn floor(self, index: usize, suit: Suit) -> u8 {
        let bit = 1u64 << index;
        self.length_floor
            .iter()
            .rposition(|floor| floor[suit as usize] & bit != 0)
            .map_or(0, |i| i as u8 + 3)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoredProjection {
    pub(super) unions: [EnvelopeUnion; 4],
    pub(super) announced_unions: [EnvelopeUnion; 4],
    pub(super) masks: CallMasks,
}

impl AuthoredProjection {
    fn unknown() -> Self {
        Self {
            unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            announced_unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            masks: CallMasks::default(),
        }
    }

    fn apply(
        &mut self,
        len: usize,
        index: usize,
        effect: AuthoredEffect<'_>,
        profile: ReadingProfile,
    ) {
        let who = relative_of(len, index) as usize;
        self.announced_unions[who].intersect_assign(effect.agreement(), profile);
        self.unions[who].intersect_assign(effect.projection.as_union(), profile);
        self.masks.record(index, &effect);
    }
}

struct AuthoredEffect<'a> {
    projection: crate::bidding::rules::ProjectedUnion<'a>,
    agreement: Option<crate::bidding::rules::ProjectedUnion<'a>>,
    authored: bool,
    artificial: bool,
    walk_shape: bool,
    substitutes_natural: bool,
    suppresses_natural: bool,
}

impl AuthoredEffect<'_> {
    fn agreement(&self) -> &EnvelopeUnion {
        self.agreement
            .as_ref()
            .unwrap_or(&self.projection)
            .as_union()
    }
}

#[inline(always)]
fn authored_effect<'a>(
    made: Call,
    ctx: &Context<'_>,
    classifier: &dyn crate::bidding::trie::Classifier,
    compiled: Option<&'a crate::bidding::rules::CompiledRules>,
    decode_pass: bool,
    announce_split: bool,
) -> Option<AuthoredEffect<'a>> {
    let rules = classifier.as_rules()?;
    let profile = ctx.reading_profile();
    let is_pass = made == Call::Pass;
    let scope = (!is_pass).then_some(profile.scope);
    let mut face_memo = crate::bidding::rules::FaceMemo::new();

    // A compiled call plan can reject structurally unreadable groups before
    // evaluating faces or materializing their projections.  The shipped
    // Alerted scope sees many natural authoring nodes while walking every
    // prefix; those groups cannot contribute regardless of context.  Keep the
    // non-compiled oracle's evaluation order intact for opaque public hooks.
    if let Some(compiled) = compiled {
        if is_pass {
            if !decode_pass && compiled.can_skip_pass_effect() {
                return None;
            }
        } else {
            match scope.expect("non-pass reading scope") {
                ReadingScope::None if compiled.can_skip_nonpass_effect(made, profile) => {
                    return None;
                }
                ReadingScope::Alerted
                    if compiled.alerted_rule_indices(made).is_empty()
                        && compiled.can_skip_nonpass_effect(made, profile) =>
                {
                    return None;
                }
                ReadingScope::None | ReadingScope::Alerted | ReadingScope::All => {}
            }
        }
    }

    // Knob-on, each of `made`'s rules is intersected with what *its own*
    // strictly-heavier siblings deny, so the fold needs the rules kept apart
    // instead of unioned here — and it waits until the decode gate below has
    // passed, so an undecoded call never evaluates a sibling.
    let excluding = profile.bid_exclusion && !is_pass;
    let live = if excluding {
        live_rules(rules, compiled, ctx, made, false, &mut face_memo)
    } else {
        Vec::new()
    };
    let projection = if excluding {
        if live.is_empty() {
            return None;
        }
        None
    } else if is_pass {
        Some(project_pass(rules, compiled, ctx, profile)?)
    } else if let Some(compiled) = compiled {
        Some(
            compiled
                .rule_indices(made)
                .iter()
                .filter(|&&index| compiled.face_live_memoized(rules, index, ctx, &mut face_memo))
                .map(|&index| compiled.project_union_matched(rules, index, ctx))
                .reduce(|a, b| a.disjoin(b, profile))?,
        )
    } else {
        Some(
            rules
                .rules()
                .iter()
                .filter(|rule| rule.call() == made && rule.face_live(ctx))
                .map(|rule| ProjectedUnion::Owned(rule.project_union(ctx)))
                .reduce(|a, b| a.disjoin(b, profile))?,
        )
    };

    let alerted = !is_pass
        && if let Some(compiled) = compiled {
            compiled
                .alerted_rule_indices(made)
                .iter()
                .any(|&index| compiled.face_live_memoized(rules, index, ctx, &mut face_memo))
        } else {
            rules
                .rules()
                .iter()
                .any(|rule| rule.call() == made && rule.alert().is_some() && rule.face_live(ctx))
        };
    let decode = if is_pass {
        decode_pass
    } else {
        match scope.expect("non-pass reading scope") {
            ReadingScope::None => false,
            ReadingScope::Alerted => alerted,
            ReadingScope::All => true,
        }
    };
    if !decode {
        return None;
    }

    let agreement = if announce_split && !is_pass {
        if excluding {
            // The announced agreement is a second overlay off the same rules,
            // so it folds through the same siblings — one alerted rule at a
            // time, each against its own threshold.
            let alerted_live = live_rules(rules, compiled, ctx, made, true, &mut face_memo);
            excluded_union(
                rules,
                compiled,
                ctx,
                made,
                &alerted_live,
                ProjectionKind::Announcement,
                &mut face_memo,
            )
            .map(|(union, _)| ProjectedUnion::Owned(union))
        } else if let Some(compiled) = compiled {
            compiled
                .alerted_rule_indices(made)
                .iter()
                .filter(|&&index| compiled.face_live_memoized(rules, index, ctx, &mut face_memo))
                .map(|&index| compiled.announce_union_matched(rules, index, ctx))
                .reduce(|a, b| a.disjoin(b, profile))
        } else {
            rules
                .rules()
                .iter()
                .filter(|rule| rule.call() == made && rule.alert().is_some() && rule.face_live(ctx))
                .map(|rule| ProjectedUnion::Owned(rule.announce_union(ctx)))
                .reduce(|a, b| a.disjoin(b, profile))
        }
    } else {
        None
    };
    let (projection, positive_top) = match projection {
        Some(projection) => {
            let top = projection.as_union() == &EnvelopeUnion::unknown();
            (projection, top)
        }
        None => {
            let (union, positive_top) = excluded_union(
                rules,
                compiled,
                ctx,
                made,
                &live,
                ProjectionKind::Forward,
                &mut face_memo,
            )
            .expect("a non-empty live-rule list folds to a projection");
            (ProjectedUnion::Owned(union), positive_top)
        }
    };
    let informative = !is_pass && projection.as_union() != &EnvelopeUnion::unknown();
    // A `hcp(0..)` catch-all projects ⊤ and hands its call to the walk; the
    // fold then makes `⊤ ∩ ¬(heavier siblings)` informative, and substituting
    // it wholesale threw the walk's *lengths* away with its strength guess —
    // `(1♥) 2♥ - 2♠` read ♠0+, and the keycard ladder keyed a void.  Such a
    // call is natural (`artificial_calls_are_alerted`), so the walk reads its
    // shape and the fold keeps the strength it alone knows.
    let walk_shape = informative && positive_top && !alerted;
    let substitutes_natural = informative && !walk_shape;
    Some(AuthoredEffect {
        projection,
        agreement,
        authored: !is_pass,
        artificial: alerted,
        walk_shape,
        substitutes_natural,
        suppresses_natural: alerted || substitutes_natural,
    })
}

#[derive(Clone, Debug)]
struct AbsoluteProjection {
    unions: [EnvelopeUnion; 4],
    announced_unions: [EnvelopeUnion; 4],
    masks: CallMasks,
}

impl AbsoluteProjection {
    fn unknown() -> Self {
        Self {
            unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            announced_unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            masks: CallMasks::default(),
        }
    }

    fn push(&mut self, index: usize, effect: AuthoredEffect<'_>, profile: ReadingProfile) {
        let seat = index % 4;
        self.unions[seat].intersect_assign(effect.projection.as_union(), profile);
        self.announced_unions[seat].intersect_assign(effect.agreement(), profile);
        self.masks.record(index, &effect);
    }
}

/// One call's resolved route, recorded so the route audit can replay it
///
/// Written on every `prepare` but read only under `#[cfg(test)]`, by
/// [`assert_cached_route_matches_one_shot`] — hence the `dead_code` allowance,
/// which covers the release build alone.  Deleting this deletes that audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
struct CachedRoute {
    classifier_id: crate::bidding::decoder::DecoderClassifierId,
    provenance: crate::bidding::trie::Provenance,
    pattern_id: Option<crate::bidding::rows::PatternId>,
    table_id: Option<crate::bidding::rows::RuleTableId>,
    fast: bool,
}

impl From<crate::bidding::decoder::DecodedAuthoring<'_>> for CachedRoute {
    fn from(answer: crate::bidding::decoder::DecodedAuthoring<'_>) -> Self {
        Self {
            classifier_id: answer.classifier_id,
            provenance: answer.provenance,
            pattern_id: answer.pattern_id,
            table_id: answer.table_id,
            fast: answer.fast,
        }
    }
}

/// One route-safe append waiting for its projection effects to be committed.
///
/// A table side normally advances by two calls between decisions, so keep the
/// common transaction entirely on the stack.  Seeded auctions and other large
/// jumps spill only the excess entries below.
#[derive(Clone, Copy)]
struct PendingAuthoringStep<'a> {
    index: usize,
    call: Call,
    own: Option<crate::bidding::decoder::DecodedAuthoring<'a>>,
    own_exact: Option<&'a dyn crate::bidding::trie::Classifier>,
    own_compiled: Option<&'a crate::bidding::rules::CompiledRules>,
    routed: Option<crate::bidding::decoder::DecodedAuthoring<'a>>,
    routed_compiled: Option<&'a crate::bidding::rules::CompiledRules>,
}

/// Whether retaining this call's authored effect can omit no observable work.
///
/// Non-rule classifiers have no projection effect. Rule-backed routes require
/// a plan compiled for the current profile and its explicit purity proof;
/// otherwise public face/projection hooks must keep replaying on every read.
fn authored_effect_is_reusable(
    made: Call,
    classifier: &dyn crate::bidding::trie::Classifier,
    compiled: Option<&crate::bidding::rules::CompiledRules>,
    profile: ReadingProfile,
) -> bool {
    classifier.as_rules().is_none()
        || compiled.is_some_and(|plan| plan.can_reuse_authored_effect(made, profile))
}

/// The per-call audit trail [`AuthoringStepCache`] appends as it walks
///
/// Same story as [`CachedRoute`]: written unconditionally, read only by
/// [`AuthoringStepCache::assert_new_route_records_match_one_shot`], which checks
/// every cached route against a one-shot resolve.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct AuthoringStepRecord {
    call: Call,
    own: Option<CachedRoute>,
    routed: Option<CachedRoute>,
}

/// Deal-owned append-only cache for the authored-reading portion of inference.
///
/// It is created by `Table::bid_out_from`, never by `Context::new`.  Each call
/// is decoded and projected once, category accumulators retain the historical
/// own/opponent/Pass/probe fold order, and the snapshot is rotated to the next
/// actor in constant work.  A profile change, non-prefix query, or selected
/// opaque/dynamic route permanently drops this deal to the legacy reader.
pub(crate) struct AuthoringStepCache {
    partnership_identity: Option<std::sync::Arc<crate::bidding::book::PartnershipCacheIdentity>>,
    profile: Option<ReadingProfile>,
    vul: Option<contract_bridge::auction::RelativeVulnerability>,
    reader_parity: Option<usize>,
    phase: Option<crate::bidding::book::Phase>,
    auction: Vec<Call>,
    context_cursor: crate::bidding::context::ContextCursor,
    own_cursor: crate::bidding::decoder::DecoderCursorState,
    /// `[side][phase]`, side 0 ours and side 1 theirs — a mirror or declared
    /// book decodes their calls, so the routed walk cannot share one cursor
    /// set across the table (the legacy reader splits the same way).
    routed_cursors: [[crate::bidding::decoder::DecoderCursorState; 3]; 2],
    own: AbsoluteProjection,
    opponents: AbsoluteProjection,
    passes: AbsoluteProjection,
    probed: AbsoluteProjection,
    records: Vec<AuthoringStepRecord>,
    snapshot: AuthoredProjection,
    disabled: bool,
    #[cfg(test)]
    successful_prepares: usize,
    #[cfg(test)]
    appended_steps: usize,
}

impl Default for AuthoringStepCache {
    fn default() -> Self {
        Self {
            partnership_identity: None,
            profile: None,
            vul: None,
            reader_parity: None,
            phase: None,
            auction: Vec::new(),
            context_cursor: crate::bidding::context::ContextCursor::new(),
            own_cursor: crate::bidding::decoder::DecoderCursorState::default(),
            routed_cursors: std::array::from_fn(|_| {
                std::array::from_fn(|_| crate::bidding::decoder::DecoderCursorState::default())
            }),
            own: AbsoluteProjection::unknown(),
            opponents: AbsoluteProjection::unknown(),
            passes: AbsoluteProjection::unknown(),
            probed: AbsoluteProjection::unknown(),
            records: Vec::new(),
            snapshot: AuthoredProjection::unknown(),
            disabled: false,
            #[cfg(test)]
            successful_prepares: 0,
            #[cfg(test)]
            appended_steps: 0,
        }
    }
}

impl AuthoringStepCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) const fn coverage(&self) -> (usize, usize) {
        (self.successful_prepares, self.appended_steps)
    }

    /// Check the route identity cached for each record appended by the most
    /// recent successful prepare against both independent resolution paths.
    ///
    /// The decoder pool ID is the persisted classifier identity; the pointer
    /// comparison below is deliberately only a same-process oracle against the
    /// authoritative trie.  Comparing the full cached record also checks the
    /// exact fallback depth/index and typed-rebase count in
    /// [`Provenance`][crate::bidding::trie::Provenance].
    #[cfg(test)]
    fn assert_new_route_records_match_one_shot(
        &self,
        partnership: &crate::bidding::book::Partnership,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
        first_new: usize,
    ) {
        assert_eq!(self.auction, auction, "route audit auction differs");
        assert_eq!(
            self.records.len(),
            auction.len(),
            "route record count differs"
        );
        let phase = self.phase.expect("a successful prepare has a phase");
        let profile = self.profile.expect("a successful prepare has a profile");
        let reader_parity = self
            .reader_parity
            .expect("a successful prepare has a reader parity");
        let full_context = Context::new(vul, auction);

        for (index, record) in self.records.iter().enumerate().skip(first_new) {
            let prefix = &auction[..index];
            assert_eq!(
                record.call, auction[index],
                "cached call differs at {index}"
            );

            if profile.fallback_projection {
                assert_cached_route_matches_one_shot(
                    "own",
                    index,
                    record.own,
                    partnership.decoder_for_phase(phase),
                    partnership.trie_for(auction),
                    &full_context,
                    prefix,
                );
            } else {
                assert_eq!(record.own, None, "disabled own route cached at {index}");
            }

            let opponent = index % 2 != reader_parity;
            let routed_needed = (profile.table_alerts && opponent)
                || (profile.pass
                    && record.call == Call::Pass
                    && (!opponent || profile.table_alerts));
            if routed_needed {
                let actor_vul = if index % 2 == reader_parity {
                    vul
                } else {
                    crate::bidding::context::flipped(vul)
                };
                let at_time = Context::new(actor_vul, prefix);
                let routed_books = if opponent {
                    partnership.opponents()
                } else {
                    partnership
                };
                assert_cached_route_matches_one_shot(
                    "routed",
                    index,
                    record.routed,
                    routed_books.decoder_for(prefix),
                    routed_books.trie_for(prefix),
                    &at_time,
                    prefix,
                );
            } else {
                assert_eq!(
                    record.routed, None,
                    "unneeded routed route cached at {index}"
                );
            }
        }
    }

    fn phase_index(phase: crate::bidding::book::Phase) -> usize {
        match phase {
            crate::bidding::book::Phase::Constructive => 0,
            crate::bidding::book::Phase::Competitive => 1,
            crate::bidding::book::Phase::Defensive => 2,
        }
    }

    fn clear_steps(&mut self, phase: crate::bidding::book::Phase) {
        self.phase = Some(phase);
        self.auction.clear();
        self.context_cursor = crate::bidding::context::ContextCursor::new();
        self.own_cursor = crate::bidding::decoder::DecoderCursorState::default();
        self.routed_cursors = std::array::from_fn(|_| {
            std::array::from_fn(|_| crate::bidding::decoder::DecoderCursorState::default())
        });
        self.own = AbsoluteProjection::unknown();
        self.opponents = AbsoluteProjection::unknown();
        self.passes = AbsoluteProjection::unknown();
        self.probed = AbsoluteProjection::unknown();
        self.records.clear();
        self.snapshot = AuthoredProjection::unknown();
    }

    fn disable(&mut self) -> Option<&AuthoredProjection> {
        self.disabled = true;
        None
    }

    pub(crate) fn prepare<'a>(
        &'a mut self,
        partnership: &crate::bidding::book::Partnership,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
    ) -> Option<&'a AuthoredProjection> {
        if self.disabled {
            return None;
        }
        // ponytail: a declared opponent ([`Partnership::with_opponents`]) falls back to the
        // full-auction reader, which already routes each call to the books of
        // the side that made it.  Splitting this incremental walk's `routed`
        // cursors the same way is straightforward — six slots instead of
        // three — but a mixed table is an A/B instrument, not a hot path.
        if partnership.has_declared_opponents() {
            return self.disable();
        }
        match &self.partnership_identity {
            None => {
                self.partnership_identity =
                    Some(std::sync::Arc::clone(partnership.cache_identity()))
            }
            Some(identity) if std::sync::Arc::ptr_eq(identity, partnership.cache_identity()) => {}
            Some(_) => return self.disable(),
        }
        // The partnership's pinned reading, not bare-context defaults: this walk
        // serves that partnership, and `Partnership::profile_mut` invalidates the cache
        // identity checked just above, so a deliberate edit still resets it.
        let profile = partnership.profile().reading;
        let reader_parity = auction.len() % 2;
        match (self.profile, self.vul, self.reader_parity) {
            (None, None, None) => {
                self.profile = Some(profile);
                self.vul = Some(vul);
                self.reader_parity = Some(reader_parity);
            }
            (Some(old_profile), Some(old_vul), Some(old_parity))
                if old_profile == profile && old_vul == vul && old_parity == reader_parity => {}
            _ => return self.disable(),
        }
        if !auction.starts_with(&self.auction) {
            return self.disable();
        }

        let mut full_cursor = self.context_cursor.clone();
        for &call in &auction[self.auction.len()..] {
            full_cursor.push(call);
        }
        let phase = full_cursor.phase();
        let phase_changed = self.phase != Some(phase);
        if phase_changed {
            full_cursor = crate::bidding::context::ContextCursor::new();
            for &call in auction {
                full_cursor.push(call);
            }
        }
        let full_context = full_cursor.context(vul, auction);
        let announce_split = profile.announced;
        let read_passes = profile.pass;
        let table_alerts = profile.table_alerts;
        let fallback_projection = profile.fallback_projection;

        // Route the whole append before consulting any authored projection.
        // An opaque route at the second (normally opponent) call must not make
        // an earlier public face/projection hook run speculatively before the
        // caller falls back to the legacy full-auction reader.
        let start = if phase_changed { 0 } else { self.auction.len() };
        let mut scan_context_cursor = if phase_changed {
            crate::bidding::context::ContextCursor::new()
        } else {
            self.context_cursor.clone()
        };
        let mut scan_own_cursor = if phase_changed {
            crate::bidding::decoder::DecoderCursorState::default()
        } else {
            core::mem::take(&mut self.own_cursor)
        };
        let mut scan_routed_cursors = if phase_changed {
            std::array::from_fn(|_| {
                std::array::from_fn(|_| crate::bidding::decoder::DecoderCursorState::default())
            })
        } else {
            core::mem::take(&mut self.routed_cursors)
        };
        let mut pending_inline: [Option<PendingAuthoringStep<'_>>; 2] = [None; 2];
        let mut pending_inline_len = 0usize;
        let mut pending_spill = Vec::<PendingAuthoringStep<'_>>::new();

        for index in start..auction.len() {
            let prefix = &auction[..index];
            let made = auction[index];
            let actor_vul = if index % 2 == reader_parity {
                vul
            } else {
                crate::bidding::context::flipped(vul)
            };
            let at_time = scan_context_cursor.context(actor_vul, prefix);

            let own_decoder = partnership.decoder_for_phase(phase);
            let own_answer = if fallback_projection {
                match own_decoder.resolve_checked_with_cursor(
                    &mut scan_own_cursor,
                    &full_context,
                    prefix,
                ) {
                    crate::bidding::decoder::CheckedResolution::Decoded(answer) => answer,
                    crate::bidding::decoder::CheckedResolution::Opaque => return self.disable(),
                }
            } else {
                None
            };
            if fallback_projection && !scan_own_cursor.cache_stable() {
                return self.disable();
            }
            let own_exact = if fallback_projection {
                None
            } else {
                partnership.trie_for(auction).get(prefix)
            };
            let own_classifier = own_answer.map(|answer| answer.classifier).or(own_exact);
            let own_needed = profile.scope != ReadingScope::All || index % 2 == reader_parity;
            let own_compiled =
                own_needed
                    .then_some(own_classifier)
                    .flatten()
                    .and_then(|classifier| {
                        partnership.compiled_rules_for(auction, classifier, profile)
                    });
            if own_needed
                && own_classifier.is_some_and(|classifier| {
                    !authored_effect_is_reusable(made, classifier, own_compiled, profile)
                })
            {
                return self.disable();
            }

            let opponent = index % 2 != reader_parity;
            let routed_needed = (table_alerts && opponent)
                || (read_passes && made == Call::Pass && (!opponent || table_alerts));
            let routed_phase = scan_context_cursor.phase();
            // Each call resolves in the books of the side that *made* it — the
            // mirror book for theirs, ours for ours.  Same books twice unless
            // one is attached, and the cursor slot follows the side.
            let routed_books = if opponent {
                partnership.opponents()
            } else {
                partnership
            };
            let routed_slot = &mut scan_routed_cursors[usize::from(opponent)];
            let routed_decoder = routed_books.decoder_for_phase(routed_phase);
            let routed_answer = if routed_needed {
                match routed_decoder.resolve_checked_with_cursor(
                    &mut routed_slot[Self::phase_index(routed_phase)],
                    &at_time,
                    prefix,
                ) {
                    crate::bidding::decoder::CheckedResolution::Decoded(answer) => answer,
                    crate::bidding::decoder::CheckedResolution::Opaque => return self.disable(),
                }
            } else {
                None
            };
            if routed_needed && !routed_slot[Self::phase_index(routed_phase)].cache_stable() {
                return self.disable();
            }
            let routed_compiled = routed_answer.and_then(|answer| {
                routed_books.compiled_rules_for(prefix, answer.classifier, profile)
            });
            if routed_answer.is_some_and(|answer| {
                !authored_effect_is_reusable(made, answer.classifier, routed_compiled, profile)
            }) {
                return self.disable();
            }

            let pending = PendingAuthoringStep {
                index,
                call: made,
                own: own_answer,
                own_exact,
                own_compiled,
                routed: routed_answer,
                routed_compiled,
            };
            if pending_inline_len < pending_inline.len() {
                pending_inline[pending_inline_len] = Some(pending);
                pending_inline_len += 1;
            } else {
                pending_spill.push(pending);
            }
            scan_context_cursor.push(made);
        }

        // Every needed route is now known cache-safe. Reset on a phase change
        // only at this commit point, then replay the append in its original
        // call/category order so observable public hooks retain legacy order.
        if phase_changed {
            self.clear_steps(phase);
        }
        for pending in pending_inline[..pending_inline_len]
            .iter()
            .flatten()
            .chain(&pending_spill)
        {
            let index = pending.index;
            let prefix = &auction[..index];
            let made = pending.call;
            let actor_vul = if index % 2 == reader_parity {
                vul
            } else {
                crate::bidding::context::flipped(vul)
            };
            let at_time = self.context_cursor.context(actor_vul, prefix);

            let own_side = index % 2 == reader_parity;

            if (profile.scope != ReadingScope::All || own_side)
                && let Some(answer) = pending.own
            {
                if let Some(effect) = authored_effect(
                    made,
                    &at_time,
                    answer.classifier,
                    pending.own_compiled,
                    false,
                    announce_split,
                ) {
                    self.own.push(index, effect, profile);
                }
            } else if (profile.scope != ReadingScope::All || own_side)
                && !fallback_projection
                && let Some(classifier) = pending.own_exact
                && let Some(effect) = authored_effect(
                    made,
                    &at_time,
                    classifier,
                    pending.own_compiled,
                    false,
                    announce_split,
                )
            {
                self.own.push(index, effect, profile);
            }

            let opponent = index % 2 != reader_parity;
            let routed_answer = pending.routed;
            if let Some(answer) = routed_answer {
                if table_alerts
                    && opponent
                    && let Some(effect) = authored_effect(
                        made,
                        &at_time.clone().with_profile({
                            let mut decision = partnership.profile();
                            decision.reading.scope = ReadingScope::Alerted;
                            decision
                        }),
                        answer.classifier,
                        pending.routed_compiled,
                        false,
                        announce_split,
                    )
                {
                    self.opponents.push(index, effect, profile);
                }
                if read_passes
                    && made == Call::Pass
                    && (!opponent || table_alerts)
                    && let Some(effect) = authored_effect(
                        made,
                        &at_time,
                        answer.classifier,
                        pending.routed_compiled,
                        true,
                        announce_split,
                    )
                {
                    self.passes.push(index, effect, profile);
                }
            }

            if profile.probed
                && let Some(&box_) = partnership.probed_box(&auction[..=index])
            {
                let union = EnvelopeUnion::from(box_);
                self.probed.push(
                    index,
                    AuthoredEffect {
                        projection: crate::bidding::rules::ProjectedUnion::Owned(union),
                        agreement: None,
                        authored: false,
                        artificial: false,
                        walk_shape: false,
                        substitutes_natural: false,
                        suppresses_natural: false,
                    },
                    profile,
                );
            }
            self.records.push(AuthoringStepRecord {
                call: made,
                own: pending.own.map(Into::into),
                routed: routed_answer.map(Into::into),
            });
            self.context_cursor.push(made);
            self.auction.push(made);
            #[cfg(test)]
            {
                self.appended_steps += 1;
            }
        }
        self.own_cursor = scan_own_cursor;
        self.routed_cursors = scan_routed_cursors;

        let mut snapshot = AuthoredProjection::unknown();
        for absolute in 0..4 {
            let relative = (absolute + 4 - auction.len() % 4) % 4;
            for category in [&self.own, &self.opponents, &self.passes, &self.probed] {
                snapshot.unions[relative].intersect_assign(&category.unions[absolute], profile);
                snapshot.announced_unions[relative]
                    .intersect_assign(&category.announced_unions[absolute], profile);
                snapshot.masks.merge(category.masks);
            }
        }
        self.snapshot = snapshot;
        #[cfg(test)]
        {
            self.successful_prepares += 1;
        }
        Some(&self.snapshot)
    }
}

pub(super) fn project_authored(context: &Context<'_>) -> AuthoredProjection {
    if let Some(projection) = context.authored_projection() {
        return projection.clone();
    }
    project_authored_with(context, true)
}

/// Same-process semantic oracle retained for compilation parity tests.
#[cfg(test)]
fn project_authored_legacy(context: &Context<'_>) -> AuthoredProjection {
    project_authored_with(context, false)
}

#[cfg(test)]
pub(crate) fn assert_compiled_authoring_projection_parity(context: &Context<'_>) {
    let compiled = project_authored(context);
    let legacy = project_authored_legacy(context);
    assert_eq!(compiled, legacy, "authored projection differs");
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn assert_cached_route_matches_one_shot(
    lane: &str,
    index: usize,
    cached: Option<CachedRoute>,
    decoder: &crate::bidding::decoder::AuthoringDecoder,
    trie: &crate::bidding::trie::Trie,
    context: &Context<'_>,
    prefix: &[Call],
) {
    let decoded = decoder.resolve(context, prefix);
    let legacy = trie.resolve(context, prefix);
    match (decoded, legacy) {
        (None, None) => {}
        (Some(decoded), Some((legacy_classifier, legacy_provenance))) => {
            assert!(
                core::ptr::eq(decoded.classifier, legacy_classifier),
                "{lane} classifier differs at call {index}"
            );
            assert_eq!(
                decoded.provenance.depth, legacy_provenance.depth,
                "{lane} fallback depth differs at call {index}"
            );
            assert_eq!(
                decoded.provenance.fallback, legacy_provenance.fallback,
                "{lane} fallback index differs at call {index}"
            );
            assert_eq!(
                decoded.provenance.rebases, legacy_provenance.rebases,
                "{lane} typed-rebase count differs at call {index}"
            );
        }
        (decoded, legacy) => panic!(
            "{lane} route presence differs at call {index}: decoder={}, legacy={}",
            decoded.is_some(),
            legacy.is_some()
        ),
    }

    let decoded = decoded.map(CachedRoute::from);
    match (cached, decoded) {
        (None, None) => {}
        (Some(cached), Some(decoded)) => {
            assert_eq!(
                cached.classifier_id, decoded.classifier_id,
                "{lane} cached classifier ID differs at call {index}"
            );
            assert_eq!(
                cached.provenance.depth, decoded.provenance.depth,
                "{lane} cached fallback depth differs at call {index}"
            );
            assert_eq!(
                cached.provenance.fallback, decoded.provenance.fallback,
                "{lane} cached fallback index differs at call {index}"
            );
            assert_eq!(
                cached.provenance.rebases, decoded.provenance.rebases,
                "{lane} cached typed-rebase count differs at call {index}"
            );
            assert_eq!(
                cached.pattern_id, decoded.pattern_id,
                "{lane} cached pattern ID differs at call {index}"
            );
            assert_eq!(
                cached.table_id, decoded.table_id,
                "{lane} cached rule-table ID differs at call {index}"
            );
            assert_eq!(
                cached.fast, decoded.fast,
                "{lane} cached route stability differs at call {index}"
            );
        }
        (cached, decoded) => panic!(
            "{lane} cached route presence differs at call {index}: cached={}, decoded={}",
            cached.is_some(),
            decoded.is_some()
        ),
    }
}

#[cfg(test)]
pub(crate) fn assert_step_cache_projection_parity(
    partnership: &crate::bidding::book::Partnership,
    vul: contract_bridge::auction::RelativeVulnerability,
    auction: &[Call],
    cache: &mut AuthoringStepCache,
) -> bool {
    let expected = project_authored_legacy(&partnership.prefixed_context(vul, auction));
    let old_phase = cache.phase;
    let old_record_count = cache.records.len();
    let Some(actual) = cache.prepare(partnership, vul, auction).cloned() else {
        return false;
    };
    let first_new = if old_phase == cache.phase {
        old_record_count
    } else {
        0
    };
    cache.assert_new_route_records_match_one_shot(partnership, vul, auction, first_new);
    assert_eq!(
        actual.unions, expected.unions,
        "step-cache projection differs"
    );
    assert_eq!(
        actual.announced_unions, expected.announced_unions,
        "step-cache announcement differs"
    );
    assert_eq!(actual.masks, expected.masks, "step-cache call masks differ");
    true
}

fn project_authored_with(context: &Context<'_>, compiled_reader: bool) -> AuthoredProjection {
    let auction = context.auction();
    let len = auction.len();
    let mut projection = AuthoredProjection::unknown();

    let Some(prefixes) = context.prefixes() else {
        return projection;
    };

    let profile = context.reading_profile();
    let read_passes = profile.pass;
    let table_alerts = profile.table_alerts;
    let fallback_projection = profile.fallback_projection;
    let announce_split = profile.announced;
    let declared_opponent_all = profile.scope == ReadingScope::All
        && context
            .own_system()
            .is_some_and(crate::bidding::book::Partnership::has_declared_opponents);

    // Project the call made at `index`, authored by `classifier`, into the
    // overlay, evaluating its rules under `ctx` — always the bidder's
    // at-the-time context, for our own pair's calls as much as for
    // table-decoded opponent calls (see the `at_the_time` builder below).
    // `decode_pass` scopes the pass reading to the side whose
    // book resolved `classifier`: the own-side loops resolve *every* index in
    // the reader's trie, where an opponent's pass would land on the wrong
    // node (their direct-seat pass over our opening resolves our *responses*
    // table) — alerts are shielded by the alert filter, passes need the
    // explicit side gate.
    let mut project_call = |ctx: &Context<'_>,
                            index: usize,
                            classifier: &dyn crate::bidding::trie::Classifier,
                            compiled: Option<&crate::bidding::rules::CompiledRules>,
                            decode_pass: bool| {
        let Some(&made) = auction.get(index) else {
            return;
        };
        if let Some(effect) =
            authored_effect(made, ctx, classifier, compiled, decode_pass, announce_split)
        {
            projection.apply(len, index, effect, profile);
        }
    };

    // A rule's constraint is a claim about the moment its call was made, so it
    // must project under the bidder's **at-the-time** context — exactly as the
    // table-alert and pass walks below already do.  Projecting under the
    // reader's full-auction context mis-resolved every auction-relative atom:
    // `support(3..)` reads `partner_last_suit()`, and the reader's "partner's
    // last bid" *is the projected call itself*, so a cue raise's support atom
    // stamped n+ cards of the **cue suit** — a wrong box that excluded the
    // bidder (probe-reading-sound: `1♥ (1♠) 2♠` 8/15,
    // `1♣ (1♦) 2♦` 4/4).  The
    // same skew put the support double's 3-card claim on opener's own suit.
    // For plain raises the two contexts happen to agree (the raise suit is the
    // support suit), which is how this survived the raise-reader sweeps.
    // Resolve each enabled authoring route in one forward scan.  `own` deliberately
    // uses the reader's final-phase book and full-auction guard context, just
    // like the historical first pass.  `routed` uses the phase of each actor's
    // turn and their at-the-time context; the opponent-alert and Pass passes
    // consume the same answer later without resolving the prefix again.  Keep
    // the scans separate and in legacy category order: besides avoiding work
    // when a reading mode is disabled, public opaque guards may be stateful.
    // Each at-the-time context inherits the reader's pinned knob state: they
    // carry no partnership of their own, so without the pin they would decode under
    // the *thread's* live knobs (the shipped defaults on a rayon worker).
    let pin = context.decision_profile();
    let at_times: Vec<Context<'_>> = if compiled_reader {
        Context::at_each_turn(context.vul(), auction)
    } else {
        (0..=len)
            .map(|index| {
                let vul = if index % 2 == len % 2 {
                    context.vul()
                } else {
                    crate::bidding::context::flipped(context.vul())
                };
                Context::new(vul, &auction[..index])
            })
            .collect()
    }
    .into_iter()
    .map(|at| at.with_profile(pin))
    .collect();
    let partnership = compiled_reader.then(|| context.own_system()).flatten();
    // The opponents' books — ours unless the partnership declares an opponent
    // ([`Partnership::with_opponents`]).  Every decode below picks by the *side that made the
    // call*: our own calls resolve in `partnership`, theirs in `their_partnership`.
    let their_partnership = compiled_reader.then(|| context.their_system()).flatten();

    let decoded_own = if fallback_projection {
        if let Some(partnership) = partnership {
            let mut own_cursor = partnership.decoder_for(auction).cursor_with_capacity(len);
            let mut own = Vec::with_capacity(len);
            for index in 0..len {
                let prefix = &auction[..index];
                match own_cursor.resolve_checked(context, prefix) {
                    crate::bidding::decoder::CheckedResolution::Decoded(answer) => own.push(answer),
                    crate::bidding::decoder::CheckedResolution::Opaque => {
                        return project_authored_with(context, false);
                    }
                }
            }
            Some(own)
        } else {
            None
        }
    } else {
        None
    };
    let decode_routed = table_alerts || read_passes || declared_opponent_all;
    let decoded_routed = if decode_routed {
        if let (Some(partnership), Some(their_partnership)) = (partnership, their_partnership) {
            // Six cursors, `[side][phase]`: a declared opponent decodes their
            // calls in their own books, so the routed walk cannot share one
            // cursor set across the table.  Each already sees a *subsequence*
            // of the prefixes (the `!needed` skip below), so splitting one
            // more way costs nothing but the slots.
            let mut cursors = [partnership, their_partnership].map(|books| {
                [
                    crate::bidding::book::Phase::Constructive,
                    crate::bidding::book::Phase::Competitive,
                    crate::bidding::book::Phase::Defensive,
                ]
                .map(|phase| books.decoder_for_phase(phase).cursor_with_capacity(len))
            });
            let mut routed = Vec::with_capacity(len);
            for index in 0..len {
                let prefix = &auction[..index];
                let opponent = index % 2 != len % 2;
                let needed = ((table_alerts || declared_opponent_all) && opponent)
                    || (read_passes && auction[index] == Call::Pass && (!opponent || table_alerts));
                if !needed {
                    routed.push(None);
                    continue;
                }
                let phase = match crate::bidding::book::Phase::of(prefix) {
                    crate::bidding::book::Phase::Constructive => 0,
                    crate::bidding::book::Phase::Competitive => 1,
                    crate::bidding::book::Phase::Defensive => 2,
                };
                let resolution =
                    cursors[usize::from(opponent)][phase].resolve_checked(&at_times[index], prefix);
                match resolution {
                    crate::bidding::decoder::CheckedResolution::Decoded(answer) => {
                        routed.push(answer)
                    }
                    crate::bidding::decoder::CheckedResolution::Opaque => {
                        return project_authored_with(context, false);
                    }
                }
            }
            Some(routed)
        } else {
            None
        }
    } else {
        None
    };

    if fallback_projection {
        // Decode every prior call by the classifier that *authored* it — node or
        // guarded fallback — so contested conventions (transfers, Leaping Michaels,
        // the Lebensohl cue) survive later competition without a per-convention reader.
        if let Some(own) = &decoded_own {
            for (index, answer) in own.iter().enumerate() {
                if profile.scope == ReadingScope::All && index % 2 != len % 2 {
                    continue;
                }
                let Some(answer) = answer else { continue };
                let classifier = answer.classifier;
                let compiled = context.own_system().and_then(|partnership| {
                    partnership.compiled_rules_for(auction, classifier, profile)
                });
                project_call(&at_times[index], index, classifier, compiled, false);
            }
        } else {
            let trie = prefixes.root();
            for index in 0..len {
                if profile.scope == ReadingScope::All && index % 2 != len % 2 {
                    continue;
                }
                if let Some(classifier) = trie.authoring_classifier(context, &auction[..index]) {
                    project_call(&at_times[index], index, classifier, None, false);
                }
            }
        }
    } else {
        // Exact-node classifiers only (fallback projection, the shipped
        // default, takes the branch above); fallback-authored conventions are
        // then read by the hand-written readers in [`Inferences::read`].
        for (prefix, classifier) in prefixes.clone() {
            if profile.scope == ReadingScope::All && prefix.len() % 2 != len % 2 {
                continue;
            }
            let compiled = compiled_reader
                .then(|| context.own_system())
                .flatten()
                .and_then(|partnership| {
                    partnership.compiled_rules_for(auction, classifier, profile)
                });
            project_call(
                &at_times[prefix.len()],
                prefix.len(),
                classifier,
                compiled,
                false,
            );
        }
    }

    // Alerts are table-wide disclosure: the opponents' alerted calls are
    // explained to us, so decode them too — each resolved in *their*
    // phase-routed book (declared with [`Partnership::with_opponents`], else our own books
    // model them) under their at-the-time context, exactly as it was
    // classified when made.  Their unalerted (natural) calls keep the natural
    // walk.
    if (table_alerts || declared_opponent_all)
        && let Some(them) = context.their_system()
    {
        for index in ((len + 1) % 2..len).step_by(2) {
            let prefix = &auction[..index];
            let at_time = if declared_opponent_all {
                at_times[index].clone()
            } else {
                let mut decision = context.decision_profile();
                decision.reading.scope = ReadingScope::Alerted;
                at_times[index].clone().with_profile(decision)
            };
            let answer = decoded_routed.as_ref().and_then(|routed| routed[index]);
            if let Some(answer) = answer {
                let classifier = answer.classifier;
                let compiled =
                    them.compiled_rules_for(prefix, classifier, at_time.reading_profile());
                project_call(&at_time, index, classifier, compiled, false);
            } else if decoded_routed.is_none()
                && let Some(classifier) =
                    them.trie_for(prefix).authoring_classifier(&at_time, prefix)
            {
                project_call(&at_time, index, classifier, None, false);
            }
        }
    }

    // Passes decode in a dedicated walk: a pass's authoring node lives in the
    // phase of *its* turn (a pre-opening pass belongs to the opening table
    // even after the auction goes defensive), so each resolves via the
    // slice-relative `trie_for` on the auction cut at its index, under its
    // at-the-time context — the reader's own side always, the opponents' only
    // under table-wide disclosure.  Each pass resolves in the books of the
    // side that *made* it, which are the same books twice unless the partnership
    // declares an opponent ([`Partnership::with_opponents`]).
    if read_passes
        && let (Some(ours), Some(theirs)) = (context.own_system(), context.their_system())
    {
        for index in 0..len {
            if auction[index] != Call::Pass {
                continue;
            }
            let own_side = index % 2 == len % 2;
            if !own_side && !table_alerts {
                continue;
            }
            let them = if own_side { ours } else { theirs };
            let prefix = &auction[..index];
            let answer = decoded_routed.as_ref().and_then(|routed| routed[index]);
            if let Some(answer) = answer {
                let classifier = answer.classifier;
                let compiled = them.compiled_rules_for(prefix, classifier, profile);
                project_call(&at_times[index], index, classifier, compiled, true);
            } else if decoded_routed.is_none()
                && let Some(classifier) = them
                    .trie_for(prefix)
                    .authoring_classifier(&at_times[index], prefix)
            {
                project_call(&at_times[index], index, classifier, None, true);
            }
        }
    }

    // The probed overlay (see `ReadingProfile::probed`): fold each prior call's
    // behaviorally measured box, keyed by the prefix through it.  Runs last so
    // it composes with — never replaces — the symbolic folds above; an empty
    // probed map is a no-op.  The vacuous-scoped variant lives in
    // [`Inferences::read`] instead: its fill-only mask must be judged against
    // the *complete* symbolic reading, and the natural walk stamps after this
    // returns.
    if profile.probed
        && let (Some(ours), Some(theirs)) = (context.own_system(), context.their_system())
    {
        for index in 0..len {
            let them = if index % 2 == len % 2 { ours } else { theirs };
            if let Some(&box_) = them.probed_box(&auction[..=index]) {
                let who = relative_of(len, index) as usize;
                let union = EnvelopeUnion::from(box_);
                projection.unions[who].intersect_assign(&union, profile);
                projection.announced_unions[who].intersect_assign(&union, profile);
            }
        }
    }

    projection
}

/// Whether a call's projection floors a suit other than the one it names
///
/// The structural artificial-call detector, falling out of the projection itself:
/// a natural call floors its own strain (1♠ → 5+♠) or no suit (1NT → points only);
/// an artificial one floors a suit it did not name (Jacoby 2♦ → 5+♥, Landy 2♣ →
/// 4-4 majors).  A min-length floor of four-plus on a non-named suit is the witness
/// — above any natural by-product, below every convention's real shape.
///
/// **The "named" suit generalizes past bids.**  A call is natural when it offers to
/// play what it declares, artificial when it points partner at some *other* suit:
/// - a **bid** names its own strain;
/// - a **double / redouble** names the *doubled strain* — the contract it offers to
///   defend.  A penalty double floors that strain (or nothing) → natural; a takeout
///   double floors an *unbid* suit (support for where it sends partner) → artificial;
/// - a **pass** redirects from nothing → never artificial (a trap pass defends what
///   is on the table);
/// - a **transfer completion** names the suit it will play, flooring no other →
///   natural, so already `false`.
///
/// This is a *sound sufficient* witness, not a complete one: a takeout double whose
/// authoring rule floors nothing (opaque shape predicates, e.g. the direct takeout
/// double) reads `false` here though it is takeout by meaning — such calls are
/// classified artificial by their `.alert(...)` instead, exactly as shape-only
/// artificial bids are.
///
/// **Retired from the decode gate** — alerts now carry the signal exhaustively.
/// This survives test-only, as the `artificial_calls_are_alerted` invariant guard:
/// any future artificial call added without an `.alert(...)` must fail that test
/// rather than silently lose its decoding.
#[cfg(test)]
pub(crate) fn artificial(projection: &Envelope, made: Call, doubled: Option<Strain>) -> bool {
    // The "named" suit is the one the call offers to play: a bid names its own
    // strain; a double/redouble "names" the doubled strain it offers to defend.
    // Artificial = the projection floors some *other* suit ≥4 — the call is really
    // pointing partner at a suit it did not name.  A pass redirects from nothing.
    let named = match made {
        Call::Bid(bid) => bid.strain.suit(),
        Call::Double | Call::Redouble => doubled.and_then(|strain| strain.suit()),
        Call::Pass => return false,
    };
    Suit::ASC
        .into_iter()
        .any(|suit| Some(suit) != named && projection.length(suit).min >= 4)
}

#[cfg(test)]
mod tests;
