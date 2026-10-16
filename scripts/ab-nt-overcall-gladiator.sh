#!/bin/sh
# ab-nt-overcall-gladiator.sh — A/B for Gladiator advances of our 1NT overcall of
# their MAJOR (`set_nt_overcall_gladiator`, docs/bba-gap-campaign.md def-r1 lever):
# a `2♣` weak relay + a cue-of-their-major Stayman for the one unbid major +
# natural INV + splinter / Leaping-Michaels, replacing the opening-1NT graft over
# major openings only.  OFF = shipped default (systems-on opening-1NT graft over
# majors); ON = --ns-nt-overcall-gladiator.  Both arms keep systems-on for the
# MINOR openings (untouched), so the minor split is a ~0 sanity check and the
# major split is the real test.  Both vulnerabilities, three scorers (plain+pd via
# ab-dump-diff, sd via ab-dump-sd), arms sequential, one shared SEED_BASE.  Do NOT
# touch the codebase while it runs (bba-gen-parallel re-invokes cargo build; must
# stay a no-op).
#
#   BOARDS=204800 SPLIT=<split_by_opening.py> setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-overcall-gladiator.sh ab-results/nt-overcall-gladiator \
#       >ab-results/nt-overcall-gladiator.log 2>&1 &
R=${1:?usage: ab-nt-overcall-gladiator.sh RESULTS_DIR}
SPLIT=${SPLIT:?set SPLIT to the split_by_opening.py path}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Shards are split by opening strain, so these score $R/ARM-VUL-KIND copies of
# each arm (not the name-based $R/$name-$vul convention).  They are not
# ab-lib's diffpair/sddiff (which they once shadowed with bare `-s` guards):
# each report reuses only while its .inputs stamp -- both source arms'
# manifests, the split script's and the scorer's bytes, the kind and flags --
# still matches, the stamp written after the report.  ab-dump-diff/-sd fold a
# whole strain dir's shards into one solve.
# splitdiff ON OFF VUL KIND — plain + pd over $R/ON-VUL-KIND vs $R/OFF-VUL-KIND
splitdiff() {
    on=$1; off=$2; vul=$3; kind=$4; tag=$vul.$kind
    plain="$R/diff.$tag.plain.txt"
    pd="$R/diff.$tag.pd.txt"
    inputs=$(report_inputs "$DIFF" "$on" "$off" "$vul" --split "$(cksum <"$SPLIT")" "$kind" \
        --score both --show 0)
    [ -s "$plain" ] && [ -s "$pd" ] && fresh_report "$R/diff.$tag.inputs" "$inputs" \
        && { log "skip $plain + $pd (inputs unchanged)"; return 0; }
    log "diff $tag (plain+pd)"
    "$DIFF" "$R/$on-$vul-$kind" "$R/$off-$vul-$kind" --score both \
        --out-plain "$plain" --out-pd "$pd" --show 0 >>"$R/log" 2>&1
    printf '%s\n' "$inputs" >"$R/diff.$tag.inputs"
}

# splitsd ON OFF VUL KIND — sd-lead, 16 worlds; guarded on the result line, as
# ab-lib's sddiff is (a panic lands in the same file)
splitsd() {
    on=$1; off=$2; vul=$3; kind=$4; tag=$vul.$kind
    out="$R/sd.$tag.txt"
    inputs=$(report_inputs "$SD" "$on" "$off" "$vul" --split "$(cksum <"$SPLIT")" "$kind" \
        -v "$vul" --sd-worlds 16 --show 0)
    grep -q '^Delta' "$out" 2>/dev/null && fresh_report "$out.inputs" "$inputs" \
        && { log "skip $out (scored)"; return 0; }
    log "sd-diff $tag"
    "$SD" "$R/$on-$vul-$kind" "$R/$off-$vul-$kind" -v "$vul" --sd-worlds 16 --show 0 >"$out" 2>&1
    printf '%s\n' "$inputs" >"$out.inputs"
}

log "=== nt-overcall-gladiator A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-nt-overcall-gladiator
    python3 "$SPLIT" "$R/on-$vul"
    python3 "$SPLIT" "$R/off-$vul"
    for kind in minor major; do
        splitdiff on off "$vul" "$kind"
        splitsd   on off "$vul" "$kind"
    done
done
log "=== nt-overcall-gladiator A/B done"
