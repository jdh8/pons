#!/usr/bin/env bash
#
# ben-servers.sh — launch/stop the BEN gameapi fleet for ben-gen.
#
# One server instance per ben-gen shard process (bidding is serialized behind
# a per-instance lock), ports 8085+i, all under idle-run.sh politeness — the
# servers are the real load on this shared box (docs/shared-machine-data-gen.md).
# RSS is ~1.0 GB/instance.
#
# Usage:
#   scripts/ben-servers.sh start N [f|s]   # N instances, tier f (default) or s
#   scripts/ben-servers.sh stop            # stop every BEN server we started
#   scripts/ben-servers.sh status          # probe each port with a fixed /bid
#
# Tier f = vendor/ben/BEN-21GF-F.conf (pure policy); tier s = BEN's stock
# BEN-21GF.conf. The BEN checkout lives at $BEN_DIR (default ~/ben — the path
# must NOT contain "/src", see docs/ben-gen-design.md).
#
# IRON RULE: never restart or upgrade servers mid-experiment — finish or kill
# the run first (the analog of no-rebuild-during-A/B).
#
set -euo pipefail

BEN_DIR="${BEN_DIR:-$HOME/ben}"
BASE_PORT=8085
here="$(dirname "$(readlink -f "$0")")"
probe='hand=AK97543.K.T3.AK7&seat=N&dealer=N&vul=&ctx='

probe_port() {
	curl -s -m 30 "http://localhost:$1/bid?$probe" 2>/dev/null | grep -q '"bid"'
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
	mkdir -p "$BEN_DIR/run"
	echo "BEN $(git -C "$BEN_DIR" describe --tags) tier=$tier"
	sha256sum "$conf"
	for ((i = 0; i < n; i++)); do
		port=$((BASE_PORT + i))
		if probe_port "$port"; then
			echo "port $port: already serving — leaving it alone" >&2
			continue
		fi
		(
			cd "$BEN_DIR/src"
			setsid nohup "$here/idle-run.sh" "$BEN_DIR/.venv/bin/python" gameapi.py \
				--config "$conf" --port "$port" --seed 42 --nolimit true --record false \
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
	;;
stop)
	# ponytail: pattern-kill — ours are the only gameapi.py processes on this box
	pkill -u "$USER" -f 'gameapi\.py' && echo "stopped" || echo "nothing running"
	;;
status)
	for port in $(pgrep -u "$USER" -af 'gameapi\.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -n); do
		if probe_port "$port"; then echo "port $port: OK"; else echo "port $port: NOT ANSWERING"; fi
	done
	ps -u "$USER" -o rss=,args= | awk '/gameapi\.py/ {for (i = 1; i <= NF; i++) if ($i == "--port") port = $(i + 1); printf "port %s: %.1f GB RSS\n", port, $1 / 1048576}' | sort
	;;
*)
	sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//;/^set -euo/d'
	exit 1
	;;
esac
