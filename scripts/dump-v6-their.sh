#!/bin/sh
# v6 twin trained on the same cells/deals while reading BBA's disclosed
# Multi-Landy defenses, including the two parked Multi reading corrections.
set -eu
cd "$(dirname "$0")/.."
DUMP_OUT=target/corpus-v6-their DUMP_VS_BBA=true exec scripts/dump-v6.sh
