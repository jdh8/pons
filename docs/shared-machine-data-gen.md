# Running heavy data generation on a shared machine

The AI-bidder data-generation jobs (`dump-search`, `probe-grand`, and other
`--features search` examples) are **CPU-saturating**: the double-dummy solver
([`ddss`](https://crates.io/crates/ddss)) calls `SetMaxThreads(0)` — "use every
core" — with no caller-side thread knob, so each batch solve spins one worker per
hardware thread and pegs the whole box for hours.

On a machine shared with colleagues, the cap has to come from the **OS**, not the
app. The policy below is what we use; it's wrapped in [`scripts/idle-run.sh`](../scripts/idle-run.sh).

## The policy: SCHED_IDLE, no quota

For a box that is **idle most of the time**, run the job in the `SCHED_IDLE`
scheduling class (plus the idle I/O class) and set **no CPU quota**:

```sh
scripts/idle-run.sh cargo run --release --features search \
  --example dump-search -- --boards 10000 --seed 1 --progress
# expands to:  nice -n10  chrt --idle 0  ionice -c3  <command>
```

This is the *scavenger* pattern:

- **Uses 100% of spare capacity.** With no quota, a quiet box runs the job at
  full speed — a quota would just waste idle cores for no benefit.
- **Yields instantly when anyone shows up.** `SCHED_IDLE` tasks only get a core
  when no normal-priority task is runnable on it; a colleague's task preempts
  yours the moment it wakes there.

### Why SCHED_IDLE rather than `nice -19`

`nice -19` is the *same scheduling class* as everyone else, just at the lowest
weight — it still competes, and (see below) it does **not** reliably yield across
*users*. `SCHED_IDLE` is a distinct class strictly below all normal tasks:

- runs only on otherwise-idle CPU, with lower preemption latency;
- needs **no privilege** (de-prioritizing yourself is always allowed); and
- is **inherited by child processes**, so wrapping the parent (`cargo`) covers
  every solver thread it spawns.

We *also* prepend a cosmetic `nice -n10`: `SCHED_IDLE` ignores the nice value for
scheduling, but it's still stored on the task, so it shows up low-priority (blue) in `htop`.

Verify it took effect: `chrt -p <pid>` should print `SCHED_IDLE`.

## What this does NOT cover

Scheduling priority arbitrates **CPU time on a core**. It does nothing about the
*shared* resources that often matter more, so even a perfectly-yielding job can
still slow a neighbour while both are actually running:

1. **Turbo / clock.** Many busy cores force the CPU down to all-core base clock,
   so a colleague's single-threaded job loses its turbo headroom (a silent
   ~20–40% tax). No priority or cgroup setting fixes this.
2. **Last-level cache & memory bandwidth.** Double-dummy search is
   transposition-table-heavy; flooding cores thrashes the shared L3 (on a 3D
   V-cache part, the very cache a latency-sensitive neighbour may depend on) and
   saturates memory bandwidth. Neither `nice` nor `cpu.weight` arbitrates these.
3. **Cross-user fairness.** On a modern `systemd` + cgroup-v2 box, CPU is split
   **per user slice first** (each `user-UID.slice` defaults to `cpu.weight=100`),
   and only *within* a slice does per-task priority apply. So against another
   *active user's parallel* job the kernel tends toward a ~50/50 split by slice,
   and your `nice`/`SCHED_IDLE` only re-orders your own tasks. (For the common
   case — a colleague's light or single-threaded job — they get what they need
   regardless, so this rarely bites.)

Because of (1) and (2), prefer to **scale the thread/core count to the current
load** and run off-hours when you can, rather than assuming priority makes a
flat-out box invisible.

## Do not run your own idle-run jobs in parallel

Each `--features search` job already spins **one worker per hardware thread**
(`SetMaxThreads(0)`), so it owns the whole box by itself. Launching several at
once (e.g. four A/B variants in the background with `&`) is **N× self-
oversubscription**: your own equal-priority threads thrash against each other, and
`SCHED_IDLE` does *not* arbitrate this — it only deprioritizes you against
*normal*-priority users, not against your other idle tasks. Measured: four
concurrent 500k `ab-landy` runs drove load to ~126 on 32 cores and produced *zero*
output in ~20 min — slower wall-clock than running them one after another, where a
single run saturates the cores cleanly. **Chain multi-config sweeps sequentially**
(`for … do …; done` or `&&`); only genuinely single-threaded jobs are safe to
idle-run in parallel.

## Seed hygiene: fresh hands per experiment

Reusing the same small seeds (`--seed 0..31`, the old `bba-gen-parallel.sh`
default) across many experiments **oversamples the same deal slice**. Each
`StdRng::seed_from_u64(seed)` is an independent stream, so `--seed 5 --count C`
gives the *same* C deals every run; replay it for every A/B and your results stop
being fresh draws from the deal space — they converge on whatever those fixed
streams happen to contain, and a treatment can look good (or bad) just by fitting
that slice. **Poll new hands for each experiment.**

The rule:

- **Each experiment uses a fresh `SEED_BASE`** (default: `date +%s`). Shard *i*
  draws `--seed (SEED_BASE + i)`. `bba-gen-parallel.sh` does this automatically
  and echoes the base it chose.
- **One base per experiment, shared across its arms.** A paired `ab-dump-diff`
  (or any A/B) is only valid if the arms it compares saw *identical* deals — so
  set the base once and reuse it for every arm of the same experiment:

  ```sh
  export SEED_BASE=$(date +%s)            # one base for the whole experiment
  for arm in base +feat; do
      scripts/idle-run.sh scripts/bba-gen-parallel.sh out/$arm 6400 $arm_flags
  done
  ab-dump-diff out/+feat/merged.json out/base/merged.json --score pd
  ```

  Do **not** let each arm pick its own `date +%s` (calling the script per-arm
  without exporting `SEED_BASE` does exactly that) — the arms would then bid
  *different* deals and the pairing is meaningless.
- **The next experiment gets a new base** — a fresh `date +%s` → fresh hands.
  Bases must differ by ≥ `nproc` to keep their shard ranges disjoint; since a real
  experiment takes minutes, a per-experiment `date +%s` guarantees this. For
  genuinely back-to-back runs, bump the base by hand (`SEED_BASE=$(( $(date +%s) + 1000 ))`).
- **Record the echoed `SEED_BASE` + the git SHA** to reproduce a run; that pair
  (plus count and flags) regenerates the exact dataset.

The `gib-scavenge` unit already follows the spirit of this — it starts each shard
with a fresh random 64-bit seed (`shard-<seed>.txt`), never a fixed small one.

## When to add a hard cap

If the box is **reliably busy** (not our usual case), priority isn't enough — add
a kernel-enforced ceiling via a transient `systemd` scope:

```sh
# ~6 cores' worth, low cross-user weight, RAM guard, still idle-class within it:
systemd-run --user --scope -p CPUQuota=600% -p CPUWeight=10 -p MemoryMax=12G \
  scripts/idle-run.sh <command>
```

- `CPUQuota=600%` — hard ceiling of 6 cores of CPU-time (kernel-enforced).
- `CPUWeight=10` — lowers *your slice's* share so it actually yields to other
  **users** (the lever `nice` lacks; see caveat 3).
- `MemoryMax` — guard so a runaway can't OOM colleagues.

## Surviving disconnect & checkpointing

- Run inside `tmux`/`screen`, or detach with
  `setsid nohup scripts/idle-run.sh <command> >run.log 2>&1 < /dev/null &`.
- The datasets are reproducible from `--seed` + the git SHA recorded in the
  `.json` sidecar, so **shard by seed** (`--seed 1`, `--seed 2`, …, with distinct
  `--out`) for natural checkpoints you can stop and resume, instead of one
  monolithic run.

## Spreading across machines

Seed-shardability also makes the job **distributable** — distinct seeds are
disjoint and their outputs concatenate, so no daemon, queue, or coordinator is
needed. Run one shard per machine with a distinct `--seed` and merge; a faster
box just takes a larger count.

The cached double-dummy database is the cleanest case. `gib generate` writes a
portable GIB file (`<West-first PBN>:<20 hex DD>` per deal) — plain text whose
deals are reproducible from `--seed` — so merging is literally `cat`:

```sh
gib generate --count 50000 --seed 1 --out shard-1.txt   # one per machine, distinct seeds
cat shard-*.txt > all.txt                               # order-independent, no dedup needed
gib verify all.txt                                       # optional: re-solve and confirm
```

Name the output `.pdd` instead of `.txt` and `generate` writes the compact
binary format (`pons::pdd`, 34 bytes/deal vs 89 — 2.6× smaller); readers sniff
the magic, so every consumer accepts either. Merge binary shards with
`gib convert shard-*.pdd --out all.pdd` (also converts text ↔ binary) rather
than `cat`, since each `.pdd` carries a header.

Wrap a long `generate` in `idle-run.sh` on a shared box. `dump-teacher --deals
all.txt` then reads that cached DD for free, so the training-row dump runs
cheaply on a single machine. Seed-shardable `.f32` dumps (`dump-search`, …) merge
the same way — concatenate the per-seed `.f32`/`.tags` in seed order, keeping the
sidecars' feature/layout/SHA in agreement.

### Continuous scavenging

To keep a machine growing the database whenever it's idle, supervise the
one-shot `generate` instead of writing a daemon. The shared
[`scripts/gib-scavenge.sh`](../scripts/gib-scavenge.sh) worker starts the next
shard with a fresh random 64-bit seed each time. Shards are named
`shard-<seed>.pdd` (compact binary by default, so they stay reproducible) and
land in `~/gib-shards`; merge them with `gib convert shard-*.pdd --out all.pdd`
whenever you want a combined database. Set `GIB_EXT=txt` for `cat`-mergeable
GIB text, or `GIB_COUNT` to change the 1M-deal shard size.

The worker is single-instance by design — one shard already saturates every
core, so don't run several (the parallel-thrash caveat above applies to
scavengers too). It also **pauses itself when the disk gets low**
(`GIB_MIN_FREE_KIB`, default ~20 GiB free) so a forgotten scavenger can't fill
the target filesystem; it deletes nothing and resumes once you merge and remove
old shards. Each pass is a fresh ~34 MB `.pdd` file (1M deals), so it grows
without bound until either you clean up or the guard trips — `gib convert
~/gib-shards/shard-*.pdd --out all.pdd && rm ~/gib-shards/shard-*.pdd` is the
whole lifecycle.

#### Linux (systemd)

[`scripts/gib-scavenge.service`](../scripts/gib-scavenge.service) runs the
worker in Linux's `SCHED_IDLE` CPU class plus the idle I/O class. Its
`Restart=always` policy brings the worker back after a crash or reboot.

```sh
cargo build --release --example gib
cp scripts/gib-scavenge.service ~/.config/systemd/user/
systemctl --user daemon-reload && systemctl --user enable --now gib-scavenge
loginctl enable-linger "$USER"          # keep running across logout/reboot
```

#### macOS (launchd)

macOS has no exact equivalent of Linux's `SCHED_IDLE`. The LaunchAgent in
[`scripts/gib-scavenge.plist`](../scripts/gib-scavenge.plist) uses Darwin's
lowest nice priority and low-priority I/O, but deliberately does **not** use
Darwin's `Background` process classification. DDS auto-configures one worker per
hardware core; on Apple Silicon, background QoS steers those workers onto the
efficiency-core cluster (ten DDS workers contending for six E-cores on a base
M4) and leaves the performance cores unused. Normal classification lets DDS use
all core types, while `Nice=19` still gives ordinary work CPU priority over the
scavenger. This is only an approximation: unlike `SCHED_IDLE`, nice shares the
normal scheduling class, and using every core can still affect thermals, turbo
headroom, cache, and memory bandwidth.

The checked-in plist assumes the checkout is at `$HOME/src/pons`, matching the
Linux example above. Install it as a per-user LaunchAgent and switch it on:

```sh
LABEL=org.jdh8.pons.gib-scavenge
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
TARGET="gui/$(id -u)/$LABEL"

cargo build --release --example gib
mkdir -p "$HOME/Library/LaunchAgents" "$HOME/Library/Logs/pons"
cp scripts/gib-scavenge.plist "$PLIST"
launchctl enable "$TARGET"
launchctl bootstrap "gui/$(id -u)" "$PLIST"
```

Switch it off until explicitly re-enabled. Killing the worker PID is not an off
switch: `KeepAlive` would immediately launch another one.

```sh
launchctl disable "gui/$(id -u)/org.jdh8.pons.gib-scavenge"
launchctl bootout "gui/$(id -u)/org.jdh8.pons.gib-scavenge"
```

Switch it back on, inspect it, follow its log, or restart an enabled and loaded
worker after changing the script:

```sh
# Switch back on
launchctl enable "gui/$(id -u)/org.jdh8.pons.gib-scavenge"
launchctl bootstrap "gui/$(id -u)" \
  "$HOME/Library/LaunchAgents/org.jdh8.pons.gib-scavenge.plist"

# Status and live log
launchctl print "gui/$(id -u)/org.jdh8.pons.gib-scavenge"
tail -f "$HOME/Library/Logs/pons/gib-scavenge.log"

# Restart an enabled, loaded worker
launchctl kickstart -k "gui/$(id -u)/org.jdh8.pons.gib-scavenge"
```

To uninstall, switch it off as above and remove the installed copy:

```sh
rm "$HOME/Library/LaunchAgents/org.jdh8.pons.gib-scavenge.plist"
```

The `GIB_OUT`, `GIB_MIN_FREE_KIB`, `GIB_COUNT`, and `GIB_EXT` knobs still apply,
but environment variables exported in a terminal do not alter an already loaded
LaunchAgent. To override them, add an `EnvironmentVariables` dictionary to the
installed plist's top-level `<dict>`, then switch the agent off and back on:

```xml
<key>EnvironmentVariables</key>
<dict>
    <key>GIB_OUT</key>
    <string>/Volumes/data/gib-shards</string>
    <key>GIB_MIN_FREE_KIB</key>
    <string>10485760</string>
    <key>GIB_COUNT</key>
    <string>1000000</string>
    <key>GIB_EXT</key>
    <string>pdd</string>
</dict>
```

A per-user LaunchAgent starts after login and stops at logout. It pauses while
the Mac sleeps and resumes after wake. Running continuously before login or
after logout instead requires a root-installed LaunchDaemon with an explicit
`UserName`; that privileged setup is intentionally outside this per-user recipe.

## Etiquette

Check who is on first (`w` / `who`), prefer nights/weekends for full-throttle
runs, and give a heads-up before a multi-hour job. The dataset is regenerable, so
don't hoard old copies — delete and re-make from the seed when needed.
