#!/usr/bin/env bash
#
# ben-servers.sh — launch/stop the BEN gameapi fleet for ben-gen.
#
# One server instance per ben-gen shard process (bidding is serialized behind
# a per-instance lock), ports 8085+i, all under idle-run.sh politeness — the
# servers are the real load on this shared box (docs/shared-machine-data-gen.md).
# RSS is ~1.0 GB/instance. Each instance is pinned to one TF thread (batch-of-1
# LSTM inference gains nothing from a pool); Tier-F fleet throughput saturates
# ~9-10 boards/sec by ~N=16, so N=24 ties N=32 — use 24 and keep the headroom.
#
# Usage:
#   scripts/ben-servers.sh start N [f|s]   # N instances, tier f (default) or s
#   scripts/ben-servers.sh stop            # stop every BEN server we started
#   scripts/ben-servers.sh status          # probe each port with a fixed /bid
#
# Tier f = vendor/ben/BEN-21GF-F.conf (pure policy); tier s = BEN's stock
# BEN-21GF.conf. The BEN checkout lives at $BEN_DIR (default ~/ben — the path
# must NOT contain "/src", see docs/ben-gen-design.md). $BEN_PYTHON overrides
# the interpreter (default $BEN_DIR/.venv/bin/python), e.g. a wrapper that runs
# it under a newer glibc loader.
#
# TEMP FILES.  Each instance copies its models into ~28 mkdtemp dirs (~0.3 GB)
# that it never removes, so a fleet started and stopped a few times fills the
# system /tmp (2026-10-19: 6,000 dirs, a full root disk, 15 of 48 servers dead
# at load).  The fleet gets its own TMPDIR, $BEN_TMPDIR (default
# $BEN_DIR/run/tmp), which `stop` removes; put it on a local disk if $BEN_DIR
# is on NFS.
#
# IRON RULE: never restart or upgrade servers mid-experiment — finish or kill
# the run first (the analog of no-rebuild-during-A/B).
#
# SERVER MANIFEST.  Once every port answers, `start` writes
# $BEN_DIR/run/servers.meta: what the fleet was launched from — BEN's commit,
# a hash of its working-tree patches (the PASS-level bidding.py, the nolimit
# gameapi.py, the shimmed _dds3.so) and of each untracked file, the conf, the
# interpreter and its package set, the server flags, the tier and the fleet
# size.  It is deterministic (no dates, no pids), so a restart from the same
# runtime rewrites it byte for byte.  ab-lib.sh fingerprints it into every
# ben-gen arm and the BEN runners' `--note` records its sha256, so a changed
# runtime refuses a resume; ben-gen-parallel.sh checks `-t` against its tier.
# `start` adopts an already-answering port only when servers.meta exists and
# matches this start, and `stop` removes it, so it never vouches for a fleet
# it does not describe.  It attests the tree at launch: edit $BEN_DIR only
# with the fleet stopped.
#
set -euo pipefail

BEN_DIR="${BEN_DIR:-$HOME/ben}"
BEN_PYTHON="${BEN_PYTHON:-$BEN_DIR/.venv/bin/python}"
BASE_PORT=8085
META="$BEN_DIR/run/servers.meta"
BEN_TMPDIR="${BEN_TMPDIR:-$BEN_DIR/run/tmp}"
SERVER_ARGS=(--seed 42 --nolimit true --record false)
here="$(dirname "$(readlink -f "$0")")"
probe='hand=AK97543.K.T3.AK7&seat=N&dealer=N&vul=&ctx='

probe_port() {
	curl -s -m 30 "http://localhost:$1/bid?$probe" 2>/dev/null | grep -q '"bid"'
}

# servers_meta TIER CONF N — the fleet manifest (see SERVER MANIFEST above).
# Every piece is an assignment, so a failed probe fails the start instead of
# hashing an empty string.
servers_meta() {
	local python head diff untracked conf packages sum
	python=$(command -v "$BEN_PYTHON") || {
		echo "ben-servers: BEN_PYTHON=$BEN_PYTHON not found" >&2
		return 1
	}
	head="$(git -C "$BEN_DIR" describe --tags --always) $(git -C "$BEN_DIR" rev-parse HEAD)" &&
		diff=$(git -C "$BEN_DIR" diff --binary HEAD | sha256sum) &&
		untracked=$(git -C "$BEN_DIR" ls-files --others --exclude-standard) || {
		echo "ben-servers: cannot read the BEN checkout at $BEN_DIR" >&2
		return 1
	}
	conf=$(sha256sum <"$2") || return 1
	packages=$("$BEN_PYTHON" -c 'import sys, importlib.metadata as m
print(sys.version)
print(*sorted(d.metadata["Name"] + "==" + d.version for d in m.distributions()), sep="\n")') || {
		echo "ben-servers: $BEN_PYTHON cannot list its packages" >&2
		return 1
	}
	echo "ben: $head"
	echo "ben-diff: ${diff%% *}"
	while read -r f; do
		case $f in '' | run/*) continue ;; esac # our logs and this manifest
		sum=$(sha256sum <"$BEN_DIR/$f") || return 1
		echo "untracked: ${sum%% *} $f"
	done <<<"$untracked"
	echo "tier: $1"
	echo "conf: ${conf%% *} $2"
	sum=$(sha256sum <"$python") || return 1
	echo "python: ${sum%% *} $python"
	sum=$(sha256sum <<<"$packages")
	echo "packages: ${sum%% *}"
	echo "args: ${SERVER_ARGS[*]}"
	echo "servers: $3"
}

case "${1:-}" in
start)
	n="${2:?usage: ben-servers.sh start N [f|s]}"
	tier="${3:-f}"
	case "$tier" in
	f) conf="$(readlink -f "$here/../vendor/ben/BEN-21GF-F.conf")" ;;
	s) conf="$BEN_DIR/src/config/BEN-21GF.conf" ;;
	*)
		echo "tier must be f or s, got '$tier'" >&2
		exit 1
		;;
	esac
	mkdir -p "$BEN_DIR/run" "$BEN_TMPDIR"
	want=$(servers_meta "$tier" "$conf" "$n")
	echo "$want"
	for ((i = 0; i < n; i++)); do
		port=$((BASE_PORT + i))
		if probe_port "$port"; then
			[ "$(cat "$META" 2>/dev/null)" = "$want" ] || {
				echo "port $port: already serving, but $META is missing or describes another runtime; ben-servers.sh stop first" >&2
				exit 1
			}
			echo "port $port: already serving — leaving it alone" >&2
			continue
		fi
		(
			cd "$BEN_DIR/src"
			# ponytail: batch-of-1 small-LSTM inference gains nothing from an intra-op
			# pool; pin to 1 so N instances pack onto N cores, not N*32 threads thrashing.
			setsid nohup env TMPDIR="$BEN_TMPDIR" TF_NUM_INTRAOP_THREADS=1 TF_NUM_INTEROP_THREADS=1 OMP_NUM_THREADS=1 PONS_IDLE_LOCAL= \
				"$here/idle-run.sh" "$BEN_PYTHON" gameapi.py \
				--config "$conf" --port "$port" "${SERVER_ARGS[@]}" \
				>"$BEN_DIR/run/server-$port.log" 2>&1 </dev/null &
		)
	done
	for ((i = 0; i < n; i++)); do
		port=$((BASE_PORT + i))
		for _ in $(seq 60); do
			probe_port "$port" && break
			sleep 3
		done
		probe_port "$port" || {
			echo "port $port: no answer after 3 min — see $BEN_DIR/run/server-$port.log" >&2
			exit 1
		}
		echo "port $port: ready"
	done
	printf '%s\n' "$want" >"$META.tmp"
	mv "$META.tmp" "$META"
	echo "wrote $META"
	;;
stop)
	rm -f "$META"
	# ponytail: pattern-kill — ours are the only gameapi.py processes on this box
	pkill -u "$USER" -f 'gameapi\.py' && echo "stopped" || echo "nothing running"
	# The servers' model copies (TEMP FILES above); give them a moment to exit.
	for _ in $(seq 10); do pgrep -u "$USER" -f 'gameapi\.py' >/dev/null || break; sleep 1; done
	rm -rf "$BEN_TMPDIR"
	;;
status)
	for port in $(pgrep -u "$USER" -af 'gameapi\.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -nu); do
		if probe_port "$port"; then echo "port $port: OK"; else echo "port $port: NOT ANSWERING"; fi
	done
	ps -u "$USER" -o rss=,args= | awk '/gameapi\.py/ {for (i = 1; i <= NF; i++) if ($i == "--port") port = $(i + 1); printf "port %s: %.1f GB RSS\n", port, $1 / 1048576}' | sort
	;;
*)
	sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//;/^set -euo/d'
	exit 1
	;;
esac
