#!/usr/bin/env bash
#
# fleet-relabel.sh — drive the M-series relabel across the boxes in
# ~/.config/pons/hosts (outside the tree; host names never enter git).
#
# hosts file, one box per line, offsets assigned in file order:
#   <ssh host> <weight> <chunk root on that box> [<bank path on that box>]
# e.g.
#   main      4 /home/me/pons-relabel
#   localhost 2 /home/me/pons-relabel /home/me/22.pdd     # this box, bank copied locally
#   worker-a  1 /nfs/shared/pons/relabel
# `localhost` runs commands locally.  STRIDE = the sum of weights; a box with
# weight w owns w consecutive residues.  The first host is the mop-up box.
#
# Usage:
#   scripts/fleet-relabel.sh provision [SHA]   fetch, checkout SHA (default: HEAD here), build, install the unit
#   scripts/fleet-relabel.sh start LAYOUTS     stop poker workers, write env files, (re)start pons-worker@m everywhere
#   scripts/fleet-relabel.sh status            unit state and finished-chunk count per box
#   scripts/fleet-relabel.sh collect           rsync this box's chunks into the first host's root
#   scripts/fleet-relabel.sh mopup LAYOUTS     start pons-worker@m-mopup on the first host (stride 1, reverse order)
# Then on the first host:  dump-teacher --cut 32 --chunks <root>... --out <corpus dir>
set -euo pipefail

HOSTS=${PONS_HOSTS:-$HOME/.config/pons/hosts}
RUN=${RUN:-m}
PIN=$HOME/.config/pons/relabel.sha

hosts=() weights=() outs=() banks=()
while read -r host weight out bank; do
	[[ -z $host || $host == \#* ]] && continue
	hosts+=("$host") weights+=("$weight") outs+=("$out") banks+=("${bank:-/nfs2/jdh8/pons/22.pdd}")
done <"$HOSTS"
((${#hosts[@]})) || { echo "fleet-relabel: no hosts in $HOSTS" >&2; exit 1; }
stride=0
for w in "${weights[@]}"; do stride=$((stride + w)); done

remote() { # host, command (run in ~/src/pons; a non-login ssh shell lacks ~/.cargo/bin)
	local cmd="export PATH=\$HOME/.cargo/bin:\$PATH; cd ~/src/pons && $2"
	if [[ $1 == localhost ]]; then bash -c "$cmd"; else ssh -o BatchMode=yes "$1" "$cmd"; fi
}
pinned() { cat "$PIN" 2>/dev/null || { echo "fleet-relabel: no pinned SHA — run provision first" >&2; exit 1; }; }
check_sha() { # host
	local have
	have=$(remote "$1" "git rev-parse HEAD")
	[[ $have == "$(pinned)" ]] || { echo "fleet-relabel: $1 is at $have, pinned $(pinned) — run provision" >&2; exit 1; }
}
env_file() { # host index, extra lines...
	local i=$1 out=${outs[$1]} other=() j
	shift
	for j in "${!outs[@]}"; do [[ $j != "$i" ]] && other+=("${outs[$j]}"); done
	printf 'OUT=%s\nBANK=%s\nLAYOUTS=%s\nEXTRA_OUT=%s\n%s\n' "$out" "${banks[$i]}" "$LAYOUTS" "$(IFS=:; echo "${other[*]}")" "$*"
}

cmd=${1:-}
case $cmd in
provision)
	sha=${2:-$(git rev-parse HEAD)}
	for h in "${hosts[@]}"; do
		echo "== $h: checkout $sha, build, install unit"
		remote "$h" "git fetch -q origin && git checkout -q --detach $sha && \
			cargo build -q --release --example dump-teacher --features dd && \
			mkdir -p ~/.config/systemd/user ~/.config/pons && \
			cp scripts/pons-worker@.service ~/.config/systemd/user/ && systemctl --user daemon-reload"
	done
	echo "$sha" >"$PIN"
	;;
start)
	LAYOUTS=${2:?start LAYOUTS}
	offset=0
	for i in "${!hosts[@]}"; do
		h=${hosts[$i]}
		check_sha "$h"
		residues=$(seq -s' ' "$offset" $((offset + weights[i] - 1)))
		offset=$((offset + weights[i]))
		echo "== $h: stride $stride offsets [$residues] → ${outs[$i]}"
		remote "$h" "systemctl --user stop 'poker-worker@*' 2>/dev/null || true; \
			cat >~/.config/pons/relabel-$RUN.env <<'EOF'
$(env_file "$i" "STRIDE=$stride" "OFFSETS=$residues")
EOF
			mkdir -p '${outs[$i]}' && systemctl --user restart pons-worker@$RUN"
	done
	echo "poker-worker units are stopped on every host and stay stopped; restart them by hand when the relabel is done."
	;;
status)
	for i in "${!hosts[@]}"; do
		h=${hosts[$i]}
		printf '%-12s ' "$h"
		remote "$h" "printf '%s %s  ' \$(systemctl --user is-active pons-worker@$RUN pons-worker@$RUN-mopup 2>/dev/null); \
			echo \"\$(ls '${outs[$i]}'/*/chunk-*.json 2>/dev/null | wc -l) chunks\""
	done
	;;
collect)
	for i in "${!hosts[@]}"; do
		[[ ${hosts[$i]} == localhost ]] || continue
		echo "== rsync ${outs[$i]}/ → ${hosts[0]}:${outs[0]}/"
		rsync -a "${outs[$i]}/" "${hosts[0]}:${outs[0]}/"
	done
	;;
mopup)
	LAYOUTS=${2:?mopup LAYOUTS}
	h=${hosts[0]}
	check_sha "$h"
	echo "== $h: mop-up (stride 1, reverse) → ${outs[0]}"
	remote "$h" "cat >~/.config/pons/relabel-$RUN-mopup.env <<'EOF'
$(env_file 0 "STRIDE=1" "OFFSETS=0" "REVERSE=1")
EOF
		systemctl --user restart pons-worker@$RUN-mopup"
	;;
*)
	sed -n '2,/^set -euo/{/^set -euo/d;s/^# \{0,1\}//;p}' "$0"
	exit 1
	;;
esac
