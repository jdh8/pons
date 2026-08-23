#!/usr/bin/env bash
# Reproduce the hand-picked probes behind docs/ai-bidder/bba-book.md §5.7/§6.1.
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build --quiet --example probe-classify --example render-book
probe=target/debug/examples/probe-classify
render=target/debug/examples/render-book

section() {
    printf '\n%s\n' "$1"
}

classify() {
    local expected=$1 hand=$2 auction=$3 output actual
    output=$("$probe" --hand "$hand" --auction "$auction" --vulnerability both |
        awk '/^provenance:/{print; getline; print; found=1} END{if (!found) exit 1}')
    actual=$(awk 'NR==2 {print $1}' <<<"$output")
    printf '  %-28s  %-4s  %s\n' "$hand" "$actual" "${output%%$'\n'*}"
    if [[ $actual != "$expected" ]]; then
        printf 'expected %s, got %s for %s at %s\n' "$expected" "$actual" "$hand" "$auction" >&2
        return 1
    fi
}

probes() {
    local expected hand auction
    while IFS='|' read -r expected hand auction; do
        [[ -z $expected ]] || classify "$expected" "$hand" "$auction"
    done
}

exact_node() {
    local node=$1
    "$render" --prefix "$node" 2>/dev/null |
        awk -v node="$node" '
            !done && $0 == node { show=1; found=1 }
            show && /^$/ { show=0; done=1; next }
            show { print }
            END { if (!found) exit 1 }
        '
}

section '1. 15–17 balanced with a six-card minor: all 1NT'
probes <<'EOF'
1NT|AQ3.K2.K4.QJ8765|
1NT|KQ.KQ3.AJ7654.32|
1NT|A2.KQ3.KQ7654.Q2|
1NT|AQ2.KJ3.K2.KJ7654|
EOF

section '2. 25–27 balanced with four stoppers: all 2♣, not 3NT'
probes <<'EOF'
2♣|AQT.KQJ.AKQ3.KJ3|
2♣|AKQJ.AKQ.KJ3.QJ3|
2♣|AKQ3.AKJ3.AQ3.K2|
2♣|AKJ2.AQ2.KQJ.AK3|
EOF

section '3. 1M - 3M: the authored invitation is live'
exact_node '1♠ -'
probes <<'EOF'
3♠|KJ84.QJ3.8732.K2|1S -
3♠|QJ84.KQ3.8732.K2|1S -
2NT|KQ84.AJ3.Q732.42|1S -
2♠|KJ84.Q53.8732.J2|1S -
4♠|Q9842.853.873.J2|1S -
EOF

section '4. 1M (X): three-card support raises; 1NT is natural/no-fit'
exact_node '1♥ X'
probes <<'EOF'
2♥|J83.Q84.K732.J42|1H X
2♥|Q83.K84.Q732.J42|1H X
2♥|Q83.K84.A732.842|1H X
1NT|Q83.Q4.K732.J842|1H X
EOF

section '5. Competitive doubles: direct floor and negative-double shape'
exact_node '1♦'
exact_node '1♠ 2♥'
printf '\n  direct takeout double (pons starts at 12):\n'
probes <<'EOF'
P|KJ84.QJ84.32.K42|1D
P|KJ84.KJ84.32.K42|1D
X|KQ84.KJ84.32.K42|1D
X|KQ84.KQ84.32.K42|1D
EOF
printf '\n  four-heart overlap after 1♠ (2♥) (pons doubles from 8):\n'
probes <<'EOF'
X|Q2.QJ84.K732.842|1S 2H
X|Q2.KJ84.Q732.J42|1S 2H
X|K2.KJ84.Q732.J42|1S 2H
X|K2.KQ84.Q732.J42|1S 2H
EOF
printf '\n  BBA-shaped 2=2=4=5 controls (pons passes 8–12):\n'
probes <<'EOF'
P|Q2.J4.KJ84.J9842|1S 2H
P|Q2.Q4.KJ84.J9842|1S 2H
P|Q2.J4.KJ84.QJ842|1S 2H
P|K2.J4.KJ84.QJ842|1S 2H
P|K2.Q4.KJ84.QJ842|1S 2H
EOF

section '6. 1♠ - 2♣ is forcing for pons: opener never passes'
exact_node '1♠ - 2♣ -'
probes <<'EOF'
2♠|AQJ843.K32.Q2.42|1S - 2C -
2♦|KQJ84.K2.QJ32.42|1S - 2C -
2♥|KQJ84.AJ32.42.42|1S - 2C -
3♣|KQJ84.K2.32.QJ42|1S - 2C -
EOF

section 'id 25. 1NT opening shape 5 major: truthful/on'
probes <<'EOF'
1NT|AQJ84.K32.Q32.K2|
1NT|KQ3.AQ984.K32.Q2|
1NT|AKJ84.KJ2.K32.Q2|
EOF

section 'id 116. No NMF after a 2NT rebid; a forced 3♣ gets generic 3NT'
exact_node '1♦ - 1♠ - 2NT -'
exact_node '1♣ - 1♥ - 2NT -'
probes <<'EOF'
3NT|QJ984.K32.842.42|1D - 1S - 2NT -
3NT|KJ984.QJ84.32.42|1D - 1S - 2NT -
3NT|KQ9842.J32.84.42|1D - 1S - 2NT -
4NT|KQ984.KJ2.Q42.J2|1D - 1S - 2NT -
3NT|Q32.KJ984.842.Q2|1C - 1H - 2NT -
3NT|QJ84.KJ984.32.42|1C - 1H - 2NT -
3NT|Q32.KQ9842.84.42|1C - 1H - 2NT -
4NT|KJ2.KQ984.Q42.J2|1C - 1H - 2NT -
3NT|AQ3.K2.AQJ84.K32|1D - 1S - 2NT - 3C -
3NT|KJ.AQ84.KQJ4.K32|1D - 1S - 2NT - 3C -
3NT|KJ.KQ3.AQJ84.K32|1D - 1S - 2NT - 3C -
3NT|QJ.AQ3.K32.AQJ84|1C - 1H - 2NT - 3C -
3NT|AQJ4.K2.K32.KQJ4|1C - 1H - 2NT - 3C -
3NT|AQ3.K2.K32.AQJ84|1C - 1H - 2NT - 3C -
EOF
