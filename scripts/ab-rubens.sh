#!/bin/sh
# ab-rubens.sh — does the Rubens advance layer earn its place?
#
# `set_rubens_advances` has been default-on since it was authored and was never
# measured against the natural ladder it replaced — docs/bidding-options.md
# files it as "baseline default; knob is the A/B off-arm".  A probe of our own
# bidder says the layer is largely dead at the node it is supposed to own
# (`probe-bba-constraints --mode rub-ch --ours`, 20k hands): over `(1♣) 1♥ -`
# the 2♦ "transfer into partner's hearts" is chosen 0.4% of the time and those
# hands hold **6-7 diamonds and 1-2 hearts** (natural diamonds), the 2♣
# "transfer to diamonds" holds no diamond suit at all (median 3), and the
# natural 2♥ raise the transfer layer is meant to abolish is alive at 13.4%.
# The reading then decodes those natural calls as artificial Rubens ones — a
# phantom-suit reading on our own partner.
#
# Two arms per vul, identical deals.  The layer LOST (see the CHANGELOG entry
# and docs/bidding-options.md), so the arms are expressed off the new default:
#   off   the shipped default (natural advance ladder; `rubens_reading` silent)
#   on    --ns-rubens (the transfer ladder + cue-raise, and their reading)
#
#   setsid nohup scripts/idle-run.sh scripts/ab-rubens.sh \
#       ab-results/rubens >ab-results/rubens.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-rubens.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== rubens start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (--ns-rubens vs default)"
for vul in none both; do
    arm on  "$vul" --ns-rubens
    arm off "$vul"
    diffpair on off "$vul"
done
log "=== rubens done"
