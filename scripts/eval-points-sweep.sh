#!/usr/bin/env bash
#
# eval-points-sweep.sh — the strength-reading ablation, five arms over one corpus.
#
# The question: v4 replaced each hidden seat's *length* endpoints with a shape
# Gaussian and measured par (+0.00004 NLL).  Points are the axis where that
# argument should break — a length reading is narrow and its prior nearly flat,
# a strength reading is wide (`0..37` unshown, `11..26` after a 2/1) and its
# prior sharply peaked, so `{min, max}` discards far more.  And `strength.hcp`,
# the crisp raw band written unslacked wherever `points` is slacked, is an axis
# no shipped vector has ever read: it is non-⊤ at 23-29% of nodes and binds
# beyond what `points` implies at 12-16% of them.
#
# Every arm is a `--arm` mask over the *same* `--encoding points` corpus, so all
# five see identical rows in identical batch order at identical parameter count.
# Judge Δ NLL against the **0.0006** seed spread measured on the MLP-256 rung of
# the architecture ladder (docs/ai-bidder/evaluator-net.md); every arm here is a
# single `--seed 1` run, so anything under that is par and must be reported as
# par.  Absolute NLLs are comparable only *within* this table.
#
# Corpus (400k deals, 8,146,934 rows — the same nodes the shape sweep saw):
#   ./target/release/examples/dump-evaluator --deals /nfs2/jdh8/pons/22.pdd \
#       --count 400000 --seed 1 --systems american,dutch --encoding points --dnf \
#       --out target/eval-points-corpus
#
# Usage:  scripts/eval-points-sweep.sh [out-dir]
#   setsid nohup scripts/idle-run.sh scripts/eval-points-sweep.sh \
#       target/eval-points >target/eval-points.log 2>&1 < /dev/null &
set -euo pipefail

cd "$(dirname "$0")/.."
OUT=${1:-target/eval-points}
DATA=${DATA:-../target/eval-points-corpus}
# Overridable so a follow-up arm, or a seed replicate of a winner, reuses this
# script rather than growing a second one.
ARMS=${ARMS:-"pts-control pts-gauss pts-gauss-mass pts-both pts-hcp-ends"}
SEED=${SEED:-1}
mkdir -p "$OUT"

export PATH=/usr/local/cuda-12.8/bin:$PATH
# One GPU, one arm at a time: the whole corpus is resident on device, and two
# arms sharing a card measure each other's contention rather than the encoding.
export CUDA_VISIBLE_DEVICES=${CUDA_VISIBLE_DEVICES:-0}

# pts-control is the number every other arm is judged against: it reproduces
# the shipped `features_eval_v4` vector out of this corpus.
for arm in $ARMS; do
	stem="$OUT/$arm"
	# Resumable: an arm that already exported its sidecar is left alone.
	if [ -s "$stem.json" ]; then
		echo "eval-points-sweep: $arm already done, skipping"
		continue
	fi
	echo "eval-points-sweep: $arm …"
	(cd trainer && cargo run --release --features cuda --bin evaluator -- \
		--data "$DATA" --arm "$arm" \
		--hidden 256 --epochs 150 --seed "$SEED" \
		--weights-out "../$stem")
done

echo
python3 - "$OUT" "$ARMS" <<-'PY'
	import json, sys, os
	arms = sys.argv[2].split()
	rows = []
	for arm in arms:
	    path = os.path.join(sys.argv[1], arm + ".json")
	    if os.path.exists(path):
	        rows.append((arm, json.load(open(path))))
	base = dict(rows).get("pts-control", {}).get("val_nll")
	print(f'{"arm":<16}{"val NLL":>11}{"Δ vs control":>14}{"MAE":>9}{"slam MAE":>10}')
	for arm, m in rows:
	    d = "" if base is None else f'{m["val_nll"] - base:+.5f}'
	    print(f'{arm:<16}{m["val_nll"]:>11.5f}{d:>14}'
	          f'{m["val_mae_tricks"]:>9.4f}{m["val_slam_mae_tricks"]:>10.4f}')
	print("\nGate: the MLP-256 seed spread is 0.0006. A more-negative NLL is better;\n"
	      "anything inside +/-0.0006 of pts-control is par and reports as par.")
PY
