#!/usr/bin/env python3
"""Join probe-layer-replay provenance with probe-divergence IMPs: who lost it, book or floor?

    python3 scripts/divergence-layers.py $R/imps-none.jsonl $R/layers-none.jsonl        # the tables
    python3 scripts/divergence-layers.py $R/imps-none.jsonl $R/layers-none.jsonl veto   # the rail-design cut

Born on the §N1-lia lia3 forensic (2026-09-02); the tables it prints are the ones
quoted in scripts/ab-landy-lia3.sh's VERDICT block.  Board-level IMPs are from the
candidate's side; "vetoable" is the envelope-gated definition (floored suit bid on
<=4 cards with partner's announced minimum making <=5 combined, no bid-identity term).
"""
import json, sys, collections
BOARDS = 4_608_000
SEATS = ['North', 'East', 'South', 'West']
SUITS = '♠♥♦♣'
HCP = {'A': 4, 'K': 3, 'Q': 2, 'J': 1}


def load(p):
    return [json.loads(l) for l in open(p)]


def hand_of(rec, seat):
    hands = rec['deal'].split(':', 1)[1].split()
    return hands[SEATS.index(seat)]


def shape(hand):
    suits = hand.split('.')
    return [len(s) for s in suits], sum(HCP.get(c, 0) for c in hand)


def band(h):
    return '0-7' if h <= 7 else '8-9' if h <= 9 else '10-14' if h <= 14 else '15+'


def lane_idx(toks):
    for i in range(len(toks) - 1):
        if toks[i] == '1NT' and toks[i + 1] == '2♣':
            return i
    return None


def seat_at(dealer, i):
    return SEATS[(SEATS.index(dealer) + i) % 4]


def agg(rows, key):
    d = collections.defaultdict(lambda: [0, 0, 0])
    for r in rows:
        k = key(r)
        if k is None:
            continue
        c = d[k]
        c[0] += 1
        c[1] += r['imps_plain']
        c[2] += r['imps_pd']
    return d


def show(d, title, top=None, rev=False):
    print(f"\n--- {title}")
    rows = sorted(d.items(), key=lambda kv: (-kv[1][1] if rev else kv[1][1]))
    if top:
        rows = rows[:top]
    print(f"{'bucket':56} {'n':>7} {'plain':>8} {'/fired':>7} {'/board':>8} {'pd':>8} {'pd/f':>7}")
    for k, (n, pl, pd) in rows:
        print(f"{str(k)[:56]:56} {n:7d} {pl:8d} {pl/n:7.3f} {pl/BOARDS:8.4f} {pd:8d} {pd/n:7.3f}")


def main():
    imps = {r['index']: r for r in load(sys.argv[1])}
    layers = load(sys.argv[2])
    rows = []
    for L in layers:
        r = imps[L['index']]
        toks = r['auction_on'].split()
        i0 = lane_idx(toks)
        fd = r['first_diff']
        ours = L['calls']
        after = [c for c in ours if c['i'] >= fd]
        r['any_floored'] = any(c['floored'] for c in after)
        ff = next((c for c in after if c['floored']), None)
        r['first_floor'] = ff
        r['first_floor_key'] = None if ff is None or i0 is None else (
            ' '.join(toks[i0 + 2:ff['i']]) + f" [{ff['seat'][0]}:{ff['call']}]")
        r['first_floor_seat'] = None if ff is None else ff['seat']
        r['resp'] = next((c for c in ours if i0 is not None and c['i'] == i0 + 2), None)
        r['fd_call'] = next((c for c in ours if c['i'] == fd), None)
        r['toks'], r['i0'] = toks, i0
        r['_after'] = after
        rows.append(r)
    n = len(rows)
    pl = sum(r['imps_plain'] for r in rows)
    pd = sum(r['imps_pd'] for r in rows)
    print(f"{n} divergent joined; plain {pl} pd {pd}")

    show(agg(rows, lambda r: 'floor made >=1 of our calls after the divergence' if r['any_floored']
             else 'book made every call after the divergence'), "floor involvement after the divergence")
    show(agg(rows, lambda r: f"first floored call by {r['first_floor_seat']}" if r['first_floor'] else 'none'),
         "first floored seat")
    show(agg(rows, lambda r: ('divergent call itself is FLOORED' if r['fd_call']['floored']
                              else "divergent call is the BOOK's") if r['fd_call'] else 'divergent call is theirs'),
         "the divergent call's own layer")
    show(agg(rows, lambda r: r['first_floor_key']), "first floored node (auction after 2♣, [seat:call]) — worst 30", top=30)
    show(agg(rows, lambda r: r['first_floor_key']), "first floored node — best 10", top=10, rev=True)
    show(agg(rows, lambda r: r['first_floor_key'].split(' [')[0] if r['first_floor_key'] else None),
         "auction prefix before the first floored call — worst 25", top=25)
    show(agg(rows, lambda r: f"resp {r['resp']['call']} {'FLOOR' if r['resp']['floored'] else 'book'}" if r['resp'] else None),
         "responder's first call over 2♣, by layer")

    def resp_shape(r):
        if not r['resp'] or not r['resp']['floored']:
            return None
        sh, h = shape(hand_of(r, r['resp']['seat']))
        return f"resp FLOOR {r['resp']['call']} ♣{sh[3]} ♦{sh[2]} M{sh[0]}-{sh[1]} hcp{band(h)}"
    show(agg(rows, resp_shape), "floored responder first calls by shape — worst 25", top=25)
    show(agg(rows, resp_shape), "floored responder first calls by shape — most frequent 15", top=15, rev=False)

    # --- falsifiers ---
    def resp_hand(r):
        if r['i0'] is None:
            return None
        seat = seat_at(r['dealer'], r['i0'] + 2)
        return shape(hand_of(r, seat))

    def f2(r):
        hs = resp_hand(r)
        if hs is None:
            return None
        a = r['toks'][r['i0'] + 2:r['i0'] + 6]
        if len(a) >= 3 and a[0] == '2♥' and a[1] == '-' and a[2] == '3NT':
            return f"2♥ - 3NT: responder hcp {band(hs[1])}"
        return None
    show(agg(rows, f2), "falsifier 2: 2♥ - 3NT by responder's HCP")

    def f3(r):
        hs = resp_hand(r)
        if hs is None:
            return None
        if hs[0][2] == 5 and 8 <= hs[1] <= 9 and hs[0][3] <= 3:
            return f"resp ♦5 hcp8-9 (♣<=3) bid {r['toks'][r['i0'] + 2]}"
        return None
    show(agg(rows, f3), "falsifier 3: exactly-five 8-9 diamond hands, by responder's call")
    show(agg(rows, lambda r: f"{r['call_off']} -> {r['call_on']}" if {r['call_off'], r['call_on']} == {'X', '2♥'} else None),
         "falsifier 1: the X/2♥ swap cells")

    def f5(r):
        if r['i0'] is None:
            return None
        a = r['toks'][r['i0'] + 2:r['i0'] + 6]
        if len(a) >= 3 and a[0] == '2♥' and a[1] in ('2♠', '3♥', '3♠', '4♥', '4♠'):
            return f"2♥ ({a[1]}) {a[2]}"
        return None
    show(agg(rows, f5), "falsifier 5 + watch: 2♥ (raise) opener's call")

    worst = [k for k, _ in sorted(agg(rows, lambda r: r['first_floor_key']).items(), key=lambda kv: kv[1][1])[:8] if k]

    def suitlen(r):
        k = r['first_floor_key']
        if k not in worst:
            return None
        ff = r['first_floor']
        c = ff['call']
        if not c or c[-1] not in SUITS:
            return f"{k} (no suit)"
        sh, h = shape(hand_of(r, ff['seat']))
        return f"{k} {c[-1]}len={sh[SUITS.index(c[-1])]} hcp{band(h)}"
    show(agg(rows, suitlen), "worst floored nodes: the floored bidder's length in the suit it bid")

    # --- veto reach: would an envelope-gated new-suit veto have masked the first floored call? ---
    def veto_class(c):
        s = c.get('suit')
        if not s:
            return 'not a suit bid'
        comb = s['own_len'] + s['partner_min']
        tag = 'new suit' if s['new_suit'] else 'old suit'
        if comb <= 5:
            return f"{tag}, own+partner_min <= 5 (VETO)"
        if comb <= 6:
            return f"{tag}, own+partner_min == 6"
        return f"{tag}, own+partner_min >= 7"
    show(agg(rows, lambda r: veto_class(r['first_floor']) if r['first_floor'] else None),
         "veto reach on the FIRST floored call after the divergence")

    def vetoable(c):
        # The envelope-gated definition: no bid-identity term (an artificial 2♠ would
        # otherwise mark spades as "bid"); own length <= 4 and <= 5 announced combined.
        s = c.get('suit')
        return bool(c['floored'] and s and s['own_len'] <= 4 and s['own_len'] + s['partner_min'] <= 5)

    def any_veto(r):
        after = [c for c in r['_after'] if vetoable(c)]
        return 'some floored suit bid after the divergence is vetoable (own<=4, own+partner_min<=5)' if after else (
            'floor involved, nothing vetoable' if r['any_floored'] else 'book only')
    show(agg(rows, any_veto), "veto reach on ANY floored call after the divergence")

    def veto_node(r):
        for c in r['_after']:
            if vetoable(c):
                i0 = r['i0']
                return None if i0 is None else ' '.join(r['toks'][i0 + 2:c['i']]) + f" [{c['seat'][0]}:{c['call']} own{c['suit']['own_len']}+p{c['suit']['partner_min']}]"
        return None
    show(agg(rows, veto_node), "first vetoable floored call — worst 25 nodes", top=25)


if __name__ == '__main__':
    main()


def veto_design(imps_path, layers_path):
    """Cut the floored suit bids by own length, partner's announced min and level."""
    imps = {r['index']: r for r in load(imps_path)}
    rows = []
    for L in load(layers_path):
        r = imps[L['index']]
        fd = r['first_diff']
        for c in L['calls']:
            s = c.get('suit')
            if c['i'] >= fd and c['floored'] and s:
                rows.append({'imps_plain': r['imps_plain'], 'imps_pd': r['imps_pd'], 'c': c, 's': s,
                             'first': c['i'] == next(x['i'] for x in L['calls'] if x['i'] >= fd and x['floored'])})
    print(f"\n=== floored SUIT bids after the divergence: {len(rows)} calls (a board can contribute several; IMPs are the board's)")
    def key(r):
        s = r['s']; lvl = r['c']['call'][0]
        comb = s['own_len'] + s['partner_min']
        return f"own{min(s['own_len'],6)} pmin{min(s['partner_min'],4)} comb{'<=5' if comb <= 5 else '6' if comb == 6 else '>=7'} lvl{lvl if lvl in '234' else '5+'}"
    show(agg(rows, key), "floored suit bids by (own length, partner's announced min, combined, level) — worst 30", top=30)
    show(agg(rows, lambda r: f"own{min(r['s']['own_len'],6)} comb{'<=5' if r['s']['own_len']+r['s']['partner_min'] <= 5 else '>=6'}"), "floored suit bids by own length × combined<=5")
    show(agg(rows, lambda r: f"comb<=5 own<=4 lvl{r['c']['call'][0] if r['c']['call'][0] in '234' else '5+'}" if r['s']['own_len'] + r['s']['partner_min'] <= 5 and r['s']['own_len'] <= 4 else None), "candidate rule: own<=4 & own+partner_min<=5, by level")


if __name__ == '__main__' and len(sys.argv) > 3 and sys.argv[3] == 'veto':
    veto_design(sys.argv[1], sys.argv[2])
