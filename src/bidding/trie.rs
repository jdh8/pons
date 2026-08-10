use super::Map;
use super::context::Context;
use super::fallback::{Fallback, Guard};
use super::rows::AuthoringLedger;
#[cfg(test)]
use super::rows::{
    AuthoringCatalog, BoundPattern, ConcreteSite, PatternId, Placement, PreservedTarget,
};
use contract_bridge::Hand;
use contract_bridge::auction::Call;
use core::fmt;
use core::iter::FusedIterator;
#[cfg(test)]
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for a function that classifies a hand into logits for each call
pub trait Classifier: Send + Sync {
    /// Classify a hand with the given context into logits
    fn classify(&self, hand: Hand, context: &Context<'_>) -> super::array::Logits;

    /// Downcast to the authored [`Rules`][super::rules::Rules], if this is one
    ///
    /// Classifiers live type-erased in the [`Trie`]; the description-corpus
    /// exporter and `explain()`-style tooling recover the authored rules — their
    /// calls, weights, and labels — through this hook.  Defaults to [`None`];
    /// only [`Rules`][super::rules::Rules] overrides it to return itself.
    fn as_rules(&self) -> Option<&super::rules::Rules> {
        None
    }
}

impl fmt::Debug for dyn Classifier + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Classifier({:p})", &self)
    }
}

impl<F> Classifier for F
where
    F: Fn(Hand, &Context<'_>) -> super::array::Logits + Send + Sync,
{
    fn classify(&self, hand: Hand, context: &Context<'_>) -> super::array::Logits {
        self(hand, context)
    }
}

/// Coerce a closure into a [`Classifier`]
///
/// The compiler cannot generalize the lifetime of `&Context` when a plain
/// closure is passed straight to a generic [`Classifier`] parameter such as
/// [`Trie::insert`].  Routing the closure through this identity function
/// provides the expected signature:
///
/// ```
/// use pons::Trie;
/// use pons::bidding::array::Logits;
/// use pons::bidding::trie::classifier;
///
/// let mut trie = Trie::new();
/// trie.insert(&[], classifier(|_, _| Logits::new()));
/// ```
pub const fn classifier<F>(f: F) -> F
where
    F: Fn(Hand, &Context<'_>) -> super::array::Logits + Send + Sync,
{
    f
}

/// Decision trie as a vulnerability-agnostic bidding system
///
/// A trie stores a [`Classifier`] for each covered auction without
/// vulnerability.  For example, `- 1♠` as an index stands for the 2nd-seat
/// opening of 1♠.
///
/// Besides the exact book, every node may carry guarded [`Fallback`]s that
/// cover the continuations the book does not; see [`Trie::resolve`].
#[derive(Clone)]
pub struct Trie {
    root: TrieNode,
    authoring: AuthoringLedger,
}

// Preserve the public diagnostic shape from before authoring metadata was
// retained beside the trie.  The pending ledger is a build-time concern and
// can be very large; rendering it here would be both noisy and observable.
impl fmt::Debug for Trie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Trie")
            .field("children", &self.root.children)
            .field("classify", &self.root.classify)
            .field("fallbacks", &self.root.fallbacks)
            .finish()
    }
}

/// One routing node inside a [`Trie`]
///
/// Keeping the recursive node separate from the public wrapper is
/// load-bearing for whole-book finalization: preserved row metadata belongs
/// once to the book, not once to every auction node.
#[derive(Clone)]
pub(crate) struct TrieNode {
    pub(crate) children: Map<Box<Self>>,
    pub(crate) classify: Option<Arc<dyn Classifier>>,
    pub(crate) fallbacks: Vec<(Arc<dyn Guard>, Fallback)>,
}

// Keep the recursive diagnostic shape of the original recursive `Trie`.
// The new book-level authoring ledger must stay invisible at every depth.
impl fmt::Debug for TrieNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Trie")
            .field("children", &self.children)
            .field("classify", &self.classify)
            .field("fallbacks", &self.fallbacks)
            .finish()
    }
}

impl TrieNode {
    const fn new() -> Self {
        Self {
            children: Map::new(),
            classify: None,
            fallbacks: Vec::new(),
        }
    }
}

impl Default for TrieNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

/// Maximum number of rebases during one resolution
///
/// Rebases rewrite the auction and resolve again; this limit breaks rewrite
/// cycles.
pub const REBASE_LIMIT: usize = 8;

/// How a classifier was found by [`Trie::resolve`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// Depth of the node where the classifier was found, measured in the
    /// possibly rewritten auction
    pub depth: usize,
    /// Index of the fallback entry at that node, or [`None`] for the exact
    /// book classifier
    pub fallback: Option<usize>,
    /// Number of rebases taken
    pub rebases: usize,
}

impl Trie {
    /// Construct an empty trie
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: TrieNode::new(),
            authoring: AuthoringLedger::new(),
        }
    }

    /// Get the sub-trie for the auction
    ///
    /// This method is not made public because auctions have context.
    #[must_use]
    fn subtrie(&self, auction: &[Call]) -> Option<&TrieNode> {
        let mut node = &self.root;

        for &call in auction {
            node = node.children.get(call)?;
        }
        Some(node)
    }

    /// Get the [`Classifier`] for the exact auction
    #[must_use]
    pub fn get(&self, auction: &[Call]) -> Option<&dyn Classifier> {
        self.subtrie(auction)
            .and_then(|node| node.classify.as_deref())
    }

    /// Check if the query auction is a prefix in the trie
    #[must_use]
    pub fn is_prefix(&self, auction: &[Call]) -> bool {
        self.subtrie(auction).is_some()
    }

    /// Get the longest prefix of the auction that has a [`Classifier`]
    #[must_use]
    pub fn longest_prefix<'a>(&self, auction: &'a [Call]) -> Option<(&'a [Call], &dyn Classifier)> {
        let mut prefix = self.root.classify.as_deref().map(|f| (&[][..], f));
        let mut node = &self.root;

        for (depth, &call) in auction.iter().enumerate() {
            node = match node.children.get(call) {
                Some(child) => child,
                None => break,
            };
            if let Some(f) = node.classify.as_deref() {
                prefix.replace((&auction[..=depth], f));
            }
        }
        prefix
    }

    /// Insert a [`Classifier`] into the trie
    pub fn insert(
        &mut self,
        auction: &[Call],
        f: impl Classifier + 'static,
    ) -> Option<Arc<dyn Classifier>> {
        self.insert_arc(auction, Arc::new(f))
    }

    /// Insert an already shared [`Classifier`] into the trie
    ///
    /// Sharing one [`Arc`] across several keys — such as one classifier reused
    /// across seat prefixes — is pointer-cheap.
    pub fn insert_arc(
        &mut self,
        auction: &[Call],
        f: Arc<dyn Classifier>,
    ) -> Option<Arc<dyn Classifier>> {
        let mut node = &mut self.root;

        for &call in auction {
            node = node.children.entry(call).get_or_insert_with(Box::default);
        }
        node.classify.replace(f)
    }

    /// Attach a guarded [`Fallback`] at the node for the auction
    ///
    /// Fallbacks at a node cover every continuation below it that resolution
    /// reaches; within a node they are tried in declaration order.  See
    /// [`Trie::resolve`] for the full precedence.
    pub fn fallback_at(
        &mut self,
        auction: &[Call],
        guard: impl Guard + 'static,
        fallback: Fallback,
    ) {
        self.fallback_arc_at(auction, Arc::new(guard), fallback);
    }

    /// Attach a guarded [`Fallback`] with an already shared [`Guard`]
    pub fn fallback_arc_at(&mut self, auction: &[Call], guard: Arc<dyn Guard>, fallback: Fallback) {
        let mut node = &mut self.root;

        for &call in auction {
            node = node.children.entry(call).get_or_insert_with(Box::default);
        }
        node.fallbacks.push((guard, fallback));
    }

    /// Merge another trie into this one, reusing the shared classifiers
    ///
    /// This is the structural union for assembling a system from separately
    /// authored fragments (an uncontested core, a competitive package, …).
    /// Classifiers from `other` fill nodes that have none; when both tries
    /// classify the same auction, `self` keeps its classifier and the
    /// auction is reported back — fragments are expected to occupy disjoint
    /// paths, so a collision is almost certainly an authoring bug.  Fallback
    /// lists concatenate with `self`'s entries first.
    pub fn merge(&mut self, other: Self) -> Vec<Box<[Call]>> {
        let mut collisions = Vec::new();
        self.root
            .merge_at(other.root, &mut Vec::new(), &mut collisions);
        self.authoring.extend(other.authoring);
        collisions
    }

    /// Graft the subtree rooted at `src_prefix` in `src` under `dst_prefix`
    ///
    /// Copies `src`'s node at `src_prefix` (classifiers shared through their
    /// [`Arc`]s) into the node at `dst_prefix`, creating the destination path
    /// if absent.  This re-roots an authored subtree at another key — e.g.
    /// grafting the uncontested `1NT`-opening responses under a `1NT`-overcall
    /// prefix so the advancer plays them (systems on).  Returns the colliding
    /// keys (relative to `dst_prefix`); a vacant destination yields none, so a
    /// non-empty result is an authoring bug.  A no-op when `src` lacks
    /// `src_prefix`.
    pub fn graft(
        &mut self,
        dst_prefix: &[Call],
        src: &Self,
        src_prefix: &[Call],
    ) -> Vec<Box<[Call]>> {
        let Some(sub) = src.subtrie(src_prefix) else {
            return Vec::new();
        };
        let sub = sub.clone();
        let mut node = &mut self.root;
        for &call in dst_prefix {
            node = node.children.entry(call).get_or_insert_with(Box::default);
        }
        let mut collisions = Vec::new();
        node.merge_at(sub, &mut Vec::new(), &mut collisions);
        self.authoring
            .extend_graft(&src.authoring, dst_prefix, src_prefix);
        collisions
    }

    /// Preserve one declarative row pattern beside its lowered trie objects.
    pub(crate) fn preserve_authoring(&mut self, pattern: super::rows::PreservedPattern) {
        self.authoring.push(pattern);
    }

    /// The preserved declarative patterns, in authoring order.
    #[cfg(test)]
    pub(crate) const fn authoring(&self) -> &AuthoringLedger {
        &self.authoring
    }

    /// Root routing node for eager bound-book compilation.
    pub(crate) const fn root_node(&self) -> &TrieNode {
        &self.root
    }

    /// Freeze the row metadata that still owns a live runtime slot
    ///
    /// Exact overwrites (notably the Dutch opening), merge collisions, and
    /// arbitrary public trie mutation can displace a lowered row after it was
    /// recorded. The concrete trie is authoritative: only pointer-identical
    /// classifiers/guards/rewrites survive into the bound catalog.
    #[cfg(test)]
    pub(crate) fn finalize_authoring(&self) -> AuthoringCatalog {
        self.finalize_patterns(self.authoring.patterns().iter().cloned())
    }

    /// Drain the build-time row ledger for streaming decoder finalization.
    pub(crate) fn take_authoring_ledger(&mut self) -> AuthoringLedger {
        core::mem::take(&mut self.authoring)
    }

    /// Consume the pending authoring ledger while finalizing a runtime book.
    ///
    /// This is the production path used by `System::bind`: stale objects and
    /// pre-finalization key lists are dropped before the trie enters a partnership.
    #[cfg(test)]
    pub(crate) fn take_finalized_authoring(&mut self) -> AuthoringCatalog {
        let ledger = self.take_authoring_ledger();
        self.finalize_patterns(ledger.into_patterns())
    }

    #[cfg(test)]
    fn finalize_patterns(
        &self,
        authored: impl IntoIterator<Item = super::rows::LedgerPattern>,
    ) -> AuthoringCatalog {
        let mut patterns = Vec::<BoundPattern>::new();
        let mut by_id = HashMap::<PatternId, usize>::new();

        for pattern in authored {
            let declaration = pattern.declaration;
            let mut sites = Vec::new();
            for key in &pattern.keys {
                let Some(node) = self.subtrie(key) else {
                    continue;
                };
                if pattern.guard.is_none() {
                    let Some(actual) = &node.classify else {
                        continue;
                    };
                    let Some(expected) = pattern.target.classifier() else {
                        continue;
                    };
                    if Arc::ptr_eq(actual, expected) {
                        sites.push(ConcreteSite {
                            key: key.clone(),
                            placement: Placement::Exact,
                        });
                    }
                    continue;
                }

                let expected_guard = pattern.guard.as_ref().expect("guarded pattern");
                for (index, (guard, fallback)) in node.fallbacks.iter().enumerate() {
                    if !Arc::ptr_eq(guard, expected_guard) {
                        continue;
                    }
                    let target_matches = match (&pattern.target, fallback) {
                        (
                            PreservedTarget::Rules(expected) | PreservedTarget::Computed(expected),
                            Fallback::Classify(actual),
                        ) => Arc::ptr_eq(expected, actual),
                        (PreservedTarget::Rebase(expected), Fallback::Rebase(actual)) => {
                            Arc::ptr_eq(expected, actual)
                        }
                        _ => false,
                    };
                    if target_matches {
                        let site = ConcreteSite {
                            key: key.clone(),
                            placement: Placement::Fallback { index },
                        };
                        if !sites.contains(&site) {
                            sites.push(site);
                        }
                    }
                }
            }
            if sites.is_empty() {
                continue;
            }

            if let Some(&index) = by_id.get(&pattern.id) {
                let bound = &mut patterns[index];
                bound.declaration = bound.declaration.min(declaration);
                let mut joined = bound.sites.to_vec();
                for site in sites {
                    if !joined.contains(&site) {
                        joined.push(site);
                    }
                }
                bound.sites = joined.into_boxed_slice();
            } else {
                by_id.insert(pattern.id, patterns.len());
                patterns.push(BoundPattern {
                    id: pattern.id,
                    table_id: pattern.table_id,
                    package: Arc::clone(&pattern.package),
                    source: Arc::clone(&pattern.source),
                    grammar: pattern.grammar.clone(),
                    declaration,
                    target: pattern.target.kind(),
                    sites: sites.into_boxed_slice(),
                });
            }
        }
        AuthoringCatalog::new(patterns)
    }
}

impl TrieNode {
    fn merge_at(&mut self, other: Self, path: &mut Vec<Call>, collisions: &mut Vec<Box<[Call]>>) {
        if let Some(classifier) = other.classify {
            if self.classify.is_some() {
                collisions.push(path.as_slice().into());
            } else {
                self.classify = Some(classifier);
            }
        }
        self.fallbacks.extend(other.fallbacks);

        for (call, child) in other.children {
            path.push(call);
            self.children
                .entry(call)
                .get_or_insert_with(Box::default)
                .merge_at(*child, path, collisions);
            path.pop();
        }
    }
}

impl Trie {
    /// Resolve an auction to a classifier
    ///
    /// Precedence, most specific first:
    ///
    /// 1. the exact classifier for the full auction (the book),
    /// 2. walking **up** from the deepest reachable node, the first fallback
    ///    whose guard admits the uncovered suffix — deeper nodes win, and
    ///    entries at one node apply in declaration order,
    /// 3. [`None`].
    ///
    /// A [`Fallback::Rebase`] rewrites the auction and resolves again, at
    /// most [`REBASE_LIMIT`] times.  The returned [`Provenance`] tells where
    /// the classifier was found.
    ///
    /// `auction` is the trie key to resolve.  `context`, which guards also
    /// receive, always describes the *original* table auction: even when the
    /// classifier is found through a rebase, it classifies the real one.
    #[must_use]
    pub fn resolve(
        &self,
        context: &Context<'_>,
        auction: &[Call],
    ) -> Option<(&dyn Classifier, Provenance)> {
        self.resolve_at(context, auction, 0, false)
    }

    /// Classify, falling through to the fallback chain when the exact node
    /// yields no mass for `hand`
    ///
    /// [`resolve`][Self::resolve] picks the most specific classifier
    /// structurally, by auction prefix, and a deliberately partial book node can
    /// then reject the hand — leaving all-[`f32::NEG_INFINITY`] logits that
    /// shadow the floor it sits above.  This consults that node first and, only
    /// when it has no mass, walks up to the fallback chain (the
    /// floor).  The root `Always` floor is total, so this returns mass whenever a
    /// floor is attached; with no floor (the bare-book ablation) it returns the
    /// degenerate logits, and the driver passes as before.
    ///
    /// `ponytail:` single fall-through — it assumes the next mass-bearing
    /// candidate is the floor, which holds for the root-only floor wiring.  If
    /// intermediate partial fallbacks ever appear, loop until the result has
    /// mass.
    #[must_use]
    pub fn classify_floored(
        &self,
        hand: Hand,
        context: &Context<'_>,
        auction: &[Call],
    ) -> Option<(super::array::Logits, Provenance)> {
        self.resolve_floored(hand, context, auction)
            .map(|(_, logits, provenance)| (logits, provenance))
    }

    /// [`classify_floored`][Self::classify_floored], also yielding the winning
    /// classifier itself — the attribution hook (which node or floor rule
    /// answered), used by [`Partnership::explain_call`][super::book::Partnership::explain_call]
    pub(crate) fn resolve_floored(
        &self,
        hand: Hand,
        context: &Context<'_>,
        auction: &[Call],
    ) -> Option<(&dyn Classifier, super::array::Logits, Provenance)> {
        if let Some((classifier, provenance)) = self.resolve(context, auction) {
            let logits = classifier.classify(hand, context);
            if logits.has_mass() {
                return Some((classifier, logits, provenance));
            }
        }
        // The exact node rejected this hand — consult the fallback chain.
        let (classifier, provenance) = self.resolve_at(context, auction, 0, true)?;
        Some((classifier, classifier.classify(hand, context), provenance))
    }

    /// Resolve only the fallback chain after an exact classifier rejected.
    ///
    /// Finalized books use this seam to evaluate a rule-backed classifier
    /// through its compiled sidecar while retaining the trie's authoritative
    /// routing and single-fall-through semantics.
    pub(crate) fn resolve_after_exact_rejection(
        &self,
        context: &Context<'_>,
        auction: &[Call],
    ) -> Option<(&dyn Classifier, Provenance)> {
        self.resolve_at(context, auction, 0, true)
    }

    fn resolve_at(
        &self,
        context: &Context<'_>,
        auction: &[Call],
        rebases: usize,
        skip_exact: bool,
    ) -> Option<(&dyn Classifier, Provenance)> {
        let mut path = Vec::with_capacity(auction.len() + 1);
        let mut node = &self.root;
        path.push(node);

        for &call in auction {
            match node.children.get(call) {
                Some(child) => {
                    node = child;
                    path.push(node);
                }
                None => break,
            }
        }

        if !skip_exact
            && path.len() == auction.len() + 1
            && let Some(classifier) = node.classify.as_deref()
        {
            let provenance = Provenance {
                depth: auction.len(),
                fallback: None,
                rebases,
            };
            return Some((classifier, provenance));
        }

        for (depth, node) in path.iter().enumerate().rev() {
            for (index, (guard, fallback)) in node.fallbacks.iter().enumerate() {
                if !guard.admits(context, &auction[depth..]) {
                    continue;
                }

                match fallback {
                    Fallback::Classify(classifier) => {
                        let provenance = Provenance {
                            depth,
                            fallback: Some(index),
                            rebases,
                        };
                        return Some((classifier.as_ref(), provenance));
                    }
                    Fallback::Rebase(rewrite) => {
                        if rebases < REBASE_LIMIT
                            && let Some(rewritten) = rewrite.rewrite(auction, depth)
                            && let Some(found) =
                                self.resolve_at(context, &rewritten, rebases + 1, false)
                        {
                            return Some(found);
                        }
                    }
                }
            }
        }
        None
    }

    /// The classifier that authored the call made at the end of `prefix` — the
    /// decision resolved at `prefix`, including guarded fallbacks
    ///
    /// Unlike [`common_prefixes`][Self::common_prefixes], which yields only
    /// exact-node classifiers, this walks the same node-then-fallback chain
    /// [`classify_floored`][Self::classify_floored] uses, so a call authored by a
    /// guarded fallback (every contested convention — transfers, Leaping Michaels,
    /// the Lebensohl cue) is decoded the same way it was bid.  Used by the
    /// projection pass to read fallback-authored conventions off their rule.
    pub(crate) fn authoring_classifier(
        &self,
        context: &Context<'_>,
        prefix: &[Call],
    ) -> Option<&dyn Classifier> {
        self.resolve_at(context, prefix, 0, false).map(|(c, _)| c)
    }

    /// Every guarded [`Fallback`] in the trie, with the auction of its node
    ///
    /// Depth-first; within a node, declaration order (= resolution
    /// precedence).  The Pass child of each node is visited **last**, so seat
    /// variants — one shared entry installed under leading-pass prefixes —
    /// surface their canonical pass-less key first for the renderers'
    /// first-seen pointer dedup.
    #[must_use]
    pub fn fallbacks(&self) -> Vec<(Box<[Call]>, &dyn Guard, &Fallback)> {
        fn collect<'a>(
            node: &'a TrieNode,
            prefix: &mut Vec<Call>,
            out: &mut Vec<(Box<[Call]>, &'a dyn Guard, &'a Fallback)>,
        ) {
            for (guard, fallback) in &node.fallbacks {
                out.push((prefix.as_slice().into(), guard.as_ref(), fallback));
            }
            let pass_last = node
                .children
                .iter()
                .filter(|&(call, _)| call != Call::Pass)
                .chain(node.children.iter().filter(|&(call, _)| call == Call::Pass));
            for (call, child) in pass_last {
                prefix.push(call);
                collect(child, prefix, out);
                prefix.pop();
            }
        }

        let mut out = Vec::new();
        collect(&self.root, &mut Vec::new(), &mut out);
        out
    }

    /// Depth first iteration over all nodes with a [`Classifier`]
    #[must_use]
    pub fn iter(&'_ self) -> Suffixes<'_> {
        self.suffixes(&[])
    }

    /// Depth first iteration over all suffixes to the auction
    #[must_use]
    pub fn suffixes(&self, auction: &[Call]) -> Suffixes<'_> {
        Suffixes::new(self, auction)
    }

    /// Iterate over common prefixes of the auction
    #[must_use]
    pub fn common_prefixes<'q>(&self, query: &'q [Call]) -> CommonPrefixes<'_, 'q> {
        CommonPrefixes::new(self, query)
    }
}

impl<'a> IntoIterator for &'a Trie {
    type Item = (Box<[Call]>, &'a dyn Classifier);
    type IntoIter = Suffixes<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, Copy)]
struct StackEntry<'a> {
    depth: usize,
    call: Call,
    node: &'a TrieNode,
}

fn collect_children(node: &'_ TrieNode, depth: usize) -> impl Iterator<Item = StackEntry<'_>> {
    node.children.iter().map(move |(call, child)| StackEntry {
        depth,
        call,
        node: child,
    })
}

/// Suffix iterator for a given auction
///
/// This is the return type of [`Trie::suffixes`].
#[derive(Clone)]
pub struct Suffixes<'a> {
    stack: Vec<StackEntry<'a>>,
    auction: Vec<Call>,
    separator: usize,
    value: Option<&'a dyn Classifier>,
}

impl<'a> Suffixes<'a> {
    /// Construct an empty iterator
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            stack: Vec::new(),
            auction: Vec::new(),
            separator: 0,
            value: None,
        }
    }

    /// Construct a suffix iterator for a trie and an auction
    #[must_use]
    pub fn new(trie: &'a Trie, auction: &[Call]) -> Self {
        let Some(node) = trie.subtrie(auction) else {
            return Self::empty();
        };

        Self {
            stack: collect_children(node, 0).collect(),
            separator: auction.len(),
            value: node.classify.as_deref(),
            auction: auction.to_vec(),
        }
    }
}

impl fmt::Debug for Suffixes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Suffixes")
            .field("auction", &self.auction)
            .field("separator", &self.separator)
            .field("pending", &self.stack.len())
            .field("has_value", &self.value.is_some())
            .finish()
    }
}

impl<'a> Iterator for Suffixes<'a> {
    type Item = (Box<[Call]>, &'a dyn Classifier);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let entry = self.stack.pop()?;
            self.stack
                .extend(collect_children(entry.node, entry.depth + 1));
            self.value = entry.node.classify.as_deref();
            self.auction.truncate(self.separator + entry.depth);
            self.auction.push(entry.call);
        }

        Some((self.auction[self.separator..].into(), self.value.take()?))
    }
}

impl FusedIterator for Suffixes<'_> {}

/// Common prefix iterator for a given auction
#[derive(Clone)]
pub struct CommonPrefixes<'trie, 'q> {
    root: &'trie Trie,
    trie: &'trie TrieNode,
    query: &'q [Call],
    depth: usize,
    value: Option<&'trie dyn Classifier>,
}

impl<'trie, 'q> CommonPrefixes<'trie, 'q> {
    /// Construct a common prefix iterator for a trie and an auction
    #[must_use]
    pub fn new(trie: &'trie Trie, query: &'q [Call]) -> Self {
        Self {
            root: trie,
            trie: &trie.root,
            query,
            depth: 0,
            value: trie.root.classify.as_deref(),
        }
    }

    /// The root trie these prefixes were taken from (unchanged by iteration), so
    /// the projection pass can re-resolve each call's *authoring* classifier —
    /// including guarded fallbacks, which the exact-node walk here skips
    pub(crate) const fn root(&self) -> &'trie Trie {
        self.root
    }
}

impl fmt::Debug for CommonPrefixes<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommonPrefixes")
            .field("query", &self.query)
            .field("depth", &self.depth)
            .field("has_value", &self.value.is_some())
            .finish()
    }
}

impl<'trie, 'q> Iterator for CommonPrefixes<'trie, 'q> {
    type Item = (&'q [Call], &'trie dyn Classifier);

    fn next(&mut self) -> Option<Self::Item> {
        while self.value.is_none() {
            let &call = self.query.get(self.depth)?;
            self.trie = self.trie.children.get(call)?;
            self.value = self.trie.classify.as_deref();
            self.depth += 1;
        }

        Some((&self.query[..self.depth], self.value.take()?))
    }
}

impl FusedIterator for CommonPrefixes<'_, '_> {}

#[cfg(test)]
mod tests;
