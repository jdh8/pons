# pons-trainer

Off-crate trainer for the **AI instinct bidder** (milestone M1.1). It distills
the deterministic `american()` floor into a small MLP and exports the weights
for the crate to embed and run by hand (M1.2).

This is its **own cargo workspace** (note the empty `[workspace]` table in
`Cargo.toml`). `cargo build` / `cargo test` at the pons repo root never compile
it or candle — the crate stays dependency-light. Build and run it only from
inside this directory.

## Pipeline

1. **Generate the teacher data** (from the pons repo root):

   ```sh
   cargo run --release --example teacher-dump -- --boards 30000 --seed 1
   ```

   Writes `target/teacher-data.{f32,json,tags}` — flat LE-`f32` rows of
   `[160 features][38 teacher_softmax]`, a versioned JSON sidecar, and one `u8`
   per row (`1` = contested phase). **Don't commit the data**; regenerate it from
   the recorded seed — that is how reproducibility is preserved.

2. **Train** (from inside `trainer/`):

   ```sh
   cargo run --release
   ```

   Fits `160 -> H -> H -> 38` to the teacher softmax by soft-target
   cross-entropy, logging held-out top-1 agreement split into constructive vs
   contested rows. Win condition (M1.1): high on-book agreement (>95%), sane
   off-book.

3. **Artifact.** Writes into the crate at
   `src/bidding/weights/american_v1.{f32,json,fixture.json}`:
   - `.f32` — weights, layer order `l1.w, l1.b, l2.w, l2.b, l3.w, l3.b`, each
     `(out, in)` row-major (candle's `Linear` convention).
   - `.json` — feature version, hidden size, layer shapes, data seed, git SHA,
     held-out metrics. A model is meaningless without its exact feature
     extractor; they version together.
   - `.fixture.json` — a handful of `(features, logits)` rows; M1.2's in-crate
     forward pass must reproduce these within tolerance.

## Constants

`src/data.rs` mirrors `pons::bidding::features` (`FEATURES_VERSION = 1`,
`FEATURES_LEN = 160`) and `bidding::array` (`SOFTMAX_LEN = 38`) and asserts them
against the data sidecar. If the crate's feature spec changes, bump them
together — a version mismatch fails the load loudly.

## Useful flags

`--data`, `--weights-out`, `--hidden`, `--epochs`, `--lr`, `--batch`,
`--val-frac`, `--fixture`. See `cargo run -- --help`.
