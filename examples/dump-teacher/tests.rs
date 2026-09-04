use super::*;
use contract_bridge::{Seat, Suit};
use std::path::PathBuf;

#[test]
fn enrich_thresholds_parse_as_hcp_then_fit() {
    assert_eq!(parse_enrich("28:9"), Ok((28, 9)));
    assert!(parse_enrich("28").is_err(), "no separator");
    assert!(parse_enrich("28:x").is_err(), "fit is not a number");
}

/// The raw-hand test must ignore spades: a spade ask is 4NT under either
/// card, so a spade fit carries no configuration signal and accepting on
/// one would spend the whole enriched slice on deals that cannot diverge.
#[test]
fn the_raw_hand_test_ignores_spades() {
    // North holds all thirteen spades and all his side's points; the best
    // *non*-spade fit at the table is E-W's nine (diamonds, and clubs).
    let deal: FullDeal = "N:AKQJT98765432... .98765.T987.T987 \
                          .AKQJT.AKQJ.AKQJ .432.65432.65432"
        .parse()
        .expect("a PBN deal, North first");
    assert_eq!(deal[Seat::North][Suit::Spades].len(), 13, "a 13-card fit");
    assert_eq!(
        slam_ish(&deal),
        (40, 9),
        "all forty points to N-S, and the fit is E-W's nine — not the spade thirteen",
    );
}

// ── The relabel: partition invariance, extension, and the sidecar contract ──
//
// Random boards under the Rust teacher, so no bank and no EPBot are needed;
// one cell keeps the partnership builds cheap.  `--skip` offsets random
// boards under `--relabel`, which is what makes a window splittable.

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pons-relabel-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn dump(out: &Path, skip: u64, boards: usize, layouts: usize, seed: u64) {
    std::fs::create_dir_all(out.parent().expect("a shard dir")).expect("mkdir");
    let args = Args::parse_from([
        "dump-teacher",
        "--relabel",
        "--configured",
        "--teacher",
        "american",
        "--feature-version",
        "6",
        "--cell",
        "a-off/a-off",
        "--skip",
        &skip.to_string(),
        "--boards",
        &boards.to_string(),
        "--layouts",
        &layouts.to_string(),
        "--seed",
        &seed.to_string(),
        "--out",
        out.to_str().expect("utf-8 path"),
    ]);
    run(args).expect("dump succeeds");
}

fn cut(roots: &[&Path], out: &Path, m: usize) -> anyhow::Result<()> {
    let mut argv = vec!["dump-teacher".to_string(), "--cut".into(), m.to_string()];
    for root in roots {
        argv.push("--chunks".into());
        argv.push(root.to_str().expect("utf-8 path").into());
    }
    argv.push("--out".into());
    argv.push(out.to_str().expect("utf-8 path").into());
    run(Args::parse_from(argv))
}

fn bytes(path: PathBuf) -> Vec<u8> {
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn fired(json: PathBuf) -> u64 {
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json).expect("sidecar")).expect("json");
    meta["relabel"]["fired"].as_u64().expect("fired")
}

/// Two windows written separately, then cut, equal the one window cut whole —
/// byte for byte on the rows and the tags.
#[test]
fn split_then_cut_equals_whole_then_cut() {
    let dir = scratch("split");
    dump(&dir.join("whole/s/chunk-0"), 0, 12, 4, 7);
    dump(&dir.join("split/s/chunk-0"), 0, 6, 4, 7);
    dump(&dir.join("split/s/chunk-1"), 6, 6, 4, 7);
    cut(&[&dir.join("whole")], &dir.join("cut-whole"), 2).expect("cut whole");
    cut(&[&dir.join("split")], &dir.join("cut-split"), 2).expect("cut split");
    assert_eq!(
        bytes(dir.join("cut-whole/s.f32")),
        bytes(dir.join("cut-split/s.f32"))
    );
    assert_eq!(
        bytes(dir.join("cut-whole/s.tags")),
        bytes(dir.join("cut-split/s.tags"))
    );
    assert!(
        fired(dir.join("cut-whole/s.json")) > 0,
        "the gate never fired, so the identity is vacuous"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// A chunk stored at 4 layouts and extended to 8 cuts identically to a native
/// 8-layout draw at `M = 4`, and identically to its own pre-extension cut at
/// `M = 2` — no solve is repeated and no label moves.
#[test]
fn an_extended_draw_cuts_like_a_native_one() {
    let dir = scratch("extend");
    dump(&dir.join("ext/s/chunk-0"), 0, 8, 4, 11);
    cut(&[&dir.join("ext")], &dir.join("cut-m2-before"), 2).expect("cut at 4 layouts");
    dump(&dir.join("ext/s/chunk-0"), 0, 8, 8, 11); // extends the stored 4 to 8
    dump(&dir.join("native/s/chunk-0"), 0, 8, 8, 11);
    assert_eq!(
        bytes(dir.join("ext/s/chunk-0.ret")),
        bytes(dir.join("native/s/chunk-0.ret")),
        "the extended returns are the native 8-layout returns"
    );
    cut(&[&dir.join("ext")], &dir.join("cut-m2-after"), 2).expect("cut after extension");
    assert_eq!(
        bytes(dir.join("cut-m2-before/s.f32")),
        bytes(dir.join("cut-m2-after/s.f32")),
        "extending must not move a label cut at the old M"
    );
    cut(&[&dir.join("ext")], &dir.join("cut-m4-ext"), 4).expect("cut extended at 4");
    cut(&[&dir.join("native")], &dir.join("cut-m4-native"), 4).expect("cut native at 4");
    assert_eq!(
        bytes(dir.join("cut-m4-ext/s.f32")),
        bytes(dir.join("cut-m4-native/s.f32"))
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The cut refuses what the retired `merge.sh` refused: a chunk from another
/// commit, a chunk short of `2M` layouts, and a non-contiguous tiling.
#[test]
fn the_cut_refuses_a_foreign_or_short_chunk() {
    let dir = scratch("refuse");
    dump(&dir.join("s/chunk-0"), 0, 4, 4, 3);
    dump(&dir.join("s/chunk-1"), 4, 4, 4, 3);
    let err = cut(&[&dir], &dir.join("cut-m4"), 4).expect_err("4 layouts cannot cut at M = 4");
    assert!(err.to_string().contains("--cut 4 needs 8"), "{err}");

    let json = dir.join("s/chunk-1.json");
    let meta = std::fs::read_to_string(&json).expect("sidecar");
    let mut foreign: serde_json::Value = serde_json::from_str(&meta).expect("json");
    foreign["git_sha"] = "0000000000000000000000000000000000000000".into();
    std::fs::write(&json, format!("{foreign:#}\n")).expect("write");
    let err = cut(&[&dir], &dir.join("cut-foreign"), 2).expect_err("another commit");
    assert!(err.to_string().contains("disagrees"), "{err}");

    let mut gap: serde_json::Value = serde_json::from_str(&meta).expect("json");
    gap["skip"] = 5.into();
    std::fs::write(&json, format!("{gap:#}\n")).expect("write");
    let err = cut(&[&dir], &dir.join("cut-gap"), 2).expect_err("a gap in the tiling");
    assert!(err.to_string().contains("not contiguous"), "{err}");
    let _ = std::fs::remove_dir_all(dir);
}
