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

Wrap a long `generate` in `idle-run.sh` on a shared box. `dump-teacher --deals
all.txt` then reads that cached DD for free, so the training-row dump runs
cheaply on a single machine. Seed-shardable `.f32` dumps (`dump-search`, …) merge
the same way — concatenate the per-seed `.f32`/`.tags` in seed order, keeping the
sidecars' feature/layout/SHA in agreement.

### Continuous scavenging (systemd)

To keep a fleet box growing the database whenever it's idle, supervise the
one-shot `generate` with [`scripts/gib-scavenge.service`](../scripts/gib-scavenge.service)
instead of writing a daemon: it runs `gib generate` in the `SCHED_IDLE` class and,
via `Restart=always`, starts the next shard with a fresh random 64-bit seed each
time. Shards are named `shard-<seed>.txt` (so they stay reproducible) and land in
`~/gib-shards`; `cat` them whenever you want a merged database.

```sh
cargo build --release --example gib
cp scripts/gib-scavenge.service ~/.config/systemd/user/
systemctl --user daemon-reload && systemctl --user enable --now gib-scavenge
loginctl enable-linger "$USER"          # keep running across logout/reboot
```

The unit is single-instance by design — one shard already saturates every core,
so don't run several (the parallel-thrash caveat above applies to scavengers too).
It also **pauses itself when the disk gets low** (`GIB_MIN_FREE_KIB`, default
~20 GiB free) so a forgotten scavenger can't fill a shared `/home`; it deletes
nothing and resumes once you merge and remove old shards. Each pass is a fresh
~8.9 MB file, so it grows without bound until either you clean up or the guard
trips — `cat … > all.txt && rm ~/gib-shards/shard-*.txt` is the whole lifecycle.

## Etiquette

Check who is on first (`w` / `who`), prefer nights/weekends for full-throttle
runs, and give a heads-up before a multi-hour job. The dataset is regenerable, so
don't hoard old copies — delete and re-make from the seed when needed.
