# Strong jump rebids — round 3 handoff

**Written 2026-11-07.** The lane is opener's strong jump rebids after a one-level
response, the `6141cdb2` anchor's sharpest round-2 cluster (1,198 boards,
−2,035 plain / −2,359 PD; [bba-gap-campaign.md](bba-gap-campaign.md), "That
lane, worked"). Rounds 1–2 shipped both knobs default-on. This file owns the
rest of the lane.

## State at handoff

| Knob | Code | Verdict |
| --- | --- | --- |
| `rebid.major_jump_rebid_shape` | `src/bidding/american/rebids/major_jump_rebid.rs` | won 2/2 (plain +0.0018 / +0.0027), default-on |
| `rebid.minor_jump_shift_answers` | `src/bidding/american/rebids/minor_jump_shift.rs` | r1 inconclusive, r2 non-inferior 2/2 (+0.0004), default-on on naturalness (recorded trade-off, lower bound −0.0016) |

Runner: `scripts/ab-strong-jump-rebids.sh` (control arm
`--no-ns-minor-jump-shift-answers`). Round-2 results:
`ab-results/strong-jump-rebids-r2/`, seed 1794059060547829818. Read its
`diff.answers.vs.off.*.plain.txt` worst boards before starting.

## Round 3: the unauthored rungs

Every item below comes from a round-2 worst board, or from a floor test the
flip took over. Since `answers` is now the default, round 3 needs a
**new knob**, default off, gating the rungs below (for example
`rebid.minor_jump_shift_slam`). Its A/B arm runs against HEAD defaults.
Numbers come from the censuses, not from memory: major and minor fits in
[suit-slam.md](suit-slam.md) (best `point_count` threshold 29–30 major,
28–29 minor), notrump in [notrump-slam.md](notrump-slam.md) (6NT wants
31 HCP on actual holdings).

All auctions are written from the jump shift onward. The node is
`P* 1m - 1M - Jx {- | (X)}`.

1. **Direct `3NT` on slam values.** `1♣ - 1♥ - 2♠ - 3NT -`, responder
   K86.KQT2.AQT63.T (14 HCP): the floor bid `6NT` and made it; our `3NT`
   catch-all stops. Add a `6NT` rung (or a quantitative `4NT` in the lanes
   where `4NT` is not the major keycard raise). Set the HCP floor from
   opener's HCP spread at the jump shift (18+ *points* on a 5-4 is about
   16+ HCP). Probe that spread; don't assume it.
2. **Responder's `3NT` pull on six with slam values.** `1♦ - 1♠ - 3♣ - 3♠ -
   3NT - 4♠`, responder AKT743.K8.T2.KJ3 (14 HCP): the off arm's `6NT` made.
   `responder_over_notrump`'s `6NT` rung is `hcp(15..)`. Revisit it with
   item 1's threshold, and decide whether six with 13+ asks keycards in
   the major instead.
3. **The 11–12 raise of a minor second suit.** `1♦ - 1♠ - 3♣ - 3NT`,
   responder 9873.AQ4.J.AJT92 (11 HCP, five clubs, a singleton diamond):
   the floor reached a making `7♣`. The minor `4s` rung needs 13+ points.
   The major arm's `4NT` already starts at 11 (18 + 11 = 29). Align the
   minor arm with the minor census (28–29).
4. **Opener's raise of responder's `3M` on 21+.** `1♦ - 1♥ - 2♠ - 3♥ - 4♥`,
   opener AK76.AQ3.AK852.T (21 HCP, three hearts), responder
   J83.KJT965.76.43: the floor's `6♥` made. `opener_over_major` raises to
   `4M` on any 3+. On 21+ it should ask (`4NT` keycard in the major; wire
   `slam::rkcb_rows`).
5. **The grand rung over the minor raise.** `1♦ - 1♠ - 3♣ - 4♣ - 4NT - 5♦ -
   6♣` with opener .KQ87.AKQT4.KT52 and responder AKT6.A5.963.AJ84: the off
   arm's `7NT` made. `opener_over_raise` uses `slam::rkcb_rows`. Try
   `slam::grand_rkcb_rows`; its module doc gives the side-king bar.
6. **Flat 4-3-3-3 under the `4NT` major raise.** The instinct test
   `fit_sum_leaves_a_flat_4333_in_notrump` (South KT87.AQ2.Q75.J82 over
   `- 1♣ - 1♥ - 2♠ -`) shows the book now bids `4NT` where the floor kept
   `3NT`. The test now arms the knob off (`jump_shift_floor`). Decide
   whether `points(11..)` should exclude 4333 (the curse-of-4333 note) or
   whether the keycard ask is right there. Neither has been measured.
7. **The `3♠` jump raise's slam tools** (170 boards, −434 on the anchor):
   BBA's splinters, and `4NT` on 11–13 with shape, reach slams our `4♠`
   sign-off misses. This is a separate sub-node from the jump shift. Give
   it its own knob, and author its `(X)` tails.

Not an item: `1♦ - 1♥ - 3♣ - 3♥ - 3NT` against `3NT` played by the other hand
(−13 / −11) is right-siding. DD prices the lead, and no call is wrong.

## Checklist (per [measurement.md](measurement.md))

- Author each rung with both tails (`-`, `(X)`). Any artificial call gets an
  `.alert(...)` plus its reading. Re-bless `tests/fixtures/alert-sites.txt`
  if the alert counts move, and re-pin the `authored_calls_read_what_they_gate`
  leak meter with a ledger line.
- One fresh `SEED_BASE` per experiment, arms sequential through
  `scripts/idle-run.sh`, a fresh results dir (`ab-results/strong-jump-rebids-r3`).
- Plain DD is the verdict, PD the guard. Trace the worst boards before
  calling a loss dead.
