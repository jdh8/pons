#!/bin/sh
# Regression for ab-lib.sh's arm resume guard: an arm's fingerprint is the files
# its generator runs, not the checkout.  Stub generators only; no builds.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp/scripts" "$tmp/target/release/examples" "$tmp/docs" "$tmp/src" "$tmp/examples" "$tmp/off"
cp "$repo/scripts/ab-lib.sh" "$repo/scripts/bba-gen-parallel.sh" "$tmp/scripts/"
cd "$tmp"

# gen PATH BUILD [old] — a stub bba-gen; BUILD stands for the compiled bytes,
# `old` for a generator from before `--provenance`.
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
while [ "\$#" -gt 0 ]; do
    case "\$1" in --output) out=\$2; shift ;; esac
    shift
done
echo '{}' >"\$out"
SH
    chmod +x "$1"
}
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
SEED_BASE=1
arm "$@"
SH
chmod +x scripts/run-arm.sh
git init -q && git add -A && git -c user.name=t -c user.email=t@t commit -qm init
export SKIP_BUILD=1 JOBS=2
unset BBA_GEN PONS_FLOOR_WEIGHTS FORCE_REUSE

fails=0
# [with VAR=VALUE] expect WANT NAME DIR [arm args...] — WANT is skip, generate
# or refuse.  The environment goes through `env`: bash-as-sh keeps a prefix
# assignment on a function call (`X=1 expect …`) past the call.
with=
expect() {
    want=$1; name=$2; dir=$3; shift 3
    # shellcheck disable=SC2086
    if env $with scripts/run-arm.sh "$dir" "$@" >"$tmp/out" 2>&1; then
        got=generate
        grep -q 'skip .*(complete)' "$tmp/out" && got=skip
    else
        got=refuse
        grep -q 'refuse' "$tmp/out" || got="error: $(cat "$tmp/out")"
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
with BBA_GEN=$tmp/off/old; expect generate 'pre-provenance generator' r old none
with BBA_GEN=$tmp/off/old; expect skip 'pre-provenance generator reused' r old none
gen off/old 1 old
with BBA_GEN=$tmp/off/old; expect refuse 'pre-provenance generator rebuilt' r old none

# A wrapper around a pre-provenance default build: the wrapper's text alone
# would miss the rebuild.
gen "$default" 1 old
with BBA_GEN=$tmp/wrapper; expect generate 'wrapper, pre-provenance build' r wold none
gen "$default" 2 old
with BBA_GEN=$tmp/wrapper; expect refuse 'wrapper, pre-provenance build rebuilt' r wold none
gen "$default" 1

mkdir -p r/pre-none && echo '{}' >r/pre-none/shard-0.json
expect refuse   'arm without arm.meta' r pre none

[ "$fails" = 0 ] || { echo "arm fingerprint: $fails case(s) failed" >&2; exit 1; }
echo 'arm fingerprint passed'
