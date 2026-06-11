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
pub fn classifier<F>(f: F) -> F
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
    /// Sharing one [`Arc`] across several keys (or across the books of a
    /// [`Forest`]) is pointer-cheap.
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
    /// `auction` is the trie key to resolve — for a partnership [`Forest`],
    /// the pass-stripped key.  `context`, which guards also receive, always
    /// describes the *original* table auction: even when the classifier is
    /// found through a rebase or a stripped key, it classifies the real one.
    #[must_use]
    pub fn resolve(
        &self,
        context: &Context<'_>,
        auction: &[Call],
    ) -> Option<(&dyn Classifier, Provenance)> {
        self.resolve_at(context, auction, 0)
    }

    fn resolve_at(
        &self,
        context: &Context<'_>,
        auction: &[Call],
        rebases: usize,
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

        if path.len() == auction.len() + 1
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
                            && let Some(found) = self.resolve_at(context, &rewritten, rebases + 1)
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

/// A partnership system: two books keyed by pass-stripped auctions
///
/// Most systems treat the 1st and 2nd seats alike, and the 3rd and 4th
/// alike: what changes the book is whether our side has already passed.
/// A forest therefore stores two [`Trie`]s and strips the leading passes
/// off the table auction before lookup:
///
/// - [`unpassed`](Self::unpassed) serves the side that did not pass before
///   the first non-pass call (1st/2nd-seat openings, direct defense),
/// - [`passed`](Self::passed) serves the side that did (3rd/4th-seat
///   openings, defense after our initial pass).
///
/// The trie is selected per *side*, not per auction: with `k` leading
/// passes, the opener's side is passed iff `k ≥ 2`, the defenders' iff
/// `k ≥ 1`.  Stripping also normalizes parity — at **even** stripped depth
/// the opening side acts, at **odd** depth the defending side, in both
/// tries, for every seat.  This is what makes 1st/2nd-seat books literally
/// share nodes.
///
/// Exact-seat exceptions (e.g. no preempts in 4th seat) belong in
/// constraints such as [`nth_seat`](super::constraint::nth_seat), and
/// vulnerability conditions in [`vulnerable`](super::constraint::vulnerable)
/// — not in extra keys.
#[derive(Clone, Debug, Default)]
pub struct Forest {
    /// Book for the side that has not passed before the first non-pass call
    pub unpassed: Trie,
    /// Book for the side that passed before the first non-pass call
    pub passed: Trie,
}

bitflags::bitflags! {
    /// Seat classes of a partnership [`Forest`]
    ///
    /// Selects which book(s) of a forest an insertion targets.  Most
    /// definitions apply to both classes ([`SeatClasses::ALL`]) and share one
    /// classifier; class-specific books (light 3rd-seat openings, no 2/1
    /// game forces by a passed hand) target one class.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SeatClasses: u8 {
        /// The side that has not passed before the first non-pass call
        const UNPASSED = 1;
        /// The side that passed before the first non-pass call
        const PASSED = 2;
    }
}

impl SeatClasses {
    /// Both seat classes
    pub const ALL: Self = Self::all();
}

impl Forest {
    /// Construct a forest with empty tries
    #[must_use]
    pub const fn new() -> Self {
        Self {
            unpassed: Trie::new(),
            passed: Trie::new(),
        }
    }

    /// Insert a [`Classifier`] into the selected book(s)
    ///
    /// `auction` is the pass-stripped key (see the type-level docs).  When
    /// both classes are selected, the books share one
    /// [`Arc`]`<dyn Classifier>` — sharing is pointer-cheap.  To override
    /// one class afterwards, insert into that class alone; exact inserts
    /// replace.
    pub fn insert(&mut self, auction: &[Call], classes: SeatClasses, f: impl Classifier + 'static) {
        let f: Arc<dyn Classifier> = Arc::new(f);

        if classes.contains(SeatClasses::UNPASSED) {
            self.unpassed.insert_arc(auction, Arc::clone(&f));
        }
        if classes.contains(SeatClasses::PASSED) {
            self.passed.insert_arc(auction, f);
        }
    }

    /// Attach a guarded [`Fallback`] in the selected book(s)
    ///
    /// The guard and the fallback are shared across the books like
    /// [`Forest::insert`] shares classifiers.
    pub fn fallback_at(
        &mut self,
        auction: &[Call],
        classes: SeatClasses,
        guard: impl Guard + 'static,
        fallback: Fallback,
    ) {
        let guard: Arc<dyn Guard> = Arc::new(guard);

        if classes.contains(SeatClasses::UNPASSED) {
            self.unpassed
                .fallback_arc_at(auction, Arc::clone(&guard), fallback.clone());
        }
        if classes.contains(SeatClasses::PASSED) {
            self.passed.fallback_arc_at(auction, guard, fallback);
        }
    }

    /// Merge another forest into this one, book by book
    ///
    /// See [`Trie::merge`] for the collision policy.  The reported keys do
    /// not distinguish which book collided.
    pub fn merge(&mut self, other: Self) -> Vec<Box<[Call]>> {
        let mut collisions = self.unpassed.merge(other.unpassed);
        collisions.extend(self.passed.merge(other.passed));
        collisions
    }
}
