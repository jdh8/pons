//! Finalized reader-side routing for authored classifiers
//!
//! [`Trie`][super::trie::Trie] stays mutable and authoritative while a pair is
//! assembled.  Once bound, this module snapshots its mechanics into sparse,
//! immutable nodes and attaches the preserved row grammar to each surviving
//! site.  Resolution deliberately mirrors `Trie::resolve_at`; the first gate
//! is pointer/provenance parity, before inference uses the forward cursor.

use super::context::Context;
use super::fallback::{Fallback, Guard, GuardPlan, Rewrite, RewritePlan};
#[cfg(test)]
use super::rows::{AuthoringCatalog, Placement};
use super::rows::{
    AuthoringLedger, LedgerPattern, PatternGrammar, PatternId, PreservedTarget,
    PreservedTargetKind, RuleTableId,
};
use super::trie::{Classifier, Provenance, REBASE_LIMIT, Trie, TrieNode};
use contract_bridge::Bid;
use contract_bridge::auction::Call;
use std::collections::HashMap;
use std::sync::Arc;

type NodeId = u32;
type PoolId = u32;

const NONE: PoolId = PoolId::MAX;

#[derive(Clone, Debug)]
struct SiteMetadata {
    pattern_id: PatternId,
    table_id: Option<RuleTableId>,
    grammar: PatternGrammar,
    opaque_target: bool,
}

#[derive(Clone, Debug)]
enum DecoderGuard {
    Always,
    Undisturbed,
    First(Call),
    UpTo(Bid),
    Suffix(Box<[Call]>),
    Opaque(Arc<dyn Guard>),
}

impl DecoderGuard {
    fn compile(guard: &Arc<dyn Guard>, metadata: Option<&SiteMetadata>) -> Self {
        if let Some(metadata) = metadata {
            return match &metadata.grammar {
                PatternGrammar::Exact => Self::from_plan(guard, guard.plan()),
                PatternGrammar::First(call) => Self::First(*call),
                PatternGrammar::UpTo(bid) => Self::UpTo(*bid),
                PatternGrammar::Suffix(suffix) => Self::Suffix(suffix.clone()),
                PatternGrammar::Opaque { .. } => Self::Opaque(Arc::clone(guard)),
            };
        }
        Self::from_plan(guard, guard.plan())
    }

    fn from_plan(guard: &Arc<dyn Guard>, plan: GuardPlan) -> Self {
        match plan {
            GuardPlan::Always => Self::Always,
            GuardPlan::Undisturbed => Self::Undisturbed,
            GuardPlan::FirstIs(call) => Self::First(call),
            GuardPlan::OvercallAtMost(bid) => Self::UpTo(bid),
            GuardPlan::SuffixIs(suffix) => Self::Suffix(suffix.into_boxed_slice()),
            GuardPlan::Opaque => Self::Opaque(Arc::clone(guard)),
        }
    }

    fn admits(&self, context: &Context<'_>, suffix: &[Call]) -> bool {
        match self {
            Self::Always => true,
            Self::Undisturbed => context.undisturbed(),
            Self::First(call) => suffix.first() == Some(call),
            Self::UpTo(bound) => matches!(suffix, [Call::Bid(bid)] if bid <= bound),
            Self::Suffix(expected) => suffix == &**expected,
            Self::Opaque(guard) => guard.admits(context, suffix),
        }
    }

    const fn fast(&self) -> bool {
        !matches!(self, Self::Undisturbed | Self::Opaque(_))
    }
}

#[derive(Clone, Debug)]
enum DecoderAction {
    Classify(PoolId),
    Rebase {
        rewrite: Arc<dyn Rewrite>,
        plan: RewritePlan,
    },
}

impl DecoderAction {
    fn compile(
        fallback: &Fallback,
        classifiers: &mut Vec<Arc<dyn Classifier>>,
        classifier_ids: &mut HashMap<usize, PoolId>,
    ) -> Self {
        match fallback {
            Fallback::Classify(classifier) => {
                Self::Classify(intern_classifier(classifier, classifiers, classifier_ids))
            }
            Fallback::Rebase(rewrite) => Self::Rebase {
                rewrite: Arc::clone(rewrite),
                plan: rewrite.plan(),
            },
        }
    }

    const fn fast(&self) -> bool {
        match self {
            Self::Classify(_) => true,
            Self::Rebase { plan, .. } => !matches!(plan, RewritePlan::Opaque),
        }
    }
}

#[derive(Clone, Debug)]
struct DecoderFallback {
    guard: DecoderGuard,
    action: DecoderAction,
    metadata: PoolId,
}

#[derive(Clone, Debug)]
struct PendingFallback {
    guard: Arc<dyn Guard>,
    action: DecoderAction,
    metadata: PoolId,
}

#[derive(Clone, Copy, Debug)]
struct DecoderNode {
    edge_start: u32,
    edge_len: u32,
    fallback_start: u32,
    fallback_len: u32,
    classifier: PoolId,
    exact_metadata: PoolId,
}

impl Default for DecoderNode {
    fn default() -> Self {
        Self {
            edge_start: 0,
            edge_len: 0,
            fallback_start: 0,
            fallback_len: 0,
            classifier: NONE,
            exact_metadata: NONE,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DecoderEdge {
    call: Call,
    node: NodeId,
}

fn intern_classifier(
    classifier: &Arc<dyn Classifier>,
    classifiers: &mut Vec<Arc<dyn Classifier>>,
    classifier_ids: &mut HashMap<usize, PoolId>,
) -> PoolId {
    let key = Arc::as_ptr(classifier) as *const () as usize;
    if let Some(&id) = classifier_ids.get(&key) {
        return id;
    }
    let id = PoolId::try_from(classifiers.len()).expect("too many authoring classifiers");
    assert_ne!(id, NONE, "too many authoring classifiers");
    classifiers.push(Arc::clone(classifier));
    classifier_ids.insert(key, id);
    id
}

fn checked_pool_id(len: usize, what: &str) -> PoolId {
    let id = PoolId::try_from(len).unwrap_or_else(|_| panic!("too many {what}"));
    assert_ne!(id, NONE, "too many {what}");
    id
}

fn child_at(
    nodes: &[DecoderNode],
    edges: &[DecoderEdge],
    node: NodeId,
    call: Call,
) -> Option<NodeId> {
    let node = nodes.get(node as usize)?;
    let start = node.edge_start as usize;
    let end = start + node.edge_len as usize;
    edges[start..end]
        .iter()
        .find_map(|edge| (edge.call == call).then_some(edge.node))
}

fn node_for_key(nodes: &[DecoderNode], edges: &[DecoderEdge], key: &[Call]) -> Option<NodeId> {
    (!nodes.is_empty()).then_some(())?;
    let mut node = 0;
    for &call in key {
        node = child_at(nodes, edges, node, call)?;
    }
    Some(node)
}

fn metadata_at(metadata: &[SiteMetadata], id: PoolId) -> Option<&SiteMetadata> {
    (id != NONE).then(|| &metadata[id as usize])
}

/// Immutable sparse authoring decoder compiled at whole-book finalization
#[derive(Clone, Debug, Default)]
pub(crate) struct AuthoringDecoder {
    nodes: Box<[DecoderNode]>,
    edges: Box<[DecoderEdge]>,
    fallbacks: Box<[DecoderFallback]>,
    classifiers: Box<[Arc<dyn Classifier>]>,
    metadata: Box<[SiteMetadata]>,
    #[allow(dead_code)] // retained for the decoder coverage census
    opaque_sites: usize,
    opaque_routes: bool,
}

struct DecoderBuilder {
    nodes: Vec<DecoderNode>,
    edges: Vec<DecoderEdge>,
    fallbacks: Vec<PendingFallback>,
    classifiers: Vec<Arc<dyn Classifier>>,
    metadata: Vec<SiteMetadata>,
}

impl DecoderBuilder {
    fn flatten(trie: &Trie) -> Self {
        // Node IDs are assigned breadth-first, so every node owns one
        // contiguous edge and fallback range without a box allocation of its
        // own.
        let mut source_nodes = vec![trie.root_node()];
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut fallbacks = Vec::new();
        let mut classifiers = Vec::new();
        let mut classifier_ids = HashMap::new();
        let mut cursor = 0;

        while cursor < source_nodes.len() {
            let source: &TrieNode = source_nodes[cursor];
            let edge_start = checked_pool_id(edges.len(), "decoder edges");
            for (call, child) in &source.children {
                let child_id = checked_pool_id(source_nodes.len(), "decoder nodes");
                source_nodes.push(child.as_ref());
                edges.push(DecoderEdge {
                    call,
                    node: child_id,
                });
            }
            let edge_len = checked_pool_id(edges.len() - edge_start as usize, "edges at one node");

            let fallback_start = checked_pool_id(fallbacks.len(), "decoder fallbacks");
            for (guard, fallback) in &source.fallbacks {
                fallbacks.push(PendingFallback {
                    guard: Arc::clone(guard),
                    action: DecoderAction::compile(fallback, &mut classifiers, &mut classifier_ids),
                    metadata: NONE,
                });
            }
            let fallback_len = checked_pool_id(
                fallbacks.len() - fallback_start as usize,
                "fallbacks at one node",
            );
            let classifier = source.classify.as_ref().map_or(NONE, |classifier| {
                intern_classifier(classifier, &mut classifiers, &mut classifier_ids)
            });
            nodes.push(DecoderNode {
                edge_start,
                edge_len,
                fallback_start,
                fallback_len,
                classifier,
                exact_metadata: NONE,
            });
            cursor += 1;
        }

        Self {
            nodes,
            edges,
            fallbacks,
            classifiers,
            metadata: Vec::new(),
        }
    }

    fn push_metadata(
        &mut self,
        pattern_id: PatternId,
        table_id: Option<RuleTableId>,
        grammar: PatternGrammar,
        target: PreservedTargetKind,
    ) -> PoolId {
        let metadata_id = checked_pool_id(self.metadata.len(), "authoring metadata entries");
        self.metadata.push(SiteMetadata {
            pattern_id,
            table_id,
            grammar,
            opaque_target: target == PreservedTargetKind::Computed,
        });
        metadata_id
    }

    #[cfg(test)]
    fn attach_catalog(&mut self, catalog: &AuthoringCatalog) {
        for bound in catalog.patterns() {
            let metadata_id = self.push_metadata(
                bound.id,
                bound.table_id,
                bound.grammar.clone(),
                bound.target,
            );
            for site in &bound.sites {
                let Some(node_id) = node_for_key(&self.nodes, &self.edges, &site.key) else {
                    debug_assert!(false, "finalized authoring site is absent from decoder");
                    continue;
                };
                let node = &mut self.nodes[node_id as usize];
                match site.placement {
                    Placement::Exact => {
                        if node.exact_metadata == NONE {
                            node.exact_metadata = metadata_id;
                        }
                    }
                    Placement::Fallback { index } => {
                        let Ok(index) = u32::try_from(index) else {
                            debug_assert!(false, "fallback index exceeds decoder range");
                            continue;
                        };
                        if index >= node.fallback_len {
                            debug_assert!(false, "finalized fallback site is absent from decoder");
                            continue;
                        }
                        let slot = (node.fallback_start + index) as usize;
                        if self.fallbacks[slot].metadata == NONE {
                            self.fallbacks[slot].metadata = metadata_id;
                        }
                    }
                }
            }
        }
    }

    fn ledger_metadata(
        &mut self,
        pattern: &LedgerPattern,
        by_id: &mut HashMap<PatternId, PoolId>,
    ) -> PoolId {
        if let Some(&metadata_id) = by_id.get(&pattern.id) {
            return metadata_id;
        }
        let metadata_id = self.push_metadata(
            pattern.id,
            pattern.table_id,
            pattern.grammar.clone(),
            pattern.target.kind(),
        );
        by_id.insert(pattern.id, metadata_id);
        metadata_id
    }

    fn attach_ledger(&mut self, ledger: AuthoringLedger) {
        let mut by_id = HashMap::<PatternId, PoolId>::new();

        // Ledger order is declaration order. Assigning only when the exact Arc
        // tuple is still live preserves overwrite/collision behavior while
        // letting each row's owned keys drop immediately after this iteration.
        for pattern in ledger.into_patterns() {
            for key in &pattern.keys {
                let Some(node_id) = node_for_key(&self.nodes, &self.edges, key) else {
                    continue;
                };
                let node_index = node_id as usize;

                if pattern.guard.is_none() {
                    let node = &self.nodes[node_index];
                    let Some(expected) = pattern.target.classifier() else {
                        continue;
                    };
                    if node.classifier == NONE
                        || !Arc::ptr_eq(&self.classifiers[node.classifier as usize], expected)
                    {
                        continue;
                    }
                    let metadata_id = self.ledger_metadata(&pattern, &mut by_id);
                    if self.nodes[node_index].exact_metadata == NONE {
                        self.nodes[node_index].exact_metadata = metadata_id;
                    }
                    continue;
                }

                let expected_guard = pattern.guard.as_ref().expect("guarded pattern");
                let node = self.nodes[node_index];
                let start = node.fallback_start as usize;
                let end = start + node.fallback_len as usize;
                for slot in start..end {
                    let fallback = &self.fallbacks[slot];
                    if !Arc::ptr_eq(&fallback.guard, expected_guard) {
                        continue;
                    }
                    let target_matches = match (&pattern.target, &fallback.action) {
                        (
                            PreservedTarget::Rules(expected) | PreservedTarget::Computed(expected),
                            DecoderAction::Classify(actual),
                        ) => Arc::ptr_eq(expected, &self.classifiers[*actual as usize]),
                        (
                            PreservedTarget::Rebase(expected),
                            DecoderAction::Rebase {
                                rewrite: actual, ..
                            },
                        ) => Arc::ptr_eq(expected, actual),
                        _ => false,
                    };
                    if !target_matches {
                        continue;
                    }
                    let metadata_id = self.ledger_metadata(&pattern, &mut by_id);
                    if self.fallbacks[slot].metadata == NONE {
                        self.fallbacks[slot].metadata = metadata_id;
                    }
                }
            }
        }
    }

    fn finish(self) -> AuthoringDecoder {
        let Self {
            nodes,
            edges,
            fallbacks: pending_fallbacks,
            classifiers,
            metadata,
        } = self;
        let mut opaque_sites = 0;
        let mut opaque_routes = false;
        let fallbacks: Vec<_> = pending_fallbacks
            .into_iter()
            .map(|pending| {
                let site = metadata_at(&metadata, pending.metadata);
                let guard = DecoderGuard::compile(&pending.guard, site);
                opaque_routes |= matches!(&guard, DecoderGuard::Opaque(_))
                    || matches!(
                        &pending.action,
                        DecoderAction::Rebase {
                            plan: RewritePlan::Opaque,
                            ..
                        }
                    );
                if !guard.fast()
                    || !pending.action.fast()
                    || site.is_some_and(|site| site.opaque_target)
                {
                    opaque_sites += 1;
                }
                DecoderFallback {
                    guard,
                    action: pending.action,
                    metadata: pending.metadata,
                }
            })
            .collect();
        AuthoringDecoder {
            nodes: nodes.into_boxed_slice(),
            edges: edges.into_boxed_slice(),
            fallbacks: fallbacks.into_boxed_slice(),
            classifiers: classifiers.into_boxed_slice(),
            metadata: metadata.into_boxed_slice(),
            opaque_sites,
            opaque_routes,
        }
    }
}

/// One decoder answer, including the identity/provenance parity surface
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DecoderClassifierId(PoolId);

#[derive(Clone, Copy, Debug)]
pub(crate) struct DecodedAuthoring<'a> {
    pub(crate) classifier: &'a dyn Classifier,
    pub(crate) classifier_id: DecoderClassifierId,
    pub(crate) provenance: Provenance,
    pub(crate) pattern_id: Option<PatternId>,
    pub(crate) table_id: Option<RuleTableId>,
    /// False when resolution touched an opaque guard/rewrite/computed table.
    pub(crate) fast: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum CheckedResolution<'a> {
    Decoded(Option<DecodedAuthoring<'a>>),
    Opaque,
}

impl AuthoringDecoder {
    pub(crate) fn classifiers(&self) -> impl Iterator<Item = &dyn Classifier> {
        self.classifiers.iter().map(AsRef::as_ref)
    }

    #[cfg(test)]
    pub(crate) fn compile(trie: &Trie, catalog: &AuthoringCatalog) -> Self {
        let mut decoder = DecoderBuilder::flatten(trie);
        decoder.attach_catalog(catalog);
        decoder.finish()
    }

    /// Compile directly from the consumed build-time ledger.
    ///
    /// Unlike [`Self::compile`], this production path never materializes a
    /// catalog of cloned concrete keys. It validates each ledger key against
    /// the already-flat authoritative trie and drops it as the stream advances.
    pub(crate) fn compile_ledger(trie: &Trie, ledger: AuthoringLedger) -> Self {
        let mut decoder = DecoderBuilder::flatten(trie);
        decoder.attach_ledger(ledger);
        decoder.finish()
    }

    #[cfg(test)]
    pub(crate) fn resolve<'a>(
        &'a self,
        context: &Context<'_>,
        auction: &[Call],
    ) -> Option<DecodedAuthoring<'a>> {
        let path = self.path(auction);
        let mut cache_stable = true;
        self.resolve_at(context, auction, &path, 0, false, &mut cache_stable)
    }

    #[cfg(test)]
    pub(crate) fn cursor(&self) -> DecoderCursor<'_> {
        self.cursor_with_capacity(0)
    }

    pub(crate) fn cursor_with_capacity(&self, auction_len: usize) -> DecoderCursor<'_> {
        let mut path = Vec::with_capacity(auction_len + 1);
        path.push(0);
        DecoderCursor {
            decoder: self,
            depth: 0,
            path,
        }
    }

    #[cfg(test)]
    pub(crate) fn resolve_with_cursor<'a>(
        &'a self,
        state: &mut DecoderCursorState,
        context: &Context<'_>,
        prefix: &[Call],
    ) -> Option<DecodedAuthoring<'a>> {
        state.sync(self, prefix);
        let mut cache_stable = true;
        let found = self.resolve_at(context, prefix, &state.path, 0, false, &mut cache_stable);
        state.cache_stable = cache_stable;
        found
    }

    pub(crate) fn resolve_checked_with_cursor<'a>(
        &'a self,
        state: &mut DecoderCursorState,
        context: &Context<'_>,
        prefix: &[Call],
    ) -> CheckedResolution<'a> {
        state.sync(self, prefix);
        let mut cache_stable = true;
        let found = self.resolve_checked_incremental(state, context, prefix, &mut cache_stable);
        state.cache_stable = cache_stable && !matches!(found, CheckedResolution::Opaque);
        found
    }

    #[cfg(test)]
    pub(crate) const fn opaque_sites(&self) -> usize {
        self.opaque_sites
    }

    fn path(&self, auction: &[Call]) -> Vec<NodeId> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        let mut path = Vec::with_capacity(auction.len() + 1);
        let mut node = 0;
        path.push(node);
        for &call in auction {
            let Some(child) = child_at(&self.nodes, &self.edges, node, call) else {
                break;
            };
            node = child;
            path.push(node);
        }
        path
    }

    fn resolve_at<'a>(
        &'a self,
        context: &Context<'_>,
        auction: &[Call],
        path: &[NodeId],
        rebases: usize,
        skip_exact: bool,
        cache_stable: &mut bool,
    ) -> Option<DecodedAuthoring<'a>> {
        let &deepest = path.last()?;
        let deepest_node = &self.nodes[deepest as usize];
        if !skip_exact && path.len() == auction.len() + 1 && deepest_node.classifier != NONE {
            let classifier = self.classifiers[deepest_node.classifier as usize].as_ref();
            let metadata = metadata_at(&self.metadata, deepest_node.exact_metadata);
            if metadata.is_some_and(|site| site.opaque_target) {
                *cache_stable = false;
            }
            return Some(DecodedAuthoring {
                classifier,
                classifier_id: DecoderClassifierId(deepest_node.classifier),
                provenance: Provenance {
                    depth: auction.len(),
                    fallback: None,
                    rebases,
                },
                pattern_id: metadata.map(|site| site.pattern_id),
                table_id: metadata.and_then(|site| site.table_id),
                fast: *cache_stable,
            });
        }

        for (depth, &node_id) in path.iter().enumerate().rev() {
            let node = &self.nodes[node_id as usize];
            let start = node.fallback_start as usize;
            let end = start + node.fallback_len as usize;
            for (index, fallback) in self.fallbacks[start..end].iter().enumerate() {
                // Even a rejected dynamic guard makes the answer unsafe to reuse:
                // a later full context can change which route wins.
                if !fallback.guard.fast() {
                    *cache_stable = false;
                }
                if !fallback.guard.admits(context, &auction[depth..]) {
                    continue;
                }
                let metadata = metadata_at(&self.metadata, fallback.metadata);
                if metadata.is_some_and(|site| site.opaque_target) {
                    *cache_stable = false;
                }
                match &fallback.action {
                    DecoderAction::Classify(classifier) => {
                        return Some(DecodedAuthoring {
                            classifier: self.classifiers[*classifier as usize].as_ref(),
                            classifier_id: DecoderClassifierId(*classifier),
                            provenance: Provenance {
                                depth,
                                fallback: Some(index),
                                rebases,
                            },
                            pattern_id: metadata.map(|site| site.pattern_id),
                            table_id: metadata.and_then(|site| site.table_id),
                            fast: *cache_stable,
                        });
                    }
                    DecoderAction::Rebase { rewrite, plan } => {
                        if matches!(plan, RewritePlan::Opaque) {
                            *cache_stable = false;
                        }
                        if rebases < REBASE_LIMIT
                            && let Some(rewritten) = rewrite.rewrite(auction, depth)
                        {
                            let rewritten_path = self.path(&rewritten);
                            if let Some(found) = self.resolve_at(
                                context,
                                &rewritten,
                                &rewritten_path,
                                rebases + 1,
                                false,
                                cache_stable,
                            ) {
                                return Some(found);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn resolve_checked_at<'a>(
        &'a self,
        context: &Context<'_>,
        auction: &[Call],
        path: &[NodeId],
        rebases: usize,
        skip_exact: bool,
        cache_stable: &mut bool,
    ) -> CheckedResolution<'a> {
        let Some(&deepest) = path.last() else {
            return CheckedResolution::Decoded(None);
        };
        let deepest_node = &self.nodes[deepest as usize];
        if !skip_exact && path.len() == auction.len() + 1 && deepest_node.classifier != NONE {
            let classifier = self.classifiers[deepest_node.classifier as usize].as_ref();
            let metadata = metadata_at(&self.metadata, deepest_node.exact_metadata);
            if metadata.is_some_and(|site| site.opaque_target) {
                *cache_stable = false;
            }
            return CheckedResolution::Decoded(Some(DecodedAuthoring {
                classifier,
                classifier_id: DecoderClassifierId(deepest_node.classifier),
                provenance: Provenance {
                    depth: auction.len(),
                    fallback: None,
                    rebases,
                },
                pattern_id: metadata.map(|site| site.pattern_id),
                table_id: metadata.and_then(|site| site.table_id),
                fast: *cache_stable,
            }));
        }

        for (depth, &node_id) in path.iter().enumerate().rev() {
            let node = &self.nodes[node_id as usize];
            let start = node.fallback_start as usize;
            let end = start + node.fallback_len as usize;
            for (index, fallback) in self.fallbacks[start..end].iter().enumerate() {
                if matches!(&fallback.guard, DecoderGuard::Opaque(_)) {
                    return CheckedResolution::Opaque;
                }
                if !fallback.guard.fast() {
                    *cache_stable = false;
                }
                if !fallback.guard.admits(context, &auction[depth..]) {
                    continue;
                }
                let metadata = metadata_at(&self.metadata, fallback.metadata);
                if metadata.is_some_and(|site| site.opaque_target) {
                    *cache_stable = false;
                }
                match &fallback.action {
                    DecoderAction::Classify(classifier) => {
                        return CheckedResolution::Decoded(Some(DecodedAuthoring {
                            classifier: self.classifiers[*classifier as usize].as_ref(),
                            classifier_id: DecoderClassifierId(*classifier),
                            provenance: Provenance {
                                depth,
                                fallback: Some(index),
                                rebases,
                            },
                            pattern_id: metadata.map(|site| site.pattern_id),
                            table_id: metadata.and_then(|site| site.table_id),
                            fast: *cache_stable,
                        }));
                    }
                    DecoderAction::Rebase { rewrite, plan } => {
                        if rebases >= REBASE_LIMIT {
                            continue;
                        }
                        if matches!(plan, RewritePlan::Opaque) {
                            return CheckedResolution::Opaque;
                        }
                        if let Some(rewritten) = rewrite.rewrite(auction, depth) {
                            let rewritten_path = self.path(&rewritten);
                            match self.resolve_checked_at(
                                context,
                                &rewritten,
                                &rewritten_path,
                                rebases + 1,
                                false,
                                cache_stable,
                            ) {
                                CheckedResolution::Decoded(Some(found)) => {
                                    return CheckedResolution::Decoded(Some(found));
                                }
                                CheckedResolution::Opaque => return CheckedResolution::Opaque,
                                CheckedResolution::Decoded(None) => {}
                            }
                        }
                    }
                }
            }
        }
        CheckedResolution::Decoded(None)
    }

    /// Resolve from an append-only cursor without walking every trie ancestor.
    ///
    /// Candidate-bearing depths are maintained by [`IncrementalRoutes`].  A
    /// structured rebase deliberately re-enters the ordinary checked resolver:
    /// its rewritten key is a distinct path, and rebase recursion remains
    /// capped by [`REBASE_LIMIT`].
    fn resolve_checked_incremental<'a>(
        &'a self,
        state: &mut DecoderCursorState,
        context: &Context<'_>,
        auction: &[Call],
        cache_stable: &mut bool,
    ) -> CheckedResolution<'a> {
        let Some(&deepest) = state.path.last() else {
            return CheckedResolution::Decoded(None);
        };
        let deepest_node = &self.nodes[deepest as usize];
        if state.path.len() == auction.len() + 1 && deepest_node.classifier != NONE {
            let classifier = self.classifiers[deepest_node.classifier as usize].as_ref();
            let metadata = metadata_at(&self.metadata, deepest_node.exact_metadata);
            if metadata.is_some_and(|site| site.opaque_target) {
                *cache_stable = false;
            }
            return CheckedResolution::Decoded(Some(DecodedAuthoring {
                classifier,
                classifier_id: DecoderClassifierId(deepest_node.classifier),
                provenance: Provenance {
                    depth: auction.len(),
                    fallback: None,
                    rebases: 0,
                },
                pattern_id: metadata.map(|site| site.pattern_id),
                table_id: metadata.and_then(|site| site.table_id),
                fast: *cache_stable,
            }));
        }

        let routes = &mut state.routes;
        let mut active_index = routes.active_depths.len();
        let mut transient_index = 0usize;
        while active_index > 0 || transient_index < routes.current.len() {
            let active_depth = active_index
                .checked_sub(1)
                .map(|index| routes.active_depths[index]);
            let transient_depth = routes.current.get(transient_index).map(|route| route.depth);
            let depth = match (active_depth, transient_depth) {
                (Some(active), Some(transient)) => active.max(transient),
                (Some(active), None) => active,
                (None, Some(transient)) => transient,
                (None, None) => break,
            };
            #[cfg(test)]
            {
                routes.depth_visits += 1;
            }

            let active = if active_depth == Some(depth) {
                routes.active[depth].as_slice()
            } else {
                &[]
            };
            let transient_end = if transient_depth == Some(depth) {
                let mut end = transient_index + 1;
                while end < routes.current.len() && routes.current[end].depth == depth {
                    end += 1;
                }
                end
            } else {
                transient_index
            };
            let transient = &routes.current[transient_index..transient_end];

            let mut active_slot = 0usize;
            let mut transient_slot = 0usize;
            while active_slot < active.len() || transient_slot < transient.len() {
                let fallback = match (
                    active.get(active_slot).copied(),
                    transient.get(transient_slot).map(|route| route.fallback),
                ) {
                    (Some(active), Some(transient)) if active <= transient => {
                        active_slot += 1;
                        active
                    }
                    (Some(_), Some(transient)) => {
                        transient_slot += 1;
                        transient
                    }
                    (Some(active), None) => {
                        active_slot += 1;
                        active
                    }
                    (None, Some(transient)) => {
                        transient_slot += 1;
                        transient
                    }
                    (None, None) => break,
                };
                let node_id = state.path[depth];
                let node = &self.nodes[node_id as usize];
                let fallback_index = fallback as usize - node.fallback_start as usize;
                match self.resolve_checked_fallback(
                    context,
                    auction,
                    ResolvedRoute {
                        depth,
                        fallback,
                        fallback_index,
                    },
                    0,
                    cache_stable,
                ) {
                    CheckedResolution::Decoded(None) => {}
                    found => return found,
                }
            }

            if active_depth == Some(depth) {
                active_index -= 1;
            }
            if transient_depth == Some(depth) {
                transient_index = transient_end;
            }
        }
        CheckedResolution::Decoded(None)
    }

    fn resolve_checked_fallback<'a>(
        &'a self,
        context: &Context<'_>,
        auction: &[Call],
        route: ResolvedRoute,
        rebases: usize,
        cache_stable: &mut bool,
    ) -> CheckedResolution<'a> {
        let fallback = &self.fallbacks[route.fallback as usize];
        if matches!(&fallback.guard, DecoderGuard::Opaque(_)) {
            return CheckedResolution::Opaque;
        }
        if !fallback.guard.fast() {
            *cache_stable = false;
        }
        if !fallback.guard.admits(context, &auction[route.depth..]) {
            return CheckedResolution::Decoded(None);
        }
        let metadata = metadata_at(&self.metadata, fallback.metadata);
        if metadata.is_some_and(|site| site.opaque_target) {
            *cache_stable = false;
        }
        match &fallback.action {
            DecoderAction::Classify(classifier) => {
                CheckedResolution::Decoded(Some(DecodedAuthoring {
                    classifier: self.classifiers[*classifier as usize].as_ref(),
                    classifier_id: DecoderClassifierId(*classifier),
                    provenance: Provenance {
                        depth: route.depth,
                        fallback: Some(route.fallback_index),
                        rebases,
                    },
                    pattern_id: metadata.map(|site| site.pattern_id),
                    table_id: metadata.and_then(|site| site.table_id),
                    fast: *cache_stable,
                }))
            }
            DecoderAction::Rebase { rewrite, plan } => {
                if rebases >= REBASE_LIMIT {
                    return CheckedResolution::Decoded(None);
                }
                if matches!(plan, RewritePlan::Opaque) {
                    return CheckedResolution::Opaque;
                }
                let Some(rewritten) = rewrite.rewrite(auction, route.depth) else {
                    return CheckedResolution::Decoded(None);
                };
                let rewritten_path = self.path(&rewritten);
                self.resolve_checked_at(
                    context,
                    &rewritten,
                    &rewritten_path,
                    rebases + 1,
                    false,
                    cache_stable,
                )
            }
        }
    }
}

/// Incremental exact-transition cursor for one forward auction scan
pub(crate) struct DecoderCursor<'a> {
    decoder: &'a AuthoringDecoder,
    depth: usize,
    path: Vec<NodeId>,
}

/// Borrow-free forward-cursor state suitable for a deal-owned cache.
#[derive(Clone, Debug)]
pub(crate) struct DecoderCursorState {
    prefix: Vec<Call>,
    path: Vec<NodeId>,
    routes: IncrementalRoutes,
    cache_stable: bool,
}

/// One fallback whose guard can become relevant at a known prefix length.
///
/// `First` candidates become persistent after their first uncovered call;
/// `UpTo` and `Suffix` candidates are eligible for exactly one prefix length.
#[derive(Clone, Copy, Debug)]
struct ScheduledRoute {
    depth: usize,
    fallback: PoolId,
    persistent: bool,
}

#[derive(Clone, Copy, Debug)]
struct ResolvedRoute {
    depth: usize,
    fallback: PoolId,
    fallback_index: usize,
}

/// Incremental fallback dispatch for a deal-owned forward cursor.
///
/// The trie path itself is already advanced one edge per appended call.  This
/// sidecar does the same for guarded fallbacks: persistent candidates are
/// registered once at their node, while exact-length guards are placed in a
/// due-length bucket.  Resolution consequently visits candidate-bearing
/// depths, not every ancestor of every prefix.
#[derive(Clone, Debug, Default)]
struct IncrementalRoutes {
    /// Persistent fallback slots, indexed by trie depth and sorted in
    /// declaration order.
    active: Vec<Vec<PoolId>>,
    /// Non-empty entries in `active`, monotonically increasing by depth.
    active_depths: Vec<usize>,
    /// First/UpTo/Suffix activations keyed by the only length at which their
    /// suffix first becomes decidable.
    due: Vec<Vec<ScheduledRoute>>,
    /// Admitted UpTo/Suffix routes for the currently synced prefix.
    current: Vec<ScheduledRoute>,
    #[cfg(test)]
    depth_visits: usize,
}

impl IncrementalRoutes {
    fn clear(&mut self) {
        self.active.clear();
        self.active_depths.clear();
        self.due.clear();
        self.current.clear();
        #[cfg(test)]
        {
            self.depth_visits = 0;
        }
    }

    fn activate(&mut self, depth: usize, fallback: PoolId) {
        if self.active.len() <= depth {
            self.active.resize_with(depth + 1, Vec::new);
        }
        let slots = &mut self.active[depth];
        if slots.is_empty() {
            debug_assert!(self.active_depths.last().is_none_or(|&last| last < depth));
            self.active_depths.push(depth);
        }
        match slots.binary_search(&fallback) {
            Ok(_) => {}
            Err(index) => slots.insert(index, fallback),
        }
    }

    fn schedule(&mut self, due: usize, route: ScheduledRoute, current_len: usize) {
        if due == current_len {
            self.current.push(route);
            return;
        }
        if self.due.len() <= due {
            self.due.resize_with(due + 1, Vec::new);
        }
        self.due[due].push(route);
    }

    fn enter_node(
        &mut self,
        decoder: &AuthoringDecoder,
        node_id: NodeId,
        depth: usize,
        current_len: usize,
    ) {
        let node = &decoder.nodes[node_id as usize];
        let start = node.fallback_start as usize;
        let end = start + node.fallback_len as usize;
        for (offset, fallback) in decoder.fallbacks[start..end].iter().enumerate() {
            let slot = PoolId::try_from(start + offset).expect("decoder fallback pool overflow");
            match &fallback.guard {
                DecoderGuard::Always | DecoderGuard::Undisturbed | DecoderGuard::Opaque(_) => {
                    self.activate(depth, slot);
                }
                DecoderGuard::First(_) => self.schedule(
                    depth + 1,
                    ScheduledRoute {
                        depth,
                        fallback: slot,
                        persistent: true,
                    },
                    current_len,
                ),
                DecoderGuard::UpTo(_) => self.schedule(
                    depth + 1,
                    ScheduledRoute {
                        depth,
                        fallback: slot,
                        persistent: false,
                    },
                    current_len,
                ),
                DecoderGuard::Suffix(expected) => self.schedule(
                    depth + expected.len(),
                    ScheduledRoute {
                        depth,
                        fallback: slot,
                        persistent: false,
                    },
                    current_len,
                ),
            }
        }
    }

    fn advance(&mut self, decoder: &AuthoringDecoder, prefix: &[Call]) {
        self.current.clear();
        let len = prefix.len();
        if len >= self.due.len() {
            return;
        }
        let scheduled = core::mem::take(&mut self.due[len]);
        for route in scheduled {
            let fallback = &decoder.fallbacks[route.fallback as usize];
            let suffix = &prefix[route.depth..];
            let admitted = match &fallback.guard {
                DecoderGuard::First(call) => suffix.first() == Some(call),
                DecoderGuard::UpTo(bound) => {
                    matches!(suffix, [Call::Bid(bid)] if bid <= bound)
                }
                DecoderGuard::Suffix(expected) => suffix == &**expected,
                DecoderGuard::Always | DecoderGuard::Undisturbed | DecoderGuard::Opaque(_) => {
                    debug_assert!(false, "persistent guard entered the route scheduler");
                    false
                }
            };
            if !admitted {
                continue;
            }
            if route.persistent {
                self.activate(route.depth, route.fallback);
            } else {
                self.current.push(route);
            }
        }
        self.current.sort_unstable_by(|left, right| {
            right
                .depth
                .cmp(&left.depth)
                .then_with(|| left.fallback.cmp(&right.fallback))
        });
    }

    #[cfg(test)]
    const fn depth_visits(&self) -> usize {
        self.depth_visits
    }
}

impl DecoderCursorState {
    fn new() -> Self {
        Self {
            prefix: Vec::new(),
            // The deal cache moves live cursors out transactionally.  Keep
            // the replacement state allocation-free; the first actual sync
            // installs the root and then reuses that buffer for the deal.
            path: Vec::new(),
            routes: IncrementalRoutes::default(),
            cache_stable: true,
        }
    }

    /// Whether the most recent resolution avoided every context-dynamic route.
    pub(crate) const fn cache_stable(&self) -> bool {
        self.cache_stable
    }

    fn sync(&mut self, decoder: &AuthoringDecoder, prefix: &[Call]) {
        if self.path.is_empty() {
            if decoder.nodes.is_empty() {
                self.prefix.clear();
                self.prefix.extend_from_slice(prefix);
                return;
            }
            self.path.push(0);
            self.routes.enter_node(decoder, 0, 0, 0);
        }
        if prefix == self.prefix {
            return;
        }
        if prefix.len() > self.prefix.len() && prefix.starts_with(&self.prefix) {
            for &call in &prefix[self.prefix.len()..] {
                let old_len = self.prefix.len();
                let followed_entire_prefix = self.path.len() == old_len + 1;
                let child = followed_entire_prefix
                    .then(|| self.path.last().copied())
                    .flatten()
                    .and_then(|node| child_at(&decoder.nodes, &decoder.edges, node, call));
                self.prefix.push(call);
                self.routes.advance(decoder, &self.prefix);
                if let Some(child) = child {
                    let depth = self.prefix.len();
                    self.path.push(child);
                    self.routes.enter_node(decoder, child, depth, depth);
                    self.routes.current.sort_unstable_by(|left, right| {
                        right
                            .depth
                            .cmp(&left.depth)
                            .then_with(|| left.fallback.cmp(&right.fallback))
                    });
                }
            }
            return;
        }
        self.path.clear();
        self.prefix.clear();
        self.routes.clear();
        self.sync(decoder, prefix);
    }
}

impl Default for DecoderCursorState {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DecoderCursor<'a> {
    fn sync(&mut self, prefix: &[Call]) {
        if prefix.len() < self.depth {
            self.path = self.decoder.path(prefix);
        } else if prefix.len() > self.depth {
            for &call in &prefix[self.depth..] {
                if self.path.len() == self.depth + 1
                    && let Some(&node) = self.path.last()
                    && let Some(child) =
                        child_at(&self.decoder.nodes, &self.decoder.edges, node, call)
                {
                    self.path.push(child);
                }
                self.depth += 1;
            }
        }
        self.depth = prefix.len();
    }

    #[cfg(test)]
    pub(crate) fn resolve(
        &mut self,
        context: &Context<'_>,
        prefix: &[Call],
    ) -> Option<DecodedAuthoring<'a>> {
        self.sync(prefix);
        let mut cache_stable = true;
        self.decoder
            .resolve_at(context, prefix, &self.path, 0, false, &mut cache_stable)
    }

    pub(crate) fn resolve_checked(
        &mut self,
        context: &Context<'_>,
        prefix: &[Call],
    ) -> CheckedResolution<'a> {
        self.sync(prefix);
        let mut cache_stable = true;
        if self.decoder.opaque_routes {
            self.decoder.resolve_checked_at(
                context,
                prefix,
                &self.path,
                0,
                false,
                &mut cache_stable,
            )
        } else {
            CheckedResolution::Decoded(self.decoder.resolve_at(
                context,
                prefix,
                &self.path,
                0,
                false,
                &mut cache_stable,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::array::Logits;
    use crate::bidding::constraint::hcp;
    use crate::bidding::fallback::{Always, FirstIs, OvercallAtMost, ReplaceNext, SuffixIs};
    use crate::bidding::rows::{Entry, Package, Pattern, compile_into, rows_of};
    use crate::bidding::rules::Rules;
    use crate::bidding::trie::classifier;
    use contract_bridge::Strain;
    use contract_bridge::auction::RelativeVulnerability;

    fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    fn marker(id: u8) -> Arc<dyn Classifier> {
        Arc::new(classifier(move |_, _| {
            let _ = id;
            Logits::new()
        }))
    }

    fn compile(trie: &Trie) -> AuthoringDecoder {
        AuthoringDecoder::compile(trie, &trie.finalize_authoring())
    }

    fn assert_same_resolution(
        trie: &Trie,
        decoder: &AuthoringDecoder,
        context: &Context<'_>,
        prefix: &[Call],
    ) {
        let expected = trie.resolve(context, prefix);
        let actual = decoder.resolve(context, prefix);
        match (expected, actual) {
            (None, None) => {}
            (Some((expected_classifier, expected_provenance)), Some(actual)) => {
                assert!(
                    core::ptr::eq(expected_classifier, actual.classifier),
                    "classifier identity differs at {prefix:?}",
                );
                assert_eq!(
                    actual.provenance, expected_provenance,
                    "provenance differs at {prefix:?}",
                );
            }
            (expected, actual) => panic!(
                "resolution presence differs at {prefix:?}: trie={}, decoder={}",
                expected.is_some(),
                actual.is_some(),
            ),
        }
    }

    fn assert_every_prefix(trie: &Trie, decoder: &AuthoringDecoder, auction: &[Call]) {
        for depth in 0..=auction.len() {
            let prefix = &auction[..depth];
            let context = Context::new(RelativeVulnerability::NONE, prefix);
            assert_same_resolution(trie, decoder, &context, prefix);
        }
    }

    #[test]
    fn exact_and_structured_guards_match_trie() {
        let exact = marker(1);
        let first = marker(2);
        let up_to = marker(3);
        let suffix = marker(4);
        let undisturbed = marker(5);
        let mut trie = Trie::new();

        let exact_key = [bid(1, Strain::Clubs), Call::Pass];
        trie.insert_arc(&exact_key, Arc::clone(&exact));

        let first_key = [bid(1, Strain::Diamonds)];
        trie.fallback_at(
            &first_key,
            FirstIs(Call::Double),
            Fallback::Classify(Arc::clone(&first)),
        );

        let up_to_key = [bid(1, Strain::Hearts)];
        trie.fallback_at(
            &up_to_key,
            OvercallAtMost(Bid::new(2, Strain::Spades)),
            Fallback::Classify(Arc::clone(&up_to)),
        );

        let suffix_key = [bid(1, Strain::Spades)];
        trie.fallback_at(
            &suffix_key,
            SuffixIs(vec![Call::Double, Call::Pass]),
            Fallback::Classify(Arc::clone(&suffix)),
        );

        let undisturbed_key = [bid(1, Strain::Notrump)];
        trie.fallback_at(
            &undisturbed_key,
            super::super::fallback::Undisturbed,
            Fallback::Classify(Arc::clone(&undisturbed)),
        );

        let decoder = compile(&trie);
        let first_auction = [first_key[0], Call::Double, Call::Pass];
        let up_to_auction = [up_to_key[0], bid(2, Strain::Hearts)];
        let suffix_auction = [suffix_key[0], Call::Double, Call::Pass];
        let undisturbed_auction = [undisturbed_key[0], Call::Pass];

        for auction in [
            exact_key.as_slice(),
            first_auction.as_slice(),
            up_to_auction.as_slice(),
            suffix_auction.as_slice(),
            undisturbed_auction.as_slice(),
        ] {
            assert_every_prefix(&trie, &decoder, auction);
        }

        let exact_context = Context::new(RelativeVulnerability::NONE, &exact_key);
        let decoded = decoder
            .resolve(&exact_context, &exact_key)
            .expect("exact classifier");
        assert!(core::ptr::eq(decoded.classifier, exact.as_ref()));
        assert_eq!(
            decoded.provenance,
            Provenance {
                depth: exact_key.len(),
                fallback: None,
                rebases: 0,
            },
        );

        let disturbed_auction = [undisturbed_key[0], Call::Double];
        let disturbed_context = Context::new(RelativeVulnerability::NONE, &disturbed_auction);
        assert!(!disturbed_context.undisturbed());
        assert_same_resolution(&trie, &decoder, &disturbed_context, &disturbed_auction);
        assert!(
            decoder
                .resolve(&disturbed_context, &disturbed_auction)
                .is_none()
        );
    }

    #[test]
    fn opaque_guard_matches_trie() {
        let classified = marker(10);
        let key = [bid(2, Strain::Clubs)];
        let auction = [key[0], Call::Redouble, Call::Pass];
        let mut trie = Trie::new();
        trie.fallback_at(
            &key,
            super::super::fallback::guard(|_: &Context<'_>, suffix: &[Call]| {
                suffix.first() == Some(&Call::Redouble)
            }),
            Fallback::Classify(Arc::clone(&classified)),
        );

        let decoder = compile(&trie);
        assert_eq!(decoder.opaque_sites(), 1);
        assert_every_prefix(&trie, &decoder, &auction);

        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let decoded = decoder.resolve(&context, &auction).expect("opaque guard");
        assert!(core::ptr::eq(decoded.classifier, classified.as_ref()));
        assert_eq!(
            decoded.provenance,
            Provenance {
                depth: key.len(),
                fallback: Some(0),
                rebases: 0,
            },
        );
        assert!(!decoded.fast);
    }

    #[test]
    fn typed_and_opaque_rebases_match_trie() {
        let typed_target = marker(20);
        let opaque_target = marker(21);
        let typed_key = [bid(2, Strain::Diamonds)];
        let opaque_key = [bid(2, Strain::Hearts)];
        let typed_query = [typed_key[0], Call::Double, Call::Pass];
        let opaque_query = [opaque_key[0], Call::Double, Call::Pass];
        let mut trie = Trie::new();

        trie.insert_arc(
            &[typed_key[0], Call::Pass, Call::Pass],
            Arc::clone(&typed_target),
        );
        trie.fallback_at(
            &typed_key,
            FirstIs(Call::Double),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );

        trie.insert_arc(
            &[opaque_key[0], Call::Pass, Call::Pass],
            Arc::clone(&opaque_target),
        );
        trie.fallback_at(
            &opaque_key,
            FirstIs(Call::Double),
            Fallback::rebase(super::super::fallback::rewriter(
                |auction: &[Call], depth: usize| {
                    (depth < auction.len()).then(|| {
                        let mut rewritten = auction.to_vec();
                        rewritten[depth] = Call::Pass;
                        rewritten
                    })
                },
            )),
        );

        let decoder = compile(&trie);
        assert_eq!(decoder.opaque_sites(), 1);
        for auction in [&typed_query[..], &opaque_query[..]] {
            assert_every_prefix(&trie, &decoder, auction);
            let context = Context::new(RelativeVulnerability::NONE, auction);
            let decoded = decoder
                .resolve(&context, auction)
                .expect("rebased exact node");
            assert_eq!(
                decoded.provenance,
                Provenance {
                    depth: auction.len(),
                    fallback: None,
                    rebases: 1,
                },
            );
        }

        let typed_context = Context::new(RelativeVulnerability::NONE, &typed_query);
        let typed = decoder
            .resolve(&typed_context, &typed_query)
            .expect("typed rebase");
        assert!(core::ptr::eq(typed.classifier, typed_target.as_ref()));
        assert!(typed.fast);

        let opaque_context = Context::new(RelativeVulnerability::NONE, &opaque_query);
        let opaque = decoder
            .resolve(&opaque_context, &opaque_query)
            .expect("opaque rebase");
        assert!(core::ptr::eq(opaque.classifier, opaque_target.as_ref()));
        assert!(!opaque.fast);
    }

    #[test]
    fn fallback_depth_and_declaration_order_match_trie() {
        let root = marker(30);
        let first_deep = marker(31);
        let second_deep = marker(32);
        let exact = marker(33);
        let key = [Call::Pass];
        let mut trie = Trie::new();
        trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&root)));
        trie.fallback_at(&key, Always, Fallback::Classify(Arc::clone(&first_deep)));
        trie.fallback_at(&key, Always, Fallback::Classify(Arc::clone(&second_deep)));
        trie.insert_arc(&key, Arc::clone(&exact));

        let decoder = compile(&trie);
        let auction = [Call::Pass, Call::Redouble];
        assert_every_prefix(&trie, &decoder, &auction);

        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let decoded = decoder.resolve(&context, &auction).expect("deep fallback");
        assert!(core::ptr::eq(decoded.classifier, first_deep.as_ref()));
        assert_eq!(
            decoded.provenance,
            Provenance {
                depth: 1,
                fallback: Some(0),
                rebases: 0,
            },
        );
    }

    #[test]
    fn cursor_matches_trie_through_forward_scan() {
        let floor = marker(40);
        let contested = marker(41);
        let exact = marker(42);
        let mut trie = Trie::new();
        trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&floor)));
        trie.fallback_at(
            &[Call::Pass],
            FirstIs(Call::Double),
            Fallback::Classify(Arc::clone(&contested)),
        );
        trie.insert_arc(&[Call::Pass, Call::Double, Call::Pass], Arc::clone(&exact));
        let decoder = compile(&trie);
        let auction = [Call::Pass, Call::Double, Call::Pass, bid(3, Strain::Clubs)];
        let mut cursor = decoder.cursor();

        for depth in 0..=auction.len() {
            let prefix = &auction[..depth];
            let context = Context::new(RelativeVulnerability::NONE, prefix);
            assert_same_resolution(&trie, &decoder, &context, prefix);

            let expected = trie.resolve(&context, prefix);
            let actual = cursor.resolve(&context, prefix);
            match (expected, actual) {
                (Some((expected_classifier, expected_provenance)), Some(actual)) => {
                    assert!(
                        core::ptr::eq(expected_classifier, actual.classifier),
                        "cursor classifier identity differs at {prefix:?}",
                    );
                    assert_eq!(
                        actual.provenance, expected_provenance,
                        "cursor provenance differs at {prefix:?}",
                    );
                }
                (None, None) => {}
                _ => panic!("cursor resolution presence differs at {prefix:?}"),
            }
        }
    }

    #[test]
    fn deal_cursor_matches_trie_for_every_structured_guard_and_rebase() {
        let floor = marker(43);
        let first = marker(44);
        let up_to = marker(45);
        let suffix = marker(46);
        let rebased = marker(47);
        let exact = marker(48);
        let mut trie = Trie::new();
        trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&floor)));
        trie.fallback_at(
            &[Call::Pass],
            FirstIs(Call::Double),
            Fallback::Classify(Arc::clone(&first)),
        );
        trie.fallback_at(
            &[Call::Pass, Call::Pass],
            OvercallAtMost(Bid::new(2, Strain::Spades)),
            Fallback::Classify(Arc::clone(&up_to)),
        );
        trie.fallback_at(
            &[Call::Pass, Call::Pass, Call::Pass],
            SuffixIs(vec![Call::Double, Call::Pass]),
            Fallback::Classify(Arc::clone(&suffix)),
        );
        let rebase_key = bid(1, Strain::Clubs);
        trie.insert_arc(&[rebase_key, Call::Pass, Call::Pass], Arc::clone(&rebased));
        trie.fallback_at(
            &[rebase_key],
            FirstIs(Call::Double),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );
        trie.insert_arc(
            &[
                Call::Pass,
                Call::Pass,
                Call::Pass,
                Call::Double,
                Call::Pass,
                Call::Pass,
            ],
            Arc::clone(&exact),
        );
        let decoder = compile(&trie);

        let auctions = [
            vec![Call::Pass, Call::Double, Call::Pass],
            vec![rebase_key, Call::Double, Call::Pass],
            vec![Call::Pass, Call::Pass, bid(2, Strain::Hearts), Call::Pass],
            vec![
                Call::Pass,
                Call::Pass,
                Call::Pass,
                Call::Double,
                Call::Pass,
                Call::Pass,
            ],
        ];
        for auction in auctions {
            let mut state = DecoderCursorState::default();
            for depth in 0..=auction.len() {
                let prefix = &auction[..depth];
                let context = Context::new(RelativeVulnerability::NONE, prefix);
                let expected = trie.resolve(&context, prefix);
                let actual = match decoder.resolve_checked_with_cursor(&mut state, &context, prefix)
                {
                    CheckedResolution::Decoded(answer) => answer,
                    CheckedResolution::Opaque => panic!("structured route became opaque"),
                };
                match (expected, actual) {
                    (None, None) => {}
                    (Some((expected_classifier, expected_provenance)), Some(actual)) => {
                        assert!(core::ptr::eq(expected_classifier, actual.classifier));
                        assert_eq!(expected_provenance, actual.provenance, "at {prefix:?}");
                    }
                    _ => panic!("deal cursor differs at {prefix:?}"),
                }
            }
        }
    }

    #[test]
    fn deal_cursor_fallback_depth_visits_are_linear_not_triangular() {
        const DEPTH: usize = 32;
        let floor = marker(49);
        let exact = marker(50);
        let auction = vec![Call::Pass; DEPTH];
        let mut trie = Trie::new();
        trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&floor)));
        trie.insert_arc(&auction, Arc::clone(&exact));
        let decoder = compile(&trie);
        let mut state = DecoderCursorState::default();

        for depth in 0..=auction.len() {
            let prefix = &auction[..depth];
            let context = Context::new(RelativeVulnerability::NONE, prefix);
            let expected = trie.resolve(&context, prefix).expect("root floor or exact");
            let actual = match decoder.resolve_checked_with_cursor(&mut state, &context, prefix) {
                CheckedResolution::Decoded(Some(answer)) => answer,
                _ => panic!("expected cache-safe structured answer at {prefix:?}"),
            };
            assert!(core::ptr::eq(expected.0, actual.classifier));
            assert_eq!(expected.1, actual.provenance);
        }

        assert_eq!(
            state.routes.depth_visits(),
            DEPTH,
            "each non-exact prefix should consult only the root candidate depth",
        );
    }

    #[test]
    fn rejected_dynamic_guard_makes_cursor_result_uncacheable() {
        let selected = marker(50);
        let rejected = marker(51);
        let mut trie = Trie::new();
        trie.fallback_at(
            &[],
            super::super::fallback::guard(|_: &Context<'_>, _: &[Call]| false),
            Fallback::Classify(rejected),
        );
        trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&selected)));

        let decoder = compile(&trie);
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let direct = decoder.resolve(&context, &[]).expect("later fallback");
        assert!(core::ptr::eq(direct.classifier, selected.as_ref()));
        assert!(!direct.fast);

        let mut state = DecoderCursorState::default();
        let cached = decoder
            .resolve_with_cursor(&mut state, &context, &[])
            .expect("later fallback through state cursor");
        assert!(core::ptr::eq(cached.classifier, selected.as_ref()));
        assert!(!cached.fast);
        assert!(!state.cache_stable());

        let mut no_answer = Trie::new();
        no_answer.fallback_at(
            &[],
            super::super::fallback::guard(|_: &Context<'_>, _: &[Call]| false),
            Fallback::Classify(marker(52)),
        );
        let no_answer = compile(&no_answer);
        let mut state = DecoderCursorState::default();
        assert!(
            no_answer
                .resolve_with_cursor(&mut state, &context, &[])
                .is_none()
        );
        assert!(!state.cache_stable());
    }

    #[test]
    fn streaming_ledger_compile_matches_catalog_with_grafts_and_overwrites() {
        fn entries() -> Vec<Entry> {
            let mut entries = rows_of(
                Pattern::node("P* (1♣)"),
                Rules::new().rule(Call::Pass, 0, hcp(0..)),
            );
            entries.extend(rows_of(
                Pattern::first("P* 1♦", "X"),
                Rules::new().rule(Call::Pass, 0, hcp(0..)),
            ));
            entries
        }

        let mut source = Trie::new();
        compile_into(
            &mut source,
            &[Package {
                name: "decoder-streaming-test",
                gate: || true,
                entries,
            }],
        );
        let mut trie = source.clone();
        let original = [bid(1, Strain::Clubs)];
        let grafted = [bid(2, Strain::Clubs)];
        assert!(trie.graft(&grafted, &source, &original).is_empty());

        // The original exact slot is stale, but its grafted copy with the same
        // PatternId remains live. Streaming finalization must allocate one
        // metadata entry and attach it only at the live copy.
        let replacement = marker(99);
        trie.insert_arc(&original, Arc::clone(&replacement));

        let catalog = trie.finalize_authoring();
        let catalog_decoder = AuthoringDecoder::compile(&trie, &catalog);
        let ledger = trie.take_authoring_ledger();
        let streaming_decoder = AuthoringDecoder::compile_ledger(&trie, ledger);
        assert!(trie.authoring().patterns().is_empty());
        assert_eq!(
            catalog_decoder.metadata.len(),
            streaming_decoder.metadata.len()
        );
        assert_eq!(streaming_decoder.metadata.len(), 2, "one entry per live ID");

        let fallback = [bid(1, Strain::Diamonds), Call::Double];
        for auction in [&original[..], &grafted[..], &fallback[..]] {
            let context = Context::new(RelativeVulnerability::NONE, auction);
            let expected = catalog_decoder.resolve(&context, auction);
            let actual = streaming_decoder.resolve(&context, auction);
            match (expected, actual) {
                (Some(expected), Some(actual)) => {
                    assert!(core::ptr::eq(expected.classifier, actual.classifier));
                    assert_eq!(expected.provenance, actual.provenance);
                    assert_eq!(expected.pattern_id, actual.pattern_id);
                    assert_eq!(expected.table_id, actual.table_id);
                    assert_eq!(expected.fast, actual.fast);
                }
                (None, None) => {}
                _ => panic!("streaming/catalog presence differs at {auction:?}"),
            }
        }

        let context = Context::new(RelativeVulnerability::NONE, &original);
        let replaced = streaming_decoder
            .resolve(&context, &original)
            .expect("replacement exact classifier");
        assert!(core::ptr::eq(replaced.classifier, replacement.as_ref()));
        assert_eq!(
            replaced.pattern_id, None,
            "stale authoring was not attached"
        );
    }
}
