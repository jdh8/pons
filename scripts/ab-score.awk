# ab-score.awk — pool per-shard A/B summaries into one line per bracket, with a CI.
#
# ab-aggregate.sh only sums, and it matches /^Delta/ — which double-counts the
# two brackets ab-dump-sd now emits. This splits by bracket tag and adds the
# 95% CI measurement.md prescribes: per-shard IMPs/board treated as N samples.
#
#   awk -v name=<label> -f scripts/ab-score.awk <file>

/ fired \(/ {
    for (i = 1; i <= NF; i++) {
        if ($i == "boards,") { v = $(i-1); gsub(/\(/, "", v); boards += v }
        if ($i == "fired")   { fired += $(i-1) }
    }
}

/^Delta/ {
    # Bracket tag: ab-dump-sd emits "Delta plain" and "Delta perfect-defense";
    # ab-dump-diff emits a bare "Delta (run - sit)".
    if ($2 == "plain")           tag = "SD"
    else if ($2 == "perfect-defense") tag = "SD-PD"
    else                          tag = "-"

    for (i = 2; i <= NF; i++) {
        if ($i == "IMPs,")      { v = $(i-1); gsub(/[+,]/, "", v); imps[tag] += v }
        if ($i == "IMPs/board") { m = $(i-1); gsub(/\+/, "", m);
                                  n[tag]++; s[tag] += m; ss[tag] += m * m }
        # Pool the per-shard board-level CIs the tools print: equal-n shards, so
        # SE_pooled = sqrt(sum SE_i^2)/k. This is the method the published
        # ledger figures use; keep it alongside the shard-mean CI for comparability.
        if ($i == "CI") { c = $(i+1); gsub(/[^0-9.]/, "", c)
                          se = c / 1.959964; sesq[tag] += se * se }
    }
}

END {
    for (t in n) {
        k = n[t]
        mean = s[t] / k
        # sample variance of the per-shard means
        var = (k > 1) ? (ss[t] - k * mean * mean) / (k - 1) : 0
        if (var < 0) var = 0
        ci = 1.959964 * sqrt(var / k)
        cib = 1.959964 * sqrt(sesq[t]) / k
        label = (t == "-") ? name : name " [" t "]"
        printf "%-44s %8d bd, %6d fired (%.3f%%), %+7d IMPs, %+.4f IMPs/board  CI(shard) +/-%.4f  CI(board) +/-%.4f, %+.3f /fired (%d shards)\n",
            label, boards, fired, 100.0 * fired / (boards ? boards : 1),
            imps[t], mean, ci, cib, imps[t] / (fired ? fired : 1), k
    }
}
