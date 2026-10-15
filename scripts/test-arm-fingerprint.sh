#!/bin/sh
# Regression for ab-lib.sh's arm resume guard: an arm's fingerprint is the files
# its generator runs, not the checkout.  Stub generators only; no builds.
set -eu
# Keep a caller's measurement settings from changing this isolated fixture.
unset BBA_GEN PONS_FLOOR_WEIGHTS FORCE_REUSE BOARDS PER_SHARD BUILD_EXTRA FEATURES
repo=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp/scripts" "$tmp/target/release/examples" "$tmp/docs" "$tmp/src" "$tmp/examples" "$tmp/off"
cp "$repo/scripts/ab-lib.sh" "$repo/scripts/bba-gen-parallel.sh" "$tmp/scripts/"
cd "$tmp"

# gen PATH BUILD [old] — a stub bba-gen; BUILD stands for the compiled bytes,
# `old` for a generator from before `--provenance`.  A stub shard holds
# "SEED COUNT"; `--verify` checks it (the default build is the checker), and
# FAIL_SEED makes the worker for that seed die without writing.
gen() {
    cat >"$1" <<SH
#!/bin/sh
# build $2
if [ "\$1" = --provenance ]; then
    [ "${3:-}" = old ] && exit 2
    echo "exe \$0"
    echo "lib epbot.so"
    [ -z "\${PONS_FLOOR_WEIGHTS:-}" ] || echo "\$PONS_FLOOR_WEIGHTS" | tr , '\n' | sed 's/^/weights /'
    exit 0
fi
verify=
while [ "\$#" -gt 0 ]; do
    case "\$1" in
        --output) out=\$2; shift ;;
        --verify) verify=\$2; shift ;;
        --seed) seed=\$2; shift ;;
        --count) count=\$2; shift ;;
    esac
    shift
done
[ -z "\$verify" ] || { [ "\$(cat "\$verify")" = "\$seed \$count" ]; exit; }
[ "\$seed" != "\${FAIL_SEED:-}" ] || exit 3
echo "\$seed \$count" >"\$out"
SH
    chmod +x "$1"
}
# shard SEED — a valid stub shard of the default PER_SHARD
shard() { echo "$1 6400"; }
default=target/release/examples/bba-gen
gen "$default" 1
cp "$default" target/release/examples/ab-dump-diff
cat >wrapper <<'SH'
#!/bin/sh
# a loader wrapper, as BBA_GEN on an old-glibc box
exec "$(dirname "$0")/target/release/examples/bba-gen" "$@"
SH
chmod +x wrapper
echo v1 >epbot.so
echo card1 >card.bbsa
echo w1 >w.f32
echo src >src/lib.rs && echo ex >examples/x.rs
echo lock >Cargo.lock
cat >scripts/run-arm.sh <<'SH'
#!/bin/sh
R=$1; shift
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=${ARM_SEED:-1}
arm "$@"
SH
chmod +x scripts/run-arm.sh
git init -q && git add -A && git -c user.name=t -c user.email=t@t commit -qm init
export SKIP_BUILD=1 JOBS=2

fails=0
cases=0
# [with VAR=VALUE] expect WANT NAME DIR [arm args...] — WANT is skip, generate
# or refuse.  The environment goes through `env`: bash-as-sh keeps a prefix
# assignment on a function call (`X=1 expect …`) past the call.
with=
expect() {
    want=$1; name=$2; dir=$3; shift 3
    cases=$((cases + 1))
    # shellcheck disable=SC2086
    if env $with scripts/run-arm.sh "$dir" "$@" >"$tmp/out" 2>&1; then
        got=generate
        grep -q 'skip .*(complete' "$tmp/out" && got=skip
    else
        got=refuse
        if grep -q 'FAILED' "$tmp/out"; then got=fail
        elif ! grep -q 'refuse' "$tmp/out"; then got="error: $(cat "$tmp/out")"; fi
    fi
    if [ "$got" = "$want" ]; then
        echo "ok   $name"
    else
        echo "FAIL $name: want $want, got $got" >&2
        fails=$((fails + 1))
    fi
    with=
}
# with VAR=VALUE... — the environment of the next expect only
with() { with="$*"; }

expect generate 'fresh arm' r a none
expect skip     'unchanged generator reused' r a none
echo doc >docs/x.md && git add -A && git -c user.name=t -c user.email=t@t commit -qm doc
echo edit >src/lib.rs
expect skip     'doc commit and unrebuilt source edit reused' r a none
git checkout -q src/lib.rs
gen "$default" 2
expect refuse   'rebuilt binary, same source' r a none
gen "$default" 1
expect skip     'original bytes back' r a none
expect refuse   'flags changed' r a none --foo
with FORCE_REUSE=1; expect skip 'FORCE_REUSE overrides' r a none --foo

rm r/a-none/shard-1.json
expect generate 'partial arm resumed, unchanged' r a none
rm r/a-none/shard-1.json
gen "$default" 2
expect refuse   'partial arm, rebuilt binary' r a none
gen "$default" 1

echo v2 >epbot.so
expect refuse   'EPBot library swapped' r a none
echo v1 >epbot.so

with BBA_GEN=$tmp/wrapper; expect generate 'wrapper arm' r w none
gen "$default" 2
with BBA_GEN=$tmp/wrapper; expect refuse 'wrapped binary rebuilt, wrapper unchanged' r w none
gen "$default" 1

printf '\n# changed launcher\n' >>wrapper
with BBA_GEN=$tmp/wrapper; expect refuse 'launcher edited, reported binary unchanged' r w none

gen off/bba-gen 0
with BBA_GEN=$tmp/off/bba-gen; expect generate 'override arm' r off none
expect refuse   'override arm resumed on the default build' r off none
gen off/bba-gen 9
with BBA_GEN=$tmp/off/bba-gen; expect refuse 'override build changed' r off none

expect generate 'card flag' r c none --our-card card.bbsa
echo card2 >card.bbsa
expect refuse   'card file edited' r c none --our-card card.bbsa
expect generate 'card flag, = form' r c2 none --our-card=card.bbsa
echo card3 >card.bbsa
expect refuse   'card file edited, = form' r c2 none --our-card=card.bbsa

with PONS_FLOOR_WEIGHTS=w.f32; expect generate 'weights arm' r wt none
echo w2 >w.f32
with PONS_FLOOR_WEIGHTS=w.f32; expect refuse 'weights blob edited' r wt none

gen off/old 0 old
with BBA_GEN=$tmp/off/old; expect refuse 'unreported fresh generator' r old none

# Historical metadata can match its old fingerprint exactly and still cannot
# prove which runtime files a legacy generator used. Cover both resume paths.
for state in complete partial; do
    arm_dir="r/legacy-$state-none"
    mkdir -p "$arm_dir"
    {
        echo "unreported exe $(cksum <off/old)"
        echo 'PONS_FLOOR_WEIGHTS='
        echo "wrapped $(cksum <"$default")"
        echo 'flags: '
    } >"$arm_dir/arm.meta"
    shard 1 >"$arm_dir/shard-0.json"
    [ "$state" != complete ] || shard 2 >"$arm_dir/shard-1.json"
    with BBA_GEN=$tmp/off/old; expect refuse "unreported $state arm, unchanged legacy metadata" r "legacy-$state" none
    if [ "$state" = complete ]; then want=skip; else want=generate; fi
    with BBA_GEN=$tmp/off/old FORCE_REUSE=1; expect "$want" "FORCE_REUSE unreported $state arm" r "legacy-$state" none
    if ! grep -q 'WARNING .*FORCE_REUSE=1.*unverified' "$tmp/out"; then
        echo "FAIL unreported $state arm: FORCE_REUSE omitted its unverified-identity warning" >&2
        fails=$((fails + 1))
    fi
done

# Explicit adapter for an alternate executable without --provenance. It names
# the actual executable, EPBot and every runtime weight blob before forwarding.
cat >reporting-wrapper <<'SH'
#!/bin/sh
if [ "$1" = --provenance ]; then
    echo 'exe off/old'
    echo 'lib epbot.so'
    [ -z "${PONS_FLOOR_WEIGHTS:-}" ] || echo "$PONS_FLOOR_WEIGHTS" | tr , '\n' | sed 's/^/weights /'
    exit 0
fi
exec "$(dirname "$0")/off/old" "$@"
SH
chmod +x reporting-wrapper
echo w1 >w.f32
echo other1 >other.f32
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect generate 'reported legacy wrapper' r reported none
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect skip 'reported legacy wrapper unchanged' r reported none
gen off/old 1 old
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect refuse 'alternate wrapped executable rebuilt' r reported none
gen off/old 0 old
echo w2 >w.f32
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect refuse 'legacy wrapper same-path weights edited' r reported none
echo w1 >w.f32
echo other2 >other.f32
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect refuse 'legacy wrapper second weights blob edited' r reported none
echo other1 >other.f32
echo v2 >epbot.so
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect refuse 'legacy wrapper EPBot swapped' r reported none
echo v1 >epbot.so
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect skip 'reported legacy runtime restored' r reported none
rm r/reported-none/shard-1.json
gen off/old 1 old
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect refuse 'partial legacy arm, alternate executable rebuilt' r reported none
gen off/old 0 old
echo w2 >w.f32
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect refuse 'partial legacy arm, same-path weights edited' r reported none
echo w1 >w.f32
echo v2 >epbot.so
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect refuse 'partial legacy arm, EPBot swapped' r reported none
echo v1 >epbot.so
with BBA_GEN=$tmp/reporting-wrapper PONS_FLOOR_WEIGHTS=w.f32,other.f32; expect generate 'partial legacy arm, runtime restored' r reported none

# Each invalid report gets a fresh arm and a second attempt. Assert the first
# refusal left no arm metadata or shards, so the repeated refusal cannot be a
# stale-metadata mismatch masquerading as provenance validation.
cat >bad-report <<'SH'
#!/bin/sh
if [ "$1" = --provenance ]; then
    cat report.txt
    exit 0
fi
exec "$(dirname "$0")/off/old" "$@"
SH
chmod +x bad-report
echo secret >unreadable.f32
chmod 000 unreadable.f32
bad_report() {
    label=$1; report=$2
    printf '%s\n' "$report" >report.txt
    with BBA_GEN=$tmp/bad-report; expect refuse "$label report, fresh arm" r "bad-$label" none
    if [ -e "r/bad-$label-none/arm.meta" ] || ls "r/bad-$label-none"/shard-*.json >/dev/null 2>&1; then
        echo "FAIL $label: invalid provenance left metadata or shards" >&2
        fails=$((fails + 1))
    fi
    with BBA_GEN=$tmp/bad-report; expect refuse "$label report, repeated attempt" r "bad-$label" none
}
bad_report blank ''
bad_report malformed 'unknown off/old'
bad_report missing-path 'exe'
bad_report missing-lib 'exe off/old'
bad_report missing-exe 'lib epbot.so'
bad_report missing-file 'exe off/missing
lib epbot.so'
bad_report unreadable-file 'exe off/old
lib epbot.so
weights unreadable.f32'
bad_report nonregular-file 'exe off/old
lib off'
chmod 600 unreadable.f32

mkdir -p r/pre-none && echo '{}' >r/pre-none/shard-0.json
expect refuse   'arm without arm.meta' r pre none

# The manifest pins the deal set, and every resume verifies the shards.
expect generate 'manifest arm' r m none
grep -q '^seed-base: 1$' r/m-none/arm.meta && grep -q '^# source: ' r/m-none/arm.meta \
    || { echo "FAIL manifest: arm.meta lacks seed-base or provenance lines" >&2; fails=$((fails + 1)); }
with ARM_SEED=5; expect refuse 'SEED_BASE changed' r m none
echo '1 64' >r/m-none/shard-0.json
expect generate 'corrupt shard regenerated' r m none
[ "$(cat r/m-none/shard-0.json)" = "$(shard 1)" ] \
    || { echo "FAIL corrupt shard: not regenerated" >&2; fails=$((fails + 1)); }
shard 3 >r/m-none/shard-2.json
expect generate 'stray shard past JOBS regenerated' r m none
[ ! -e r/m-none/shard-2.json ] \
    || { echo "FAIL stray shard: survived regeneration" >&2; fails=$((fails + 1)); }
sed '/^# /d' r/m-none/arm.meta >r/m-none/arm.meta.new && mv r/m-none/arm.meta.new r/m-none/arm.meta
expect skip     'provenance lines are not compared' r m none
grep -v -e '^shards:' -e '^per-shard:' -e '^seed-base:' r/m-none/arm.meta >r/m-none/arm.meta.new
mv r/m-none/arm.meta.new r/m-none/arm.meta
expect skip     'pre-manifest arm.meta, matching fingerprint' r m none
grep -q 'WARNING .*predates the manifest' "$tmp/out" \
    || { echo "FAIL pre-manifest arm.meta: no warning" >&2; fails=$((fails + 1)); }
with ARM_SEED=5; expect generate 'pre-manifest arm.meta, other seed: shard check regenerates' r m none

# A worker that dies fails the arm loudly and leaves a hole, never a shard.
with FAIL_SEED=2; expect fail 'failed worker' r f none
[ -e r/f-none/shard-0.json ] && [ ! -e r/f-none/shard-1.json ] \
    || { echo "FAIL failed worker: expected shard-0 only" >&2; fails=$((fails + 1)); }
expect generate 'failed arm regenerated on resume' r f none

# A report is reused only while its arms, scorer and flags are unchanged.
cat >target/release/examples/ab-dump-diff <<'SH'
#!/bin/sh
while [ "$#" -gt 0 ]; do
    case "$1" in --out-plain|--out-pd) echo diff >"$2"; shift ;; esac
    shift
done
SH
chmod +x target/release/examples/ab-dump-diff
cat >scripts/run-diff.sh <<'SH'
#!/bin/sh
R=$1
. "$(dirname "$0")/ab-lib.sh"
diffpair m f none
SH
chmod +x scripts/run-diff.sh
diff_expect() {
    cases=$((cases + 1))
    scripts/run-diff.sh r >"$tmp/out" 2>&1 || { echo "FAIL $2: $(cat "$tmp/out")" >&2; fails=$((fails + 1)); return; }
    got=rescore; grep -q 'skip .*inputs unchanged' "$tmp/out" && got=skip
    if [ "$got" = "$1" ]; then echo "ok   $2"; else echo "FAIL $2: want $1, got $got" >&2; fails=$((fails + 1)); fi
}
diff_expect rescore 'fresh report'
diff_expect skip    'report reused'
echo '# touched' >>r/f-none/arm.meta && sed -i 's/^# touched$/x: 1/' r/f-none/arm.meta
diff_expect rescore 'arm manifest changed'
echo '# rebuilt' >>target/release/examples/ab-dump-diff
diff_expect rescore 'scorer rebuilt'
rm r/diff.m.vs.f.none.inputs
diff_expect rescore 'unstamped report'

[ "$fails" = 0 ] || { echo "arm fingerprint: $fails case(s) failed" >&2; exit 1; }
echo "arm fingerprint passed ($cases cases)"
