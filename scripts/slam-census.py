# slam-census.py — census the anchor's "Constructive / book / round-2" bucket by
# slam direction and lane (docs/next-steps.md item 2, 2026-09-30).
#   python3 scripts/slam-census.py ab-results/anchor/<snapshot>
import json,glob,sys,collections
S=sys.argv[1] if len(sys.argv)>1 else "ab-results/anchor/2026-09-26-7e0bc648"
BUCKET=("Constructive","book","round-2")
# join key: (vul, seed, board)
shards={}
for v in ["none","both"]:
    for f in glob.glob(f"{S}/{v}/shard-*.json"):
        d=json.load(open(f)); shards[(v,d["seed"])]=d["boards"]
def calls(a): return a.split()
def final(a):
    c=[x for x in calls(a) if x not in ("-","X","XX")]
    return c[-1] if c else None
def level(a):
    f=final(a); return int(f[0]) if f else 0
def slammy(a,b):
    return level(a)>=6 or level(b)>=6 or "4NT" in calls(a) or "4NT" in calls(b)
rows=[json.loads(l) for l in open(f"{S}/boards.jsonl")]
rows=[r for r in rows if (r["phase"],r["provenance"],r["family"])==BUCKET]
tot=collections.Counter(); lane=collections.defaultdict(lambda:[0,0,0])
dirs=collections.defaultdict(lambda:[0,0,0]); worst=[]
nslam=0
for r in rows:
    b=shards[(r["vul"],r["seed"])][r["board"]]
    A,B=b["table_a"],b["table_b"]
    ours = A if r["our_call"]==calls(A)[r["div_index"]] and r["our_call"]!=calls(B)[r["div_index"]] else B  # ponytail: only used for lane label
    # our seat at divergence: NS at A iff seat index parity; simpler: use the table whose div call == our_call
    tbl = A if calls(A)[r["div_index"]]==r["our_call"] else B
    pre=calls(tbl)[:r["div_index"]]
    # lane = first two non-pass calls
    nz=[c for c in pre if c!="-"][:2]
    L=" - ".join(nz)
    d=r["direction"]
    if slammy(A,B): d2=d if d.startswith("missed") or d=="overbid" else "other"; nslam+=1
    else: d2="game-level"
    key=(d2 if d2!="overbid" else ("overbid-slam" if max(level(A),level(B))>=6 else "overbid-game"))
    for m,k in ((tot,"all"),(dirs,key),(lane,(L,key))):
        m[k] if m is tot else None
        if m is tot: tot["n"]+=1; tot["plain"]+=r["swing_plain"]; tot["pd"]+=r["swing_pd"]
        else: m[k][0]+=1; m[k][1]+=r["swing_plain"]; m[k][2]+=r["swing_pd"]
    if key not in ("game-level",):
        worst.append((r["swing_pd"],r["vul"],A,B,b["deal"],key,r["hand"],r["our_call"],r["bba_call"]))
print("bucket rows",tot["n"],"plain",tot["plain"],"pd",tot["pd"],"slam-flavoured rows",nslam)
print("\n== by direction (n, plain, pd)")
for k,v in sorted(dirs.items(),key=lambda kv:kv[1][2]): print(f"{k:14} {v[0]:6} {v[1]:7} {v[2]:7}")
print("\n== by lane x direction, slam-flavoured only, sorted by PD")
for k,v in sorted(lane.items(),key=lambda kv:kv[1][2])[:40]:
    if k[1]=="game-level": continue
    print(f"{k[0]:12} {k[1]:14} {v[0]:5} {v[1]:6} {v[2]:6}")
print("\n== worst 25 slam-flavoured boards (pd, vul, key)")
for w in sorted(worst)[:25]:
    print(w[0],w[1],w[5],"| hand",w[6],"we",w[7],"bba",w[8]); print("   A:",w[2]); print("   B:",w[3]); print("   ",w[4])
