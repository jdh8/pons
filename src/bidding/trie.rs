use super::Map;
use super::context::Context;
use super::fallback::{Fallback, Guard};
use contract_bridge::Hand;
use contract_bridge::auction::Call;
use core::fmt;
use core::iter::FusedIterator;
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

impl fmt::Debug for dyn Classifier {
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
/// vulnerability.  For example, `[P, 1♠]` as an index stands for the 2nd-seat
/// opening of 1♠.
///
/// Besides the exact book, every node may carry guarded [`Fallback`]s that
/// cover the continuations the book does not; see [`Trie::resolve`].
#[derive(Debug, Clone)]
pub struct Trie {
    children: Map<Box<Self>>,
    classify: Option<Arc<dyn Classifier>>,
    fallbacks: Vec<(Arc<dyn Guard>, Fallback)>,
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
            children: Map::new(),
            classify: None,
            fallbacks: Vec::new(),
        }
    }

    /// Get the sub-trie for the auction
    ///
    /// This method is not made public because auctions have context.
    #[must_use]
    fn subtrie(&self, auction: &[Call]) -> Option<&Self> {
        let mut node = self;

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
        let mut prefix = self.classify.as_deref().map(|f| (&[][..], f));
        let mut node = self;

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
        let mut node = self;

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
        let mut node = self;

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
        self.merge_at(other, &mut Vec::new(), &mut collisions);
        collisions
    }

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
        if let Some((classifier, provenance)) = self.resolve(context, auction) {
            let logits = classifier.classify(hand, context);
            if logits.has_mass() {
                return Some((logits, provenance));
            }
        }
        // The exact node rejected this hand — consult the fallback chain.
        let (classifier, provenance) = self.resolve_at(context, auction, 0, true)?;
        Some((classifier.classify(hand, context), provenance))
    }

    fn resolve_at(
        &self,
        context: &Context<'_>,
        auction: &[Call],
        rebases: usize,
        skip_exact: bool,
    ) -> Option<(&dyn Classifier, Provenance)> {
        let mut path = Vec::with_capacity(auction.len() + 1);
        let mut node = self;
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
    node: &'a Trie,
}

fn collect_children(node: &'_ Trie, depth: usize) -> impl Iterator<Item = StackEntry<'_>> {
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
    trie: &'trie Trie,
    query: &'q [Call],
    depth: usize,
    value: Option<&'trie dyn Classifier>,
}

impl<'trie, 'q> CommonPrefixes<'trie, 'q> {
    /// Construct a common prefix iterator for a trie and an auction
    #[must_use]
    pub fn new(trie: &'trie Trie, query: &'q [Call]) -> Self {
        Self {
            trie,
            query,
            depth: 0,
            value: trie.classify.as_deref(),
        }
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
mod tests {
    use super::*;
    use crate::bidding::Rules;
    use crate::bidding::constraint::hcp;
    use crate::bidding::fallback::Always;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Bid, Strain};

    /// A deliberately partial book node — it only passes weak hands — must not
    /// shadow the floor: a strong hand it rejects (all-`-∞` logits) falls
    /// through to the total floor rather than leaving the driver with no call.
    /// This is the 7NT degenerate-result regression.
    #[test]
    fn partial_node_falls_through_to_the_floor() {
        let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
        let weak_only = Rules::new().rule(Call::Pass, 0.0, hcp(..6));
        // A total floor: `hcp(0..)` accepts every hand, so Pass is always finite.
        let floor = Rules::new().rule(Call::Pass, 0.0, hcp(0..));

        let mut trie = Trie::new();
        trie.insert(&auction, weak_only);
        trie.fallback_at(&[], Always, Fallback::classify(floor));

        let strong: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        // The exact node alone rejects this 21-count: all-`-∞`, no mass.
        let (exact, _) = trie.resolve(&context, &auction).expect("exact node");
        assert!(!exact.classify(strong, &context).has_mass());

        // `classify_floored` falls through to the total floor instead.
        let (logits, provenance) = trie
            .classify_floored(strong, &context, &auction)
            .expect("the floor answers");
        assert!(logits.has_mass(), "the floor gives the hand a finite call");
        assert_eq!(provenance.depth, 0, "the answer came from the root floor");
        assert!(
            provenance.fallback.is_some(),
            "via a fallback, not the book"
        );
    }

    /// A node that *does* cover the hand keeps its own answer — fall-through
    /// triggers only on a no-mass result, never overriding a live book rule.
    #[test]
    fn exact_node_with_mass_is_not_floored() {
        let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
        let opener = Rules::new().rule(Call::Pass, 0.0, hcp(0..));
        let floor = Rules::new().rule(Call::Pass, -5.0, hcp(0..));

        let mut trie = Trie::new();
        trie.insert(&auction, opener);
        trie.fallback_at(&[], Always, Fallback::classify(floor));

        let hand: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let (_, provenance) = trie
            .classify_floored(hand, &context, &auction)
            .expect("the node answers");
        assert_eq!(
            provenance.fallback, None,
            "the exact node wins, not the floor"
        );
    }
}
