#!/bin/sh
# Regression for same-second automatic seeds; no bidders, servers, or builds.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp/scripts" "$tmp/target/release/examples" "$tmp/bin"
cp "$repo/scripts/bba-gen-parallel.sh" "$repo/scripts/ben-gen-parallel.sh" "$tmp/scripts/"

cat >"$tmp/bin/date" <<'SH'
#!/bin/sh
case "$1" in
    +%s) echo 1800000000 ;;
    +%s%N)
        n=$(cat "$SEED_CLOCK_STATE")
        echo $((1800000000100000000 + n * 100000000))
        echo $((n + 1)) >"$SEED_CLOCK_STATE"
        ;;
    *) exit 1 ;;
esac
SH
cat >"$tmp/bin/pgrep" <<'SH'
#!/bin/sh
printf '%s\n' '100 gameapi.py --port 8000' '101 gameapi.py --port 8001'
SH
# A fake generator writes the seed it was dealt from; with --verify it accepts
# a fake shard that holds that seed.  Both parallel scripts verify each shard
# with the checkout's generator, so it doubles as bba-gen's checker and ben-gen.
cat >"$tmp/fake-gen" <<'SH'
#!/bin/sh
set -eu
verify=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --verify) verify=$2; shift ;;
        --seed) seed=$2; shift ;;
        --output|-o) out=$2; shift ;;
    esac
    shift
done
[ -z "$verify" ] || { [ "$(cat "$verify")" = "$seed" ]; exit; }
printf '%s\n' "$seed" >"$out"
SH
chmod +x "$tmp/bin/date" "$tmp/bin/pgrep" "$tmp/fake-gen"
cp "$tmp/fake-gen" "$tmp/target/release/examples/bba-gen"
cp "$tmp/fake-gen" "$tmp/target/release/examples/ben-gen"
export PATH="$tmp/bin:$PATH" SKIP_BUILD=1 JOBS=2 BBA_GEN="$tmp/fake-gen"
# The stub fleet's manifest, as ben-servers.sh writes it at start.
export BEN_DIR="$tmp/ben"
mkdir -p "$BEN_DIR/run"
printf 'tier: f\nservers: 2\n' >"$BEN_DIR/run/servers.meta"
export SEED_CLOCK_STATE="$tmp/clock"
echo 0 >"$SEED_CLOCK_STATE"
unset SEED_BASE

for engine in bba ben; do
    wrapper="$tmp/scripts/$engine-gen-parallel.sh"
    for run in first second; do
        "$wrapper" "$tmp/$engine-$run" 1 >"$tmp/log" 2>&1 || {
            cat "$tmp/log" >&2
            exit 1
        }
    done
    first=$(cat "$tmp/$engine-first/shard-0.json")
    second=$(cat "$tmp/$engine-second/shard-0.json")
    [ "$(cat "$tmp/$engine-first/shard-1.json")" = "$((first + 1))" ]
    [ "$(cat "$tmp/$engine-second/shard-1.json")" = "$((second + 1))" ]
    [ "$second" -gt "$((first + 1))" ] || {
        echo "$engine automatic shard ranges overlap: $first, $second" >&2
        exit 1
    }
    for run in first second; do
        SEED_BASE=42 "$wrapper" "$tmp/$engine-explicit-$run" 1 >"$tmp/log" 2>&1
        [ "$(cat "$tmp/$engine-explicit-$run/shard-0.json")" = 42 ]
        [ "$(cat "$tmp/$engine-explicit-$run/shard-1.json")" = 43 ]
    done
done
echo 'seed minting passed: same-second automatic runs are disjoint; explicit seeds replay'
