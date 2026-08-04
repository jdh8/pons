
# Why a closure is safe

Both box closures are **exact** — they drop no hand the box actually claims —
so neither ever *widens* an arm's apparent claim.  That is what makes the extra
`Envelope::subset_of` containments they expose real, and the `EnvelopeUnion::tidy` dedup
eating them correct (unlike the rejected `suit_hcp` couplings).

Whether a closure is **membership-inert** — whether `Envelope::admits` and
`EnvelopeUnion::contains` accept exactly the same hands before and after, so the sampler
cannot move — depends on which axes it reads:

- `set_sum_closure` **is** inert.  It reads and writes only `lengths`, an axis
  `admits` already enforces, and every real 13-card hand satisfies
  `Σ len = 13`.  Measured over 409,708 sampled layouts by
  `examples/probe-closure-features.rs`: zero rejections in either direction.
- `set_upgrade_closure` is **not**.  It bounds `points` — which `admits` tests
  — using `hcp`, which `admits` ignores until `set_gauge_membership`.  So it
  gives an otherwise unenforced HCP claim teeth *through* `points`: a box
  reading `hcp ..=8` with `points` slacked to `..=10` narrows back to `..=8`
  once its lengths force balanced, and the sampler stops dealing the 9- and
  10-counts it was accepting outside the stated band.  Same probe: 249
  rejections in 8,576 layouts (pinned by `upgrade_closure_gives_hcp_teeth`).
  Arguably the old acceptance was the wrong one — but this is a reading change,
  and it must be measured as one.

The remaining delta is `EnvelopeUnion::hull` — tighter, so the bilans evaluator and the
feature nets see tighter bands.  This is the intended effect, and also the known
risk: the chop-F1 saga is precisely that tighter prefixed hulls put
`trick_estimates` out of distribution for the evaluator net.  Budget the F2b
evaluator-twin retrain before calling a measured loss the idea's fault.
