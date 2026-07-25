
# Why a closure is safe

Both box closures are **exact** — they drop no hand — and therefore
**membership-inert**: every real 13-card hand already satisfies `Σ len = 13`
and `points = hcp + upgrade`, so `Envelope::admits` and `Dnf::contains` accept
exactly the same hands before and after.  The sampler cannot move.

The deltas are confined to two places:

- `Dnf::hull` — tighter, so the bilans evaluator and the feature nets see
  tighter bands.  This is the intended effect, and also the known risk: the
  chop-F1 saga is precisely that tighter prefixed hulls put `trick_estimates`
  out of distribution for the evaluator net.  Budget the F2b evaluator-twin
  retrain before calling a measured loss the idea's fault.
- `Envelope::subset_of` — more containments found, so fewer terms survive
  `Dnf::tidy`.  Exactness is what makes that safe: unlike the rejected
  `suit_hcp` couplings, the narrowing never *widens* an arm's apparent claim,
  so every containment it exposes is real and the dedup eating it is correct.
