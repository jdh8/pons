#!/usr/bin/env python3
# ponytail: throwaway analysis for docs/ai-bidder/bba-floor.md — delete after report lands.
# Characterizes BBA's MB.TXT rule DB: how much of it is parametric/generic (the
# "floor") vs concrete-auction-specific. Line format (space-separated fields):
#   leader  pattern~constraint~term  weight  @alert  #index
# The auction pattern and its trailing ~constraint~ are glued in field 1.
import sys, re, collections

path = sys.argv[1] if len(sys.argv) > 1 else "vendor/bba/MB.TXT"
lines = [l.rstrip("\n") for l in open(path) if l.strip()]
total = len(lines)

depth_all = collections.Counter()
depth_by_class = {"specific": collections.Counter(), "generic": collections.Counter()}
axes = collections.Counter()       # how many lines hit each generality axis
op_counts = collections.Counter()  # constraint operator/function inventory
weights = []
with_constraint = 0
empty_pattern = 0
specific = generic = 0

# A pattern is "generic" if it generalizes across auctions/suits rather than
# matching one concrete call sequence.
CLASS_CHARS = re.compile(r"[\[\]]")        # [1-7] level / [^N] suit ranges
KLEENE = re.compile(r"[*+]")               # repetition: variable-length auctions
SUITVAR = re.compile(r"[a-h]")             # lowercase suit *variables* (templated)
LITERAL_ONLY = re.compile(r"^[1-7CDHSNPXR:.#0-9\(\),]*$")  # rough literal check

for l in lines:
    f = l.split()
    if len(f) < 2:
        continue
    field1 = f[1]
    # split pattern from its glued ~constraint~
    pat = field1.split("~", 1)[0]
    constraint = ""
    if "~" in field1:
        constraint = "~".join(field1.split("~")[1:])
        with_constraint += 1

    seg = pat.count(":") + 1 if pat else 0
    depth_all[seg] += 1

    is_generic = False
    if pat == "":
        empty_pattern += 1; axes["empty-pattern (matches any auction)"] += 1; is_generic = True
    if "START" in pat:
        axes["START catch-all"] += 1; is_generic = True
    if "TERMINATE" in pat:
        axes["TERMINATE catch-all"] += 1; is_generic = True
    if CLASS_CHARS.search(pat):
        axes["char-class [..] ranges"] += 1; is_generic = True
    if KLEENE.search(pat):
        axes["Kleene */+ (variable-length)"] += 1; is_generic = True
    if SUITVAR.search(pat):
        axes["suit variable a-h (templated)"] += 1; is_generic = True

    cls = "generic" if is_generic else "specific"
    depth_by_class[cls][seg] += 1
    if is_generic: generic += 1
    else: specific += 1

    # weight = first all-digit field after field1
    for tok in f[2:]:
        if tok.isdigit():
            weights.append(int(tok)); break

    # constraint operator inventory
    for op in ["&&", "||", "==", "<=", ">=", "<", ">", "$$", "$"]:
        op_counts[op] += constraint.count(op)
    op_counts["#N(..) hand-feature fn"] += len(re.findall(r"#\d+\(", constraint))
    op_counts[":action codes (m/M/D/G/F)"] += len(re.findall(r":[mMDGF]", l))

print(f"total rule lines: {total}\n")

print("== genericity (a line is generic if it hits ANY axis below) ==")
print(f"  generic : {generic:5d}  ({100*generic/total:.1f}%)")
print(f"  specific: {specific:5d}  ({100*specific/total:.1f}%)")
print(f"  sanity  : {generic+specific} == {total}\n")

print("== generality axes (non-exclusive; a line can hit several) ==")
for k, v in axes.most_common():
    print(f"  {k:36s}: {v:5d}  ({100*v/total:.1f}%)")
print()

print("== auction depth (colon-segments in pattern) ==")
for d in sorted(depth_all):
    g = depth_by_class['generic'][d]; s = depth_by_class['specific'][d]
    print(f"  depth {d:2d}: {depth_all[d]:5d}  (specific {s:5d} / generic {g:5d})")
print(f"  max depth: {max(depth_all)}\n")

print("== constraint usage ==")
print(f"  lines with a ~constraint~ : {with_constraint} ({100*with_constraint/total:.1f}%)")
for k, v in op_counts.most_common():
    print(f"  {k:30s}: {v}")
print()

print("== weight distribution (deterministic conflict resolver, 0-99) ==")
ws = sorted(weights)
import statistics
buckets = collections.Counter()
for w in ws:
    buckets[(w//10)*10] += 1
for b in sorted(buckets):
    print(f"  {b:2d}-{b+9:2d}: {buckets[b]:5d}")
print(f"  n={len(ws)} min={ws[0]} median={statistics.median(ws)} "
      f"mean={statistics.mean(ws):.1f} max={ws[-1]}")
print(f"  weight==99 (hard/forcing): {buckets[90] and sum(1 for w in ws if w==99)} "
      f"  weight<=10 (soft floor): {sum(1 for w in ws if w<=10)}")
