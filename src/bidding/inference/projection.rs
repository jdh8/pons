//! Rule projection: decoding an alerted call back into boxes
//!
//! Where the natural walk in [`super::read`] guesses a call's meaning from its
//! shape, this module replays the *authoring rule's own* `project` fold, so a
//! convention reads exactly what its rules promise.  [`AuthoringStepCache`] is
//! the per-auction memo that keeps the replay affordable.

use super::envelope::{Envelope, EnvelopeUnion, relative_of};
use super::knobs::*;
use crate::bidding::context::Context;
use contract_bridge::auction::Call;
#[cfg(test)]
use contract_bridge::{Strain, Suit};

/// One table's pass reading: the union of its Pass rules' bands, knob-on
/// intersected with the complements of the sibling gates the passer declined
///
/// The exclusion half (see [`set_pass_exclusion_reading`]) leans on argmax
/// selection: a hand inside a sibling gate whose weight strictly beats
/// **every** Pass rule's weight could not have let Pass win, so the passer
/// lies in that gate's complement.  Single-box complements only — the
/// shape-free tiers; skipping the rest costs precision, never soundness, and
/// holds the box count down.  [`None`] when the table authors no Pass rule at
/// all (the projection pass then records nothing, as before).
#[inline]
pub(super) fn project_pass<'a>(
    rules: &crate::bidding::rules::Rules,
    compiled: Option<&'a crate::bidding::rules::CompiledRules>,
    ctx: &Context<'_>,
    profile: ReadingProfile,
) -> Option<crate::bidding::rules::ProjectedUnion<'a>> {
    let band = if let Some(compiled) = compiled {
        compiled
            .pass_rule_indices()
            .iter()
            .map(|&index| compiled.project_band_union_matched(rules, index, ctx))
            .reduce(|a, b| a.disjoin(b, profile))?
    } else {
        crate::bidding::rules::ProjectedUnion::Owned(
            rules
                .rules()
                .iter()
                .filter(|rule| rule.call() == Call::Pass)
                .map(|rule| rule.project_band_union(ctx))
                .reduce(|a, b| a.disjoin_with(b, profile))?,
        )
    };
    if !profile.pass_exclusion_reading() {
        return Some(band);
    }
    if let Some(compiled) = compiled {
        let pass = compiled
            .pass_plan()
            .expect("Pass indices imply a Pass plan");
        return Some(crate::bidding::rules::ProjectedUnion::Owned(
            pass.stronger_nonpass_indices()
                .iter()
                .map(|&index| compiled.project_complement_union_matched(rules, index, ctx))
                .filter(|complement| {
                    complement.as_union().boxes().len() == 1
                        && complement.as_union().boxes()[0] != Envelope::unknown()
                })
                .fold(band.into_owned(), |acc, complement| {
                    acc.intersect_owned(complement.as_union(), profile)
                }),
        ));
    }
    let ceiling = rules
        .rules()
        .iter()
        .filter(|rule| rule.call() == Call::Pass)
        .map(crate::bidding::rules::Rule::weight)
        .max()
        .unwrap_or(i16::MIN);
    Some(crate::bidding::rules::ProjectedUnion::Owned(
        rules
            .rules()
            .iter()
            .filter(|rule| rule.call() != Call::Pass && rule.weight() > ceiling)
            .map(|rule| rule.project_complement_union(ctx))
            .filter(|complement| {
                complement.boxes().len() == 1 && complement.boxes()[0] != Envelope::unknown()
            })
            .fold(band.into_owned(), |acc, complement| {
                acc.intersect_owned(&complement, profile)
            }),
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoredProjection {
    unions: [EnvelopeUnion; 4],
    announced_unions: [EnvelopeUnion; 4],
    suppressed: u64,
}

impl AuthoredProjection {
    fn unknown() -> Self {
        Self {
            unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            announced_unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            suppressed: 0,
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
        if effect.suppresses_natural && index < 64 {
            self.suppressed |= 1 << index;
        }
    }

    fn into_parts(self) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
        (self.unions, self.announced_unions, self.suppressed)
    }

    fn cloned_parts(&self) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
        (
            self.unions.clone(),
            self.announced_unions.clone(),
            self.suppressed,
        )
    }
}

struct AuthoredEffect<'a> {
    projection: crate::bidding::rules::ProjectedUnion<'a>,
    agreement: Option<crate::bidding::rules::ProjectedUnion<'a>>,
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
            if !decode_pass && compiled.can_skip_pass_effect(profile.pass_exclusion_reading()) {
                return None;
            }
        } else {
            match scope.expect("non-pass reading scope") {
                ReadingScope::None if compiled.can_skip_nonpass_effect(made) => return None,
                ReadingScope::Alerted
                    if compiled.alerted_rule_indices(made).is_empty()
                        && compiled.can_skip_nonpass_effect(made) =>
                {
                    return None;
                }
                ReadingScope::None | ReadingScope::Alerted | ReadingScope::All => {}
            }
        }
    }

    let projection = if is_pass {
        project_pass(rules, compiled, ctx, profile)
    } else if let Some(compiled) = compiled {
        compiled
            .rule_indices(made)
            .iter()
            .filter(|&&index| compiled.face_live_memoized(rules, index, ctx, &mut face_memo))
            .map(|&index| compiled.project_union_matched(rules, index, ctx))
            .reduce(|a, b| a.disjoin(b, profile))
    } else {
        rules
            .rules()
            .iter()
            .filter(|rule| rule.call() == made && rule.face_live(ctx))
            .map(|rule| crate::bidding::rules::ProjectedUnion::Owned(rule.project_union(ctx)))
            .reduce(|a, b| a.disjoin(b, profile))
    }?;

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
        if let Some(compiled) = compiled {
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
                .map(|rule| crate::bidding::rules::ProjectedUnion::Owned(rule.announce_union(ctx)))
                .reduce(|a, b| a.disjoin(b, profile))
        }
    } else {
        None
    };
    Some(AuthoredEffect {
        projection,
        agreement,
        suppresses_natural: alerted,
    })
}

#[derive(Clone, Debug)]
struct AbsoluteProjection {
    unions: [EnvelopeUnion; 4],
    announced_unions: [EnvelopeUnion; 4],
    suppressed: u64,
}

impl AbsoluteProjection {
    fn unknown() -> Self {
        Self {
            unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            announced_unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            suppressed: 0,
        }
    }

    fn push(&mut self, index: usize, effect: AuthoredEffect<'_>, profile: ReadingProfile) {
        let seat = index % 4;
        self.unions[seat].intersect_assign(effect.projection.as_union(), profile);
        self.announced_unions[seat].intersect_assign(effect.agreement(), profile);
        if effect.suppresses_natural && index < 64 {
            self.suppressed |= 1 << index;
        }
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
        || compiled.is_some_and(|plan| {
            plan.can_reuse_authored_effect(made, profile.pass_exclusion_reading())
        })
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
    stance_identity: Option<std::sync::Arc<crate::bidding::book::StanceCacheIdentity>>,
    profile: Option<ReadingProfile>,
    vul: Option<contract_bridge::auction::RelativeVulnerability>,
    reader_parity: Option<usize>,
    phase: Option<crate::bidding::book::Phase>,
    auction: Vec<Call>,
    context_cursor: crate::bidding::context::ContextCursor,
    own_cursor: crate::bidding::decoder::DecoderCursorState,
    routed_cursors: [crate::bidding::decoder::DecoderCursorState; 3],
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
            stance_identity: None,
            profile: None,
            vul: None,
            reader_parity: None,
            phase: None,
            auction: Vec::new(),
            context_cursor: crate::bidding::context::ContextCursor::new(),
            own_cursor: crate::bidding::decoder::DecoderCursorState::default(),
            routed_cursors: std::array::from_fn(|_| {
                crate::bidding::decoder::DecoderCursorState::default()
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
        stance: &crate::bidding::book::Stance,
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
                    stance.decoder_for_phase(phase),
                    stance.trie_for(auction),
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
                assert_cached_route_matches_one_shot(
                    "routed",
                    index,
                    record.routed,
                    stance.decoder_for(prefix),
                    stance.trie_for(prefix),
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
        self.routed_cursors =
            std::array::from_fn(|_| crate::bidding::decoder::DecoderCursorState::default());
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
        stance: &crate::bidding::book::Stance,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
    ) -> Option<&'a AuthoredProjection> {
        if self.disabled {
            return None;
        }
        // ponytail: a declared opponent ([`Stance::with_opponents`]) falls back to the
        // full-auction reader, which already routes each call to the books of
        // the side that made it.  Splitting this incremental walk's `routed`
        // cursors the same way is straightforward — six slots instead of
        // three — but a mixed table is an A/B instrument, not a hot path.
        if !std::ptr::eq(stance.opponents(), stance) {
            return self.disable();
        }
        match &self.stance_identity {
            None => self.stance_identity = Some(std::sync::Arc::clone(stance.cache_identity())),
            Some(identity) if std::sync::Arc::ptr_eq(identity, stance.cache_identity()) => {}
            Some(_) => return self.disable(),
        }
        // The stance's pinned reading, not the thread's live one: this walk
        // serves that stance, and `Stance::repin` invalidates the cache
        // identity checked just above, so a deliberate re-pin still resets it.
        let profile = stance.profile().reading;
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
        let announce_split = profile.announced_reading();
        let read_passes = profile.pass_reading();
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
            std::array::from_fn(|_| crate::bidding::decoder::DecoderCursorState::default())
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

            let own_decoder = stance.decoder_for_phase(phase);
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
                stance.trie_for(auction).get(prefix)
            };
            let own_classifier = own_answer.map(|answer| answer.classifier).or(own_exact);
            let own_compiled = own_classifier
                .and_then(|classifier| stance.compiled_rules_for(auction, classifier, profile));
            if own_classifier.is_some_and(|classifier| {
                !authored_effect_is_reusable(made, classifier, own_compiled, profile)
            }) {
                return self.disable();
            }

            let opponent = index % 2 != reader_parity;
            let routed_needed = (table_alerts && opponent)
                || (read_passes && made == Call::Pass && (!opponent || table_alerts));
            let routed_phase = scan_context_cursor.phase();
            let routed_decoder = stance.decoder_for_phase(routed_phase);
            let routed_answer = if routed_needed {
                match routed_decoder.resolve_checked_with_cursor(
                    &mut scan_routed_cursors[Self::phase_index(routed_phase)],
                    &at_time,
                    prefix,
                ) {
                    crate::bidding::decoder::CheckedResolution::Decoded(answer) => answer,
                    crate::bidding::decoder::CheckedResolution::Opaque => return self.disable(),
                }
            } else {
                None
            };
            if routed_needed && !scan_routed_cursors[Self::phase_index(routed_phase)].cache_stable()
            {
                return self.disable();
            }
            let routed_compiled = routed_answer
                .and_then(|answer| stance.compiled_rules_for(prefix, answer.classifier, profile));
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

            if let Some(answer) = pending.own {
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
            } else if !fallback_projection
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
                        &at_time,
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
                && let Some(&box_) = stance.probed_box(&auction[..=index])
            {
                let union = EnvelopeUnion::from(box_);
                self.probed.push(
                    index,
                    AuthoredEffect {
                        projection: crate::bidding::rules::ProjectedUnion::Owned(union),
                        agreement: None,
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
                snapshot.suppressed |= category.suppressed;
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

pub(super) fn project_authored(
    context: &Context<'_>,
) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
    if let Some(projection) = context.authored_projection() {
        return projection.cloned_parts();
    }
    project_authored_with(context, true)
}

/// Same-process semantic oracle retained for compilation parity tests.
#[cfg(test)]
fn project_authored_legacy(context: &Context<'_>) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
    project_authored_with(context, false)
}

#[cfg(test)]
pub(crate) fn assert_compiled_authoring_projection_parity(context: &Context<'_>) {
    let compiled = project_authored(context);
    let legacy = project_authored_legacy(context);
    assert_eq!(compiled.0, legacy.0, "authored projection boxes differ");
    assert_eq!(compiled.1, legacy.1, "authored announcement boxes differ");
    assert_eq!(compiled.2, legacy.2, "authored suppression mask differs");
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
    stance: &crate::bidding::book::Stance,
    vul: contract_bridge::auction::RelativeVulnerability,
    auction: &[Call],
    cache: &mut AuthoringStepCache,
) -> bool {
    let expected = project_authored_legacy(&stance.prefixed_context(vul, auction));
    let old_phase = cache.phase;
    let old_record_count = cache.records.len();
    let Some(actual) = cache.prepare(stance, vul, auction).cloned() else {
        return false;
    };
    let first_new = if old_phase == cache.phase {
        old_record_count
    } else {
        0
    };
    cache.assert_new_route_records_match_one_shot(stance, vul, auction, first_new);
    assert_eq!(actual.unions, expected.0, "step-cache projection differs");
    assert_eq!(
        actual.announced_unions, expected.1,
        "step-cache announcement differs"
    );
    assert_eq!(
        actual.suppressed, expected.2,
        "step-cache suppression differs"
    );
    true
}

fn project_authored_with(
    context: &Context<'_>,
    compiled_reader: bool,
) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
    let auction = context.auction();
    let len = auction.len();
    let mut projection = AuthoredProjection::unknown();

    let Some(prefixes) = context.prefixes() else {
        return projection.into_parts();
    };

    let profile = context.reading_profile();
    let read_passes = profile.pass_reading();
    let table_alerts = profile.table_alerts;
    let fallback_projection = profile.fallback_projection;
    let announce_split = profile.announced_reading();

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
    let at_times = if compiled_reader {
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
    };
    let stance = compiled_reader.then(|| context.own_system()).flatten();
    // The opponents' books — ours unless the stance declares an opponent
    // ([`Stance::with_opponents`]).  Every decode below picks by the *side that made the
    // call*: our own calls resolve in `stance`, theirs in `their_stance`.
    let their_stance = compiled_reader.then(|| context.their_system()).flatten();

    let decoded_own = if fallback_projection {
        if let Some(stance) = stance {
            let mut own_cursor = stance.decoder_for(auction).cursor_with_capacity(len);
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
    let decode_routed = table_alerts || read_passes;
    let decoded_routed = if decode_routed {
        if let (Some(stance), Some(their_stance)) = (stance, their_stance) {
            // Six cursors, `[side][phase]`: a declared opponent decodes their
            // calls in their own books, so the routed walk cannot share one
            // cursor set across the table.  Each already sees a *subsequence*
            // of the prefixes (the `!needed` skip below), so splitting one
            // more way costs nothing but the slots.
            let mut cursors = [stance, their_stance].map(|books| {
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
                let needed = (table_alerts && opponent)
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
                let Some(answer) = answer else { continue };
                let classifier = answer.classifier;
                let compiled = context
                    .own_system()
                    .and_then(|stance| stance.compiled_rules_for(auction, classifier, profile));
                project_call(&at_times[index], index, classifier, compiled, false);
            }
        } else {
            let trie = prefixes.root();
            for index in 0..len {
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
            let compiled = compiled_reader
                .then(|| context.own_system())
                .flatten()
                .and_then(|stance| stance.compiled_rules_for(auction, classifier, profile));
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
    // phase-routed book (declared with [`Stance::with_opponents`], else our own books
    // model them) under their at-the-time context, exactly as it was
    // classified when made.  Their unalerted (natural) calls keep the natural
    // walk.
    if table_alerts && let Some(them) = context.their_system() {
        for index in ((len + 1) % 2..len).step_by(2) {
            let prefix = &auction[..index];
            let answer = decoded_routed.as_ref().and_then(|routed| routed[index]);
            if let Some(answer) = answer {
                let classifier = answer.classifier;
                let compiled = them.compiled_rules_for(prefix, classifier, profile);
                project_call(&at_times[index], index, classifier, compiled, false);
            } else if decoded_routed.is_none()
                && let Some(classifier) = them
                    .trie_for(prefix)
                    .authoring_classifier(&at_times[index], prefix)
            {
                project_call(&at_times[index], index, classifier, None, false);
            }
        }
    }

    // Passes decode in a dedicated walk: a pass's authoring node lives in the
    // phase of *its* turn (a pre-opening pass belongs to the opening table
    // even after the auction goes defensive), so each resolves via the
    // slice-relative `trie_for` on the auction cut at its index, under its
    // at-the-time context — the reader's own side always, the opponents' only
    // under table-wide disclosure.  Each pass resolves in the books of the
    // side that *made* it, which are the same books twice unless the stance
    // declares an opponent ([`Stance::with_opponents`]).
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

    // The probed overlay (see [`set_probed_reading`]): fold each prior call's
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

    projection.into_parts()
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
