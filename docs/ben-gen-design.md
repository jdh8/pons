# ben-gen — design for the pons↔BEN bidding harness

**Status: Phase 0 in progress (2026-07-16). Servers validated live; harness
being built.** This doc is the engineering half of the
[BEN gap campaign](ben-gap-campaign.md); read that first for the *why*.
Validation steps 1 (wire probe) and the environment half of step 2 are done —
flagged unknowns below have been replaced with measured facts from the pinned
tag running on this box.

## Goal and non-goals

`ben-gen` generates paired-table bidding-only dumps of `american()` (our
pair) vs **BEN** (their pair), byte-compatible with the existing `Dump`
schema, so every downstream consumer — `bba-score`, `ab-dump-diff`,
`ab-dump-sd`, `bba-decompose`, `scripts/anchor.sh`-style orchestration —
works unchanged.

Non-goals (v1): card play; feeding BEN's disclosed call meanings into our
inference (we read BEN through `Family::NATURAL`, exactly as we read BBA);
BEN-side replay verification; fleet distribution.

## Interface decision: REST `/bid`, not Blue Chip, not files

| Route | Verdict |
| --- | --- |
| **REST `GET /bid`** (`src/gameapi.py`, port 8085) | **Chosen.** Stateless "what do you bid" call: `hand`+`seat`+`dealer`+`vul`+`ctx` → JSON `{"bid": "1S", ...}`. Maps 1:1 onto our synchronous per-call auction loop (`bid_out`→`next_call`), same role `BbaOracle::classify` plays today. |
| Blue Chip table-manager protocol v18 (pons as TM, BEN's `table_manager_client.py --biddingonly` as client) | Deferred. Small protocol (CRLF ASCII over TCP, ~a dozen auction-phase templates) but adds 4 client processes + session state per table. **Worth building later**: it is the lingua franca that would also admit WBridge5, GIB, and BBA-as-member (`--TM_MEMBER`) — i.e. one Rust TM unlocks every yardstick in BBA's Table 1. Not needed to measure BEN. |
| File-based (`game.py --biddingonly` over `.ben` files) | Rejected: BEN bids both sides unless abusing the NS/EW replay trick; no per-call control. |

## BEN server operation

- **Pin release v0.8.8.4** (the build BBA's cross-engine table measured).
  Python 3.12 strict (uv-managed CPython 3.12.13), TF 2.18.1. Installed at
  **`~/ben`** with a `uv venv` at `~/ben/.venv` (GPL-3.0: BEN stays a
  separate process; never link or vendor its code into pons —
  process-boundary HTTP keeps pons MIT/Apache clean). Two install traps hit
  on this box (Ubuntu 22.04, glibc 2.35):
  - **The clone path must not contain `/src`**: `gameapi.py` derives the
    model base path as `getcwd().replace("/src", "")`, so a checkout under
    `~/src/ben` resolves models to `/home/jdh8/ben/models` and dies. Hence
    `~/ben`, where the substitution is correct by construction. Servers run
    with CWD `~/ben/src`.
  - **The vendored `_dds3.so` (DDS 3.0 Python extension) links
    `__isoc23_strtol@GLIBC_2.38`** — its *only* post-2.34 glibc symbol
    (`libdds.so`/`libEPBot.so` need ≤ 2.34 and load fine). One-time local
    fix, no rebuild: compile the forwarding shim
    `pons/vendor/ben/isoc23-shim.c` (build command in its header) into
    `~/ben/bin/dds3-linux/dds3/libisoc23shim.so`, then
    `uvx patchelf --clear-symbol-version __isoc23_strtol --add-needed
    libisoc23shim.so --set-rpath '$ORIGIN' _dds3.so`, then
    `pons/vendor/ben/fix-dds3-verneed.py` rewrites the now symbol-less
    strong `GLIBC_2.38` verneed entry to `GLIBC_2.34` (ld.so validates
    strong verneeds even when unreferenced). Pristine copy kept as
    `_dds3.so.orig`. Verified:
    `ddsolver.DDSolver()` constructs, Tier-S search bids answer.
  - BEN's BBA consultation uses **ctypes against the vendored native
    `bin/BBA/linux/x64/libEPBot.so`** at this tag (the .NET/pythonnet path
    was removed upstream) — works headlessly, no runtime needed; `/bid`
    responses carry BBA explanations.
- **Config = stock `src/config/BEN-21GF.conf` unmodified** for the strong
  tier (bidder `BEN-21GF-8730_2025-04-18-E30.keras`, in-repo weights). Stock
  is non-negotiable: it is the measured artifact from BBA's table — if it
  embeds BBA consultation (`consult_bba` nudges candidate scores by
  `bba_trust`), that is part of the engine we are chasing. Read the raw
  `.conf` at the pinned tag before coding; the summarized fetch paraphrased
  it.
- **Two tiers** (see campaign doc for usage):
  - **Tier S (strong)** — stock config, sampling + DD-rollout search on.
    The headline-anchor engine.
  - **Tier F (fast)** — **`pons/vendor/ben/BEN-21GF-F.conf`** (committed,
    provenance + stock sha256 in its header): exactly two edits from stock,
    `search_threshold = -1` (the code's pure-policy switch — one NN-argmax
    candidate, no sampling, no rollout) and `check_final_contract = False`
    (the only other sampler trigger once candidates can't exceed one).
    BBA keycard consult stays on — it is search-free engine behavior.
    Measured: **~0.1 s/bid** (10 sequential /bid ≈ 0.99 s). The per-fix
    A/B engine.
- **Launch flags**: `python gameapi.py --config <conf> --port <p> --seed 42
  --nolimit true --record false`. `--nolimit` is mandatory (default rate
  limit 100/min; Tier F needs ~600 req/min/instance) — **but the tag parses
  it and never applies it**; `pons/vendor/ben/nolimit.patch` (one line,
  `limiter.enabled = not nolimit`, ops-only) makes the documented flag work.
  `--record false` stops per-response logging on 100k-board runs. Never pass
  `tournament=` in requests (it mutates a racy server-global; the config
  default is IMPs, which is what we want).
- **One server instance per shard process**, ports `8085+i`: bidding is
  serialized per instance behind a global `model_lock_bid`, so parallelism =
  N instances × N ben-gen client processes, mirroring
  `bba-gen-parallel.sh`'s process-level sharding. A `scripts/ben-servers.sh
  start|stop N` launcher: nice -n19/SCHED_IDLE (this box is shared —
  [shared-machine-data-gen.md](shared-machine-data-gen.md) applies to the
  *servers*, they are the actual load), health-probe each port with a fixed
  `/bid` request before declaring ready. **RSS measured: ~1.0 GB/instance**
  (TF runtime dominates; nets are ~5 MB) — 8 instances ≈ 8 GB on the 61 GB
  box, not RAM-bound.
- **Never restart/upgrade servers mid-experiment** — the analog of the
  no-rebuild-during-A/B iron rule. Record BEN tag + config hash + startup
  seed in `gen_args`.

## Wire protocol (the whole integration surface)

Request — one HTTP/1.1 GET per BEN call, `Host: localhost` (wrong Host ⇒
silent HTTP 444):

```text
GET /bid?hand=AK97543.K.T3.AK7&seat=S&dealer=N&vul=NS&ctx=1C--1S
```

- `hand`: PBN suit order `S.H.D.C`, dots between suits. Always send full
  13-card hands (never `x` pips).
- `ctx`: dash-separated calls, dealer-anchored — tokens `P`, `X`, `XX`,
  bids `1C`…`7N` (e.g. `ctx=1S-X-XX-P`; empty for the opening call). The
  tag's parser also accepts the legacy 2-char concatenation, but the dash
  form is unambiguous and is what we emit. **Confirmed live at the tag.**
- `vul`: **absolute** — empty / `NS` / `EW` / `Both` (case-insensitive;
  `N-S`, `All` also accepted). The README's relative `@v@V` format is
  stale; `parse_vuln` at the tag is absolute. **Confirmed live: all four
  values → 200.**
- Response: JSON; take `bid` — tokens `"PASS"`, `"X"`, `"XX"`, `"1S"`…
  **confirmed live**. `{"message": "Bidding is over"}` ⇒ our loop desynced
  ⇒ abort. A malformed/inconsistent request returns `{"error": …}` — BEN
  validates that `seat` matches dealer+auction length, a free desync guard;
  treat any response without a `bid` key as fatal for the shard.
- Omit `details=true` (we only need the call; skips candidates/samples
  serialization). `explanation`/`alert` ride along for free — log-only in
  v1.

Determinism: with fixed server version+config+startup-seed, `/bid` is a pure
function of the request — the MC sampler is re-seeded per request from a
hash of the hand string (`np.random.default_rng(calculate_seed(hand))`).
BEN's historical nondeterminism (issue #40) is cured by this scheme. So
same seed ⇒ identical dump, and re-running a shard is an exact reproduction.

Error policy: transport error / non-200 / unparsable ⇒ retry with backoff
(3×), then **abort the shard loudly** — never silently substitute Pass; a
silent fallback biases the measurement and the shard is cheaply re-runnable
from its seed.

## The Rust side (`examples/ben-gen/`)

Mirror `bba-gen`'s anatomy; everything non-EPBot is reused as-is:

- **`BenOracle`**: the `BbaOracle` counterpart — a blocking HTTP call inside
  the same synchronous `classify(hand, vul, auction) → call` role, invoked
  from the sequential per-board loop (`bid_out(ours, ben, ours_is_ns, …)`,
  table_a ours-NS / table_b ours-EW, dealer rotating `boards.len() % 4`).
  bba-gen is already single-threaded per process, so a blocking client fits
  the identical control flow; our-side `thread_local!` knobs stay on the
  main thread as today.
- **HTTP client**: `std::net::TcpStream` + hand-rolled GET + `serde_json`
  response parse — the query strings contain no characters needing escaping,
  and requests are serialized server-side anyway, so one
  connection-per-request (`Connection: close`) to localhost is fine.
  **Zero new dependencies.** If keep-alive/1.1 parsing gets annoying, `ureq`
  is the fallback — a new dep needs justifying at review.
- **Knobs**: v1 takes the *default* stance plus `--count/--seed/--vul/
  --output/--port/--tier` only. Do **not** port bba-gen's ~120 flags;
  add a knob when a measurement needs it.
- **Dump**: same `Dump { our_label, their_label, vulnerability, seed,
  gen_args, boards }`. `their_label` = e.g. `"BEN v0.8.8.4 21GF/S"` (tier
  suffix matters — a Tier-F dump must never be mistaken for a Tier-S one).
  One cosmetic fix while here: `bba-decompose` hardcodes
  `"our american floor"`/`"BBA 2/1"` in its report headline — read
  `Dump.{our,their}_label` instead.
- **Calibration mode**: `ben-gen --calibrate-epbot` wires **EPBot vs BEN**
  (no pons at either table: table_a EPBot-NS/BEN-EW, table_b mirrored),
  reusing `BbaOracle` from bba-gen (factor it into `examples/common/` or a
  small shared module). This is validation step 4 below and the resolver
  for the vendored-EPBot-vintage question.

Scoring/DD is untouched: divergence-only DD solve on the main thread via
`Solver::lock()`, plain + PD brackets from `src/scoring.rs`, sd via
`ab-dump-sd` — all consume the dump, none know about BEN.

## Throughput budget (estimates — smoke run calibrates)

Per board, BEN bids ~half the calls at each of two tables ≈ one full
auction's worth of BEN calls, so README-speed's per-board figures apply
roughly per matched board. With 8 instances/shards:

| Run | Tier | Boards | Wall |
| --- | --- | --- | --- |
| Smoke / probes | F | 100 | ~1.5 min (measured: 0.92 s/board/instance) |
| Ship A/B arm | F | 102.4k (8×6,400×2 vul) | ~3.5 h (est. from smoke) |
| Decompose sweep | F | 102.4k | ~3.5 h (est.) |
| Headline anchor | S | 20k (8×1,250×2 vul) | ~8–10 h est. (overnight) |

BBA's own tables quote ±0.04 IMP/deal at 20k hands; against a starting gap
estimated at ~2 IMPs/board, a 20k Tier-S anchor is precision to spare. Small
per-fix effects (±0.01) resolve at Tier-F scale.

## Validation plan (ordered; each gates the next)

1. **Live probe at the pinned tag** — **DONE 2026-07-16**: vul absolute,
   response tokens `PASS`/`X`/`XX`/bids, "Bidding is over" + `{"error"}`
   behavior, `/bid` deterministic on repeat, Tier F ~0.1 s/bid, RSS
   ~1.0 GB/instance. Facts folded into the sections above.
2. **100-board smoke** — **DONE 2026-07-16**: auctions legal and sane;
   **0.92 s/board** at Tier F (one instance); the dump feeds `bba-score`
   end-to-end unchanged.
3. **Determinism check** — **DONE 2026-07-16**: same seed twice ⇒ identical
   `boards` (the only byte diff is `gen_args` echoing the differing
   `--output` argv, by design).
4. **EPBot-vs-BEN calibration vs BBA's Table 1**: `--calibrate-epbot`,
   Tier S, 10–20k boards. Published reference: EPBot v.8741 scores
   **−0.51 SD / −0.38 DD** vs BEN v0.8.8.4 (21GF card) per deal. Acceptance
   is sign + rough magnitude (say DD in −0.2…−0.55): our protocol differs
   (deal stream, sd machinery) and our vendored EPBot's build vintage is
   unknown (DLL reports only 11.0.0.0), so exact agreement is not expected.
   A wildly-off number means harness bug or EPBot vintage mismatch —
   investigate before trusting any pons-vs-BEN number. This step
   independently validates the harness with **zero pons code in the loop**.
5. **First pons-vs-BEN anchor**: Tier S, 20k boards, fresh `SEED_BASE`,
   persisted like the BBA anchor series — this replaces the survey's
   chained ≈2.1-behind estimate with a measurement. Hand off to the
   campaign doc.

## Work estimate

Encoder/decoder + `BenOracle` + loop + dump writer ≈ 1–2 days (most of the
harness is reuse); `ben-servers.sh` + venv setup ≈ half a day; validation
runs ≈ 2–3 nights of shared-box idle compute. The Blue Chip TM route, if we
later want WBridge5/GIB under the same roof, is an independent ~2–4 day
follow-up.

## Open questions (resolve during validation, not before)

1. ~~v0.8.8.4 tag's `/bid` parameter dialect~~ — RESOLVED (step 1): vul
   absolute; tokens `P`/`X`/`XX` in ctx, `PASS`/`X`/`XX` in responses.
2. ~~BEN server RSS~~ — RESOLVED: ~1.0 GB/instance; cold start ~30 s to
   first answered `/bid`.
3. ~~`BEN-21GF.conf` contents at the tag~~ — RESOLVED: stock enables
   `consult_bba`/`use_bba_rollout`/`use_bba_to_count_aces` (kept — part of
   the measured engine); Tier-F derivation is the two-edit conf in
   `vendor/ben/`.
4. ~~Linux BBA-consultation path~~ — RESOLVED: ctypes against vendored
   native `libEPBot.so` at this tag (pythonnet path removed upstream);
   works headlessly.
5. Vendored EPBot vintage vs the site's v.8741 — step 4 bounds it.
