//! binky — an additive hand valuation that publishes a *spread*, not just a centre.
//!
//! Thomas Andrews' Binky Points fit one number per hand to double-dummy truth,
//! additive across the partnership. So does every other published count — Work,
//! Fifths, BUM-RAP, Zar. None of them reports how *uncertain* that centre is,
//! and uncertainty is what a bidding decision actually consumes: a game gate
//! fires on `P(T ≥ 9) ≥ break-even`, which needs a distribution.
//!
//! This fits **two** additive numbers per suit holding — a contribution to the
//! mean (tricks) and a contribution to the variance (trick²) — so a partnership
//! sums both, takes a square root, and reads `P(make) = Φ((μ − target + ½)/σ)`
//! in closed form.
//!
//! `probe-trick-variance`'s Cut C is the gate this passed to exist: at matched
//! partnership HCP, the conditional σ of notrump tricks moves 0.5–0.9 tricks
//! across shape classes while the mean barely moves at all.
//!
//! **Variance adds; log-variance does not.** Given both N-S hands the only
//! randomness left is the E-W split of 26 cards, and it enters as near-independent
//! per-suit events — a two-way finesse contributes ≈ +0.25 trick², AKQJT opposite
//! xxx contributes 0. Independent sources add *variances*, so the additive column
//! is σ² and `ln σ` never appears: it is a trainer's parameterization trick (free
//! positivity under gradient descent), and nothing here does gradient descent.
//!
//!   cargo run --release --example binky -- \
//!       --deals /nfs2/jdh8/pons/24.pdd --skip 10000000 --count 20000000 \
//!       --test-skip 40000000 --test-count 2000000
use std::collections::BTreeMap;
use std::io::Write as _;

use clap::{Parser, ValueEnum};
use contract_bridge::deck::fill_deals;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{Builder, Hand, Holding, Rank, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver, StrainFlags, TrickCountTable};
use nalgebra::{DMatrix, DVector};
use pons::bidding::evaluator::Gaussian;
use rand::SeedableRng as _;
use rand::rngs::StdRng;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "/nfs2/jdh8/pons/24.pdd")]
    deals: String,
    /// Which contract the valuation is for. Andrews publishes these separately
    /// and so do we: they are different physics, and only one of them is
    /// honestly additive — see the benchmark.
    #[arg(long, value_enum, default_value_t = Label::Notrump)]
    label: Label,
    /// First deal of the fitting slice.
    #[arg(long, default_value_t = 10_000_000)]
    skip: u64,
    #[arg(long, default_value_t = 20_000_000)]
    count: usize,
    /// First deal of the held-out slice — keep it disjoint from the fit.
    #[arg(long, default_value_t = 40_000_000)]
    test_skip: u64,
    #[arg(long, default_value_t = 2_000_000)]
    test_count: usize,
    /// N-S pairs whose reshuffled truth fits the physical variance column (0 = skip).
    #[arg(long, default_value_t = 0)]
    variance_truth: usize,
    /// E-W shuffles per pair when fitting the physical column. Two is unbiased.
    #[arg(long, default_value_t = 2)]
    variance_shuffles: usize,
    /// First deal of the physical-variance slice — disjoint from everything else.
    #[arg(long, default_value_t = 55_000_000)]
    truth_skip: u64,
    /// N-S pairs to price against the *true* conditional moments (0 = skip).
    #[arg(long, default_value_t = 0)]
    benchmark: usize,
    /// E-W shuffles solved per benchmarked pair.
    #[arg(long, default_value_t = 200)]
    shuffles: usize,
    /// First deal of the benchmark slice — disjoint from fit and held-out.
    #[arg(long, default_value_t = 50_000_000)]
    bench_skip: u64,
    #[arg(long, default_value_t = 20_260_726)]
    seed: u64,
    /// Where to write the machine-readable table.
    #[arg(long)]
    json: Option<String>,
    /// Where to write the human-readable table.
    #[arg(long)]
    markdown: Option<String>,
}

/// The contract a valuation is for.
///
/// Exchangeability holds for both — permuting the four suits leaves notrump
/// tricks alone and permutes the four suit rows a max runs over — so one shared
/// holding table serves either. **Additivity is the assumption that differs**,
/// and the benchmark is what tests it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum Label {
    /// N-S double-dummy notrump tricks.
    Notrump,
    /// N-S double-dummy tricks in their best *suit* — notrump excluded, so the
    /// two tables answer disjoint questions rather than one shadowing the other.
    BestSuit,
}

impl Label {
    /// The one place the label is computed. The fit and the benchmark both call
    /// it, so the two can never drift apart on what they are measuring.
    fn tricks(self, table: &TrickCountTable) -> u8 {
        let ns = |strain: Strain| {
            let row = table[strain];
            row.get(Seat::North).get().max(row.get(Seat::South).get())
        };
        match self {
            Self::Notrump => ns(Strain::Notrump),
            // `Strain::ASC` runs ♣♦♥♠NT, so the first four are the suits.
            Self::BestSuit => Strain::ASC[..4].iter().map(|&s| ns(s)).max().unwrap_or(0),
        }
    }

    /// Only solve the strains the label reads — notrump alone is ~5× less work.
    fn flags(self) -> NonEmptyStrainFlags {
        let flags = match self {
            Self::Notrump => StrainFlags::NOTRUMP,
            Self::BestSuit => StrainFlags::all().difference(StrainFlags::NOTRUMP),
        };
        NonEmptyStrainFlags::new(flags).expect("both variants name at least one strain")
    }

    fn name(self) -> &'static str {
        match self {
            Self::Notrump => "N-S double-dummy notrump tricks",
            Self::BestSuit => "N-S double-dummy tricks in their best suit",
        }
    }
}

// ---------------------------------------------------------------- the basis

/// The honours a holding is keyed on, high to low.
const HONOURS: [Rank; 5] = [Rank::A, Rank::K, Rank::Q, Rank::J, Rank::T];
/// Subsets of [`HONOURS`].
const MASKS: usize = 1 << 5;
/// Spot cards are ranks 2..9, so a holding has 0..=8 of them.
const SPOTS: usize = 9;
/// One shared table of holdings, not four: see [`cell`].
const CELLS: usize = MASKS * SPOTS;
/// Four suits × two N-S hands.
const HOLDINGS: usize = 8;
/// N-S pairs whose layouts go into one `solve_deals` batch.
const PAIRS_PER_BATCH: usize = 256;

/// Index a suit holding by (honours held, spot count).
///
/// **One table serves all four suits.** Both labels this example can fit —
/// notrump tricks, and best-strain tricks — are invariant under permuting the
/// four suits, so holdings are exchangeable and four per-suit tables would be
/// four noisy copies of one.
fn cell(holding: Holding) -> usize {
    let held = HONOURS.map(|rank| holding.contains(rank));
    let mask = held.iter().enumerate().fold(0, |acc, (i, &h)| {
        acc | (usize::from(h) << (HONOURS.len() - 1 - i))
    });
    let spots = holding.len() - held.iter().filter(|&&h| h).count();
    mask * SPOTS + spots
}

/// How many cards the holdings in a cell hold — the second gauge direction.
fn cell_size(cell: usize) -> f64 {
    let spots = cell % SPOTS;
    let honours = (cell / SPOTS).count_ones() as usize;
    (spots + honours) as f64
}

/// Render a cell the way a bridge player writes a holding: `AQ` + `xxx`.
fn cell_name(cell: usize) -> String {
    let mask = cell / SPOTS;
    let honours: String = HONOURS
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << (HONOURS.len() - 1 - i)) != 0)
        .map(|(_, r)| format!("{r}"))
        .collect();
    let spots = "x".repeat(cell % SPOTS);
    if honours.is_empty() && spots.is_empty() {
        "void".to_owned()
    } else {
        format!("{honours}{spots}")
    }
}

/// The eight holdings a deal contributes, already canonicalised.
fn holdings(north: Hand, south: Hand, canon: &[usize; CELLS]) -> [usize; HOLDINGS] {
    let mut out = [0; HOLDINGS];
    for (slot, (hand, suit)) in out.iter_mut().zip(
        [north, south]
            .into_iter()
            .flat_map(|h| Suit::ASC.map(move |s| (h, s))),
    ) {
        *slot = canon[cell(hand[suit])];
    }
    out
}

/// Fold a cell too rare to fit into its nearest surviving neighbour by dropping
/// spot cards. Merging keeps the published table readable; ridge would not, and
/// it would pick a point on the null manifold by norm accident.
fn canonicalise(occupancy: &[u64; CELLS], floor: u64) -> [usize; CELLS] {
    let mut canon = [0; CELLS];
    for (c, slot) in canon.iter_mut().enumerate() {
        let mut target = c;
        while occupancy[target] < floor && target % SPOTS > 0 {
            target -= 1;
        }
        *slot = target;
    }
    canon
}

// ------------------------------------------------------------------ the fit

/// Normal equations for both heads at once: one Gram, two right-hand sides.
///
/// The mean fit is `T = w·n` and the variance fit is `E[r² | n] = v·n`, over
/// the *same* design, so they share `Σ n nᵀ` and differ only in the response.
struct Normals {
    gram: DMatrix<f64>,
    rhs: DVector<f64>,
}

impl Normals {
    fn new() -> Self {
        Self {
            gram: DMatrix::zeros(CELLS, CELLS),
            rhs: DVector::zeros(CELLS),
        }
    }

    /// `n` is a count vector with at most eight nonzeros, so `n nᵀ` is 64
    /// increments rather than a 288 × 288 outer product.
    fn push(&mut self, cells: &[usize; HOLDINGS], y: f64) {
        for &i in cells {
            self.rhs[i] += y;
            for &j in cells {
                self.gram[(i, j)] += 1.0;
            }
        }
    }

    /// The design is rank-deficient by construction (see [`gauge`]), so solve
    /// with a pseudo-inverse rather than a factorisation that has to choose a
    /// pivot — any least-squares solution will do, because [`gauge`] then pins
    /// a unique published representative.
    fn solve(&self) -> DVector<f64> {
        self.gram
            .clone()
            .pseudo_inverse(1e-9)
            .expect("symmetric Gram always has an SVD")
            * &self.rhs
    }
}

/// Move a fitted weight vector into the published gauge, returning the constant
/// that keeps predictions identical.
///
/// **There are two gauge freedoms, not one.** Every row satisfies `Σ n_c = 8`
/// *and* `Σ n_c · size(c) = 26`, so the null direction `z_c = 13 − 4·size(c)`
/// survives even with no intercept column: value slides freely between "per
/// holding" and "per card" forever, and two slices would otherwise publish
/// visibly different tables. Pin it by two population constraints,
/// `Σ p_c w_c = 0` and `Σ p_c size(c) w_c = 0`, which makes the returned
/// constant equal the population mean of the response and every weight read as
/// *excess versus an average holding, net of any pure per-card credit*.
///
/// The table is therefore only defined up to `w_c → w_c + α + β·size(c)` with
/// `8α + 26β = 0`, and that statement has to travel with it.
fn gauge(weights: &mut DVector<f64>, freq: &[f64; CELLS]) -> f64 {
    let moment = |k: i32| -> f64 {
        (0..CELLS)
            .map(|c| freq[c] * cell_size(c).powi(k))
            .sum::<f64>()
    };
    let weighted = |k: i32| -> f64 {
        (0..CELLS)
            .map(|c| freq[c] * cell_size(c).powi(k) * weights[c])
            .sum::<f64>()
    };

    // α + β·s1 = m0;  α·s1 + β·s2 = m1.
    let (s1, s2) = (moment(1), moment(2));
    let (m0, m1) = (weighted(0), weighted(1));
    let det = s2 - s1 * s1;
    let (alpha, beta) = if det.abs() < 1e-12 {
        (m0, 0.0)
    } else {
        ((m0 * s2 - m1 * s1) / det, (m1 - m0 * s1) / det)
    };

    for c in 0..CELLS {
        weights[c] -= alpha + beta * cell_size(c);
    }
    8.0f64.mul_add(alpha, 26.0 * beta)
}

/// A fitted valuation: `μ = mean_const + Σ mu[cell]`, `σ² = var_const + Σ var[cell]`.
///
/// **Two variance columns, because there are two honest questions.** `var` is
/// the *predictive* spread — fitted to the squared residual, so it answers "given
/// only this table's reading of the two hands, how uncertain is the trick count?"
/// and it is what makes `P(make)` calibrated. `truth` is the *physical* spread —
/// fitted against reshuffled E-W truth, so it answers "given the actual two
/// hands, how volatile are they?" and it is a property of the cards rather than
/// of the estimator.
///
/// They are not interchangeable and the gap between them is the table's own
/// ignorance. Publishing `var` as if it were `truth` was this artifact's first
/// real error; the benchmark is what caught it.
struct Table {
    label: Label,
    mu: DVector<f64>,
    var: DVector<f64>,
    mean_const: f64,
    var_const: f64,
    /// The physical column and its constant, when `--variance-truth` paid for it.
    truth: Option<(DVector<f64>, f64)>,
    canon: [usize; CELLS],
    freq: [f64; CELLS],
    occupancy: [u64; CELLS],
}

impl Table {
    fn predict(&self, cells: &[usize; HOLDINGS]) -> (f64, f64) {
        let mu = self.mean_const + cells.iter().map(|&c| self.mu[c]).sum::<f64>();
        let var = self.var_const + cells.iter().map(|&c| self.var[c]).sum::<f64>();
        // ponytail: unconstrained fit plus a floor, not NNLS — the report prints
        // how many deals hit the floor, and Lawson-Hanson goes in if that is ever
        // more than a rounding error.
        (mu, var.max(0.01))
    }

    /// The physical spread, when it was fitted.
    fn predict_truth(&self, cells: &[usize; HOLDINGS]) -> Option<f64> {
        let (v, k) = self.truth.as_ref()?;
        Some((k + cells.iter().map(|&c| v[c]).sum::<f64>()).max(0.01))
    }
}

// -------------------------------------------------------------- the corpus

/// One deal reduced to what both passes need.
/// One deal, packed — 20 bytes rather than 104, because 20M of them live in RAM
/// at once on a box that is also running someone else's A/B.
struct Row {
    cells: [u16; HOLDINGS],
    tricks: u8,
    pair_hcp: u8,
    /// Longest combined N-S suit — the pair-level axis an additive table cannot see.
    fit: u8,
    shortness: u8,
}

impl Row {
    fn cells(&self) -> [usize; HOLDINGS] {
        self.cells.map(usize::from)
    }
    fn tricks(&self) -> f64 {
        f64::from(self.tricks)
    }
}

fn hcp_of(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// Cards missing from every suit short of three, summed over the partnership —
/// the one extra term the HCP baseline gets, because prior work here put the
/// linear headroom in shape (~0.17 tricks) rather than in honours (~0.05).
fn shortness(north: Hand, south: Hand) -> u8 {
    [north, south]
        .iter()
        .flat_map(|h| Suit::ASC.map(|s| h[s].len()))
        .map(|len| 3_usize.saturating_sub(len) as u8)
        .sum()
}

/// Read a slice once and reduce it to rows, counting raw-cell occupancy on the
/// way — the merge threshold needs the counts before the rows can be keyed, and
/// re-reading 680 MB off NFS to learn them twice would be silly.
fn load(
    path: &str,
    skip: u64,
    count: usize,
    label: Label,
    canon: Option<&[usize; CELLS]>,
) -> std::io::Result<(Vec<Row>, [u64; CELLS])> {
    let boards = pons::pdd::load_slice(path, skip, count)?;
    let mut occ = [0; CELLS];
    let identity: [usize; CELLS] = std::array::from_fn(|c| c);
    let canon = canon.unwrap_or(&identity);

    let rows = boards
        .iter()
        .map(|(deal, table)| {
            let (north, south) = (deal[Seat::North], deal[Seat::South]);
            for hand in [north, south] {
                for suit in Suit::ASC {
                    occ[cell(hand[suit])] += 1;
                }
            }
            Row {
                cells: holdings(north, south, canon).map(|c| c as u16),
                tricks: label.tricks(table),
                pair_hcp: hcp_of(north) + hcp_of(south),
                fit: Suit::ASC
                    .iter()
                    .map(|&s| north[s].len() + south[s].len())
                    .max()
                    .unwrap_or(0) as u8,
                shortness: shortness(north, south),
            }
        })
        .collect();
    Ok((rows, occ))
}

/// Re-key rows once the merge threshold is known — cheaper than a second read.
fn recanonicalise(rows: &mut [Row], canon: &[usize; CELLS]) {
    for row in rows {
        for c in &mut row.cells {
            *c = canon[usize::from(*c)] as u16;
        }
    }
}

// ------------------------------------------------------------------ scoring

/// Mean and SD of a stream, and the tail counts the reliability diagram needs.
#[derive(Default, Clone)]
struct Moment {
    n: u64,
    sum: f64,
    sumsq: f64,
}

impl Moment {
    fn push(&mut self, x: f64) {
        self.n += 1;
        self.sum += x;
        self.sumsq += x * x;
    }
    fn mean(&self) -> f64 {
        self.sum / self.n as f64
    }
    fn sd(&self) -> f64 {
        let m = self.mean();
        (self.sumsq / self.n as f64 - m * m).max(0.0).sqrt()
    }
    fn rms(&self) -> f64 {
        (self.sumsq / self.n as f64).sqrt()
    }
}

/// Least squares for the small baselines — `y ~ [1, x…]` with a handful of columns.
fn small_fit(rows: &[Row], columns: &[fn(&Row) -> f64]) -> Vec<f64> {
    let k = columns.len() + 1;
    let mut gram = DMatrix::<f64>::zeros(k, k);
    let mut rhs = DVector::<f64>::zeros(k);
    for row in rows {
        let mut x = vec![1.0];
        x.extend(columns.iter().map(|f| f(row)));
        for i in 0..k {
            rhs[i] += x[i] * row.tricks();
            for j in 0..k {
                gram[(i, j)] += x[i] * x[j];
            }
        }
    }
    let beta: DVector<f64> = gram.pseudo_inverse(1e-9).expect("small Gram has an SVD") * rhs;
    beta.iter().copied().collect()
}

fn small_rmse(rows: &[Row], beta: &[f64], columns: &[fn(&Row) -> f64]) -> f64 {
    let mut m = Moment::default();
    for row in rows {
        let mut yhat = beta[0];
        for (b, f) in beta[1..].iter().zip(columns) {
            yhat += b * f(row);
        }
        m.push(row.tricks() - yhat);
    }
    m.rms()
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    println!("== corpus ==");
    let floor = 200;
    let (mut train, occ) = load(&args.deals, args.skip, args.count, args.label, None)?;
    let canon = canonicalise(&occ, floor);
    recanonicalise(&mut train, &canon);
    let live = (0..CELLS).filter(|&c| canon[c] == c).count();
    println!(
        "  bank={} fit slice {}..{}  raw cells={CELLS} live={live} (occupancy floor {floor})",
        args.deals,
        args.skip,
        args.skip + args.count as u64,
    );

    let (test, _) = load(
        &args.deals,
        args.test_skip,
        args.test_count,
        args.label,
        Some(&canon),
    )?;
    println!(
        "  train={} deals  held-out={} deals",
        train.len(),
        test.len()
    );
    assert!(
        args.test_skip >= args.skip + args.count as u64
            || args.skip >= args.test_skip + args.test_count as u64,
        "held-out slice must be disjoint from the fitting slice"
    );

    // Cell frequency over the fitting slice — the gauge's population weights.
    let mut freq = [0.0; CELLS];
    for row in &train {
        for &c in &row.cells() {
            freq[c] += 1.0;
        }
    }
    let total: f64 = freq.iter().sum();
    for f in &mut freq {
        *f /= total;
    }

    // Pass A — the mean head. ponytail: plain OLS, no 1/σ² reweighting sweep;
    // at 20M rows the efficiency gain is invisible and it halves the code.
    let mut normals = Normals::new();
    for row in &train {
        normals.push(&row.cells(), row.tricks());
    }
    let gram = normals.gram.clone();
    let mut mu = normals.solve();

    // Pass B — the variance head. `E[r² | n] = v·n` is plain OLS of the squared
    // residual on the same design: same Gram, second right-hand side, no link
    // function and no iteration, and consistent whatever shape the conditional
    // has (it is not Gaussian).
    let mut rhs_var = DVector::<f64>::zeros(CELLS);
    for row in &train {
        let resid = row.tricks() - row.cells().iter().map(|&c| mu[c]).sum::<f64>();
        for &c in &row.cells() {
            rhs_var[c] += resid * resid;
        }
    }
    let mut var = gram.pseudo_inverse(1e-9).expect("Gram has an SVD") * rhs_var;

    let mean_const = gauge(&mut mu, &freq);
    let var_const = gauge(&mut var, &freq);
    let truth = if args.variance_truth > 0 {
        Some(fit_physical_variance(&canon, &freq, &args)?)
    } else {
        None
    };
    let table = Table {
        label: args.label,
        mu,
        var,
        mean_const,
        var_const,
        truth,
        canon,
        freq,
        occupancy: occ,
    };
    println!(
        "\n== fit: {} ==\n  mu = {mean_const:.3} + sum of 8 holding weights (tricks)\n  \
         sigma^2 = {var_const:.3} + sum of 8 holding weights (trick^2)",
        args.label.name()
    );

    report_table(&table);
    report_accuracy(&table, &train, &test);
    report_sigma_gate(&table, &test);
    if args.benchmark > 0 {
        assert!(
            args.bench_skip >= args.test_skip + args.test_count as u64,
            "benchmark slice must be disjoint from the fitting and held-out slices"
        );
        benchmark(&table, &args)?;
    }

    if let Some(path) = &args.json {
        write_json(&table, path)?;
        println!("\nwrote {path}");
    }
    if let Some(path) = &args.markdown {
        write_markdown(&table, path)?;
        println!("wrote {path}");
    }
    Ok(())
}

/// The extremes of both columns — the whole table goes to the artifact files.
fn report_table(table: &Table) {
    let mut live: Vec<usize> = (0..CELLS).filter(|&c| table.canon[c] == c).collect();

    println!("\n== the table, by mean contribution ==");
    live.sort_by(|&a, &b| table.mu[b].total_cmp(&table.mu[a]));
    for &c in live.iter().take(6).chain(live.iter().rev().take(3).rev()) {
        println!(
            "  {:<10} mu {:+.3}  var {:+.3}  n={}",
            cell_name(c),
            table.mu[c],
            table.var[c],
            table.occupancy[c]
        );
    }

    println!("\n== the table, by variance contribution -- the column no point count has ==");
    live.sort_by(|&a, &b| table.var[b].total_cmp(&table.var[a]));
    for &c in live.iter().take(6).chain(live.iter().rev().take(3).rev()) {
        println!(
            "  {:<10} mu {:+.3}  var {:+.3}  n={}",
            cell_name(c),
            table.mu[c],
            table.var[c],
            table.occupancy[c]
        );
    }
}

fn report_accuracy(table: &Table, train: &[Row], test: &[Row]) {
    const HCP: fn(&Row) -> f64 = |r| f64::from(r.pair_hcp);
    const SHORT: fn(&Row) -> f64 = |r| f64::from(r.shortness);

    println!("\n== held-out accuracy (RMSE in tricks; lower is better) ==");
    for (label, columns) in [
        ("HCP only", &[HCP][..]),
        ("HCP + shortness", &[HCP, SHORT][..]),
    ] {
        let beta = small_fit(train, columns);
        println!("  {label:<20} {:.4}", small_rmse(test, &beta, columns));
    }

    let (mut resid, mut floored, mut below) = (Moment::default(), 0u64, 0u64);
    for row in test {
        let (mu, var) = table.predict(&row.cells());
        resid.push(row.tricks() - mu);
        floored += u64::from(var <= 0.01);
        below += u64::from(row.tricks() < mu);
    }
    println!("  {:<20} {:.4}", "holding table", resid.rms());
    println!(
        "  bias {:+.4}   below mu {:.1}% (nominal 50, the skew diagnostic)   \
         variance floored on {floored} deals",
        resid.mean(),
        100.0 * below as f64 / test.len() as f64,
    );

    // Reliability, not central-50% coverage: conditional on both hands the trick
    // count often has a two-trick jump on a suit break, and coverage is blind to
    // exactly that while the decision lives in the tail.
    for target in [9u8, 10] {
        let mut deciles = [(0u64, 0u64); 10];
        for row in test {
            let (mu, var) = table.predict(&row.cells());
            let p = f64::from(
                Gaussian {
                    mean: mu as f32,
                    sd: var.sqrt() as f32,
                }
                .p_at_least(target),
            );
            let slot = &mut deciles[((p * 10.0) as usize).min(9)];
            slot.0 += 1;
            slot.1 += u64::from(row.tricks() >= f64::from(target));
        }
        println!("\n  reliability of P(T >= {target}), by predicted decile:");
        for (d, &(n, hits)) in deciles.iter().enumerate() {
            if n == 0 {
                continue;
            }
            println!(
                "    {:>3}-{:>3}%  n={n:>8}  actual {:>5.1}%",
                d * 10,
                d * 10 + 10,
                100.0 * hits as f64 / n as f64
            );
        }
    }
}

fn report_sigma_gate(table: &Table, test: &[Row]) {
    println!("\n== sigma gate 1: is the variance column algebraically new? ==");
    // If v lies in span{1, size, mu} across the cells, predicted sigma is a
    // deterministic function of predicted mu and any within-bucket spread is
    // zero by algebra rather than by physics.
    let live: Vec<usize> = (0..CELLS).filter(|&c| table.canon[c] == c).collect();
    let mut gram = DMatrix::<f64>::zeros(3, 3);
    let mut rhs = DVector::<f64>::zeros(3);
    let (mut tss, mut mean_v) = (0.0, 0.0);
    for &c in &live {
        mean_v += table.freq[c] * table.var[c];
    }
    for &c in &live {
        let x = [1.0, cell_size(c), table.mu[c]];
        for i in 0..3 {
            rhs[i] += table.freq[c] * x[i] * table.var[c];
            for j in 0..3 {
                gram[(i, j)] += table.freq[c] * x[i] * x[j];
            }
        }
        tss += table.freq[c] * (table.var[c] - mean_v).powi(2);
    }
    let beta: DVector<f64> = gram.pseudo_inverse(1e-12).expect("3x3 has an SVD") * rhs;
    let rss: f64 = live
        .iter()
        .map(|&c| {
            let fitted = beta[0] + beta[1] * cell_size(c) + beta[2] * table.mu[c];
            table.freq[c] * (table.var[c] - fitted).powi(2)
        })
        .sum();
    println!(
        "  var ~ 1 + size + mu across cells: R^2 = {:.3}  =>  {:.1}% of the variance column is \
         NOT recoverable from the mean column",
        1.0 - rss / tss,
        100.0 * rss / tss
    );

    println!(
        "\n== sigma gate 2: assumption-free -- does predicted sigma sort EMPIRICAL spread? =="
    );
    // Bucket by predicted mu, split each bucket at the median predicted sigma,
    // then compare the *realised* SD of the two halves. Comparing predicted
    // spread against predicted spread only tests the model against itself.
    let mut buckets: BTreeMap<i64, Vec<(f64, f64)>> = BTreeMap::new();
    for row in test {
        let (mu, var) = table.predict(&row.cells());
        buckets
            .entry((mu * 2.0).round() as i64)
            .or_default()
            .push((var.sqrt(), row.tricks()));
    }
    for (key, rows) in &mut buckets {
        if rows.len() < 20_000 {
            continue;
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        let half = rows.len() / 2;
        let (lo, hi) = rows.split_at(half);
        let stat = |part: &[(f64, f64)]| {
            let mut m = Moment::default();
            let mut pred = Moment::default();
            for &(sd, t) in part {
                m.push(t);
                pred.push(sd);
            }
            (pred.mean(), m.sd())
        };
        let ((pred_lo, emp_lo), (pred_hi, emp_hi)) = (stat(lo), stat(hi));
        println!(
            "  mu~{:>5.1}  n={:>8}  narrow half: predicted {pred_lo:.3} / empirical {emp_lo:.3}  |  \
             wide half: predicted {pred_hi:.3} / empirical {emp_hi:.3}  |  \
             empirical spread {:+.3}, recovered {:.0}%",
            *key as f64 / 2.0,
            rows.len(),
            emp_hi - emp_lo,
            100.0 * (pred_hi - pred_lo) / (emp_hi - emp_lo),
        );
    }

    println!("\n== sigma gate 3: does it flip decisions where the decision is live? ==");
    // The currency that matters is not a probability gap in the abstract: it is
    // whether a sigma-aware gate and a sigma-blind one disagree about bidding
    // game, on the boards close enough to the boundary for it to be possible.
    let blind_sd = {
        let mut m = Moment::default();
        for row in test {
            let (mu, _) = table.predict(&row.cells());
            m.push(row.tricks() - mu);
        }
        m.rms()
    };
    let (mut live_n, mut flips, mut flip_right) = (0u64, 0u64, 0u64);
    for row in test {
        let (mu, var) = table.predict(&row.cells());
        if (mu - 8.5).abs() >= 0.5 {
            continue;
        }
        live_n += 1;
        let p = |sd: f64| {
            Gaussian {
                mean: mu as f32,
                sd: sd as f32,
            }
            .p_at_least(9)
                >= 0.375
        };
        let (aware, blind) = (p(var.sqrt()), p(blind_sd));
        if aware != blind {
            flips += 1;
            flip_right += u64::from(aware == (row.tricks() >= 9.0));
        }
    }
    println!(
        "  sigma-blind SD = {blind_sd:.3} tricks (one number for every hand)\n  \
         live band |mu - 8.5| < 0.5: n={live_n}, gate flips on {flips} ({:.1}%), \
         and the sigma-aware call was right on {:.1}% of those",
        100.0 * flips as f64 / live_n.max(1) as f64,
        100.0 * flip_right as f64 / flips.max(1) as f64,
    );

    println!("\n== how much of the pair-level spread can an ADDITIVE table see? ==");
    // The honest limitation: Cut C's driver is the *combined* suit length, and a
    // per-hand table cannot see alignment. It credits "long suit somewhere", not
    // "opposite partner's". This measures what that costs.
    let mut grid: BTreeMap<(u8, usize), (Moment, Moment)> = BTreeMap::new();
    for row in test {
        if !(18..=29).contains(&row.pair_hcp) {
            continue;
        }
        let (_, var) = table.predict(&row.cells());
        let entry = grid
            .entry((
                (row.pair_hcp - 18) / 3,
                usize::from(row.fit).saturating_sub(7).min(3),
            ))
            .or_default();
        entry.0.push(row.tricks());
        entry.1.push(var.sqrt());
    }
    for band in 0..4u8 {
        let cells: Vec<_> = (0..4)
            .filter_map(|f| grid.get(&(band, f)))
            .filter(|(m, _)| m.n >= 2_000)
            .collect();
        let (Some(emp_lo), Some(emp_hi)) = (
            cells.iter().map(|(m, _)| m.sd()).reduce(f64::min),
            cells.iter().map(|(m, _)| m.sd()).reduce(f64::max),
        ) else {
            continue;
        };
        let (pred_lo, pred_hi) = (
            cells.iter().map(|(_, p)| p.mean()).fold(f64::MAX, f64::min),
            cells.iter().map(|(_, p)| p.mean()).fold(f64::MIN, f64::max),
        );
        println!(
            "  {}-{} HCP: empirical sigma spread across fit classes {:.3}, table predicts {:.3} \
             ({:.0}% recovered)",
            18 + 3 * band,
            20 + 3 * band,
            emp_hi - emp_lo,
            pred_hi - pred_lo,
            100.0 * (pred_hi - pred_lo) / (emp_hi - emp_lo),
        );
    }
}

// -------------------------------------------------- the physical variance

/// Fit the variance head against **reshuffled truth** instead of against the
/// model's own residual.
///
/// OLS of the squared residual estimates `Var(T | N,S) + (μ_true − μ̂)²` — the
/// hand's uncertainty *plus the model's squared bias*. On the notrump table the
/// second term turns out to dominate, so the published column tracked where the
/// estimator was ignorant rather than where the cards were volatile. This fits
/// the first term alone.
///
/// **Two shuffles per pair is enough, and that is not a compromise.**
/// `E[(T₁ − T₂)²] = 2·Var(T | N,S)` exactly for two independent draws, so
/// `(T₁ − T₂)²/2` is an *unbiased* per-row response — noisy, but this is a
/// regression, and per-row noise costs rows rather than correctness. Buying the
/// same precision with fewer, better-estimated pairs would cost far more solves.
fn fit_physical_variance(
    canon: &[usize; CELLS],
    freq: &[f64; CELLS],
    args: &Args,
) -> std::io::Result<(DVector<f64>, f64)> {
    let boards = pons::pdd::load_slice(&args.deals, args.truth_skip, args.variance_truth)?;
    let mut rng = StdRng::seed_from_u64(args.seed ^ 0x5eed);
    let flags = args.label.flags();
    let shuffles = args.variance_shuffles.max(2);

    println!(
        "\n== fitting the PHYSICAL variance column ==\n  {} pairs x {shuffles} shuffles = {} solves",
        boards.len(),
        boards.len() * shuffles,
    );

    let mut normals = Normals::new();
    let mut observed = Moment::default();
    // Batch many pairs into one `solve_deals`. Two layouts per lock spends more
    // on acquiring the solver and standing up its batch than on the search — it
    // measured ~20x slower than this. The solver still runs on the main thread
    // only, which is the invariant that actually matters.
    for chunk in boards.chunks(PAIRS_PER_BATCH) {
        let mut layouts = Vec::with_capacity(chunk.len() * shuffles);
        for (deal, _) in chunk {
            let mut builder = Builder::new();
            builder[Seat::North] = deal[Seat::North];
            builder[Seat::South] = deal[Seat::South];
            let partial = builder
                .build_partial()
                .expect("a bank deal's N-S hands are disjoint and thirteen cards each");
            layouts.extend(fill_deals(&mut rng, partial).take(shuffles));
        }
        let solved = Solver::lock(None).solve_deals(&layouts, flags);

        // `solve_deals` preserves input order, so each pair owns one contiguous run.
        for ((deal, _), tables) in chunk.iter().zip(solved.chunks(shuffles)) {
            let mut sample = Moment::default();
            for table in tables {
                sample.push(f64::from(args.label.tricks(table)));
            }
            // The *sample* variance, Bessel-corrected: unbiased for
            // `Var(T | N,S)` at every `shuffles >= 2`.
            let n = sample.n as f64;
            let unbiased = sample.sd() * sample.sd() * n / (n - 1.0);
            observed.push(unbiased);
            normals.push(
                &holdings(deal[Seat::North], deal[Seat::South], canon),
                unbiased,
            );
        }
    }

    let mut v = normals.solve();
    let k = gauge(&mut v, freq);
    println!(
        "  mean true conditional variance {:.3} trick^2 (sigma {:.3}); fitted constant {k:.3}",
        observed.mean(),
        observed.mean().sqrt(),
    );
    Ok((v, k))
}

// --------------------------------------------------------------- benchmark

/// Price the table against the **true** conditional moments.
///
/// Every other check in this file compares one prediction against one realised
/// deal, or against a bucketed proxy. This one fixes both N-S hands and reshuffles
/// the East-West split, so the sample moments converge on `E[T | N, S]` and
/// `sd(T | N, S)` — precisely what the two columns claim to be.
///
/// **There is no sampler bias here, and that is the point.** Conditioned on the
/// two N-S hands and nothing else, the posterior over the hidden 26 cards *is*
/// uniform over E-W splits. No inference, no envelope, no rule replay — so this
/// is the unbiased denominator [`docs/ai-bidder/evaluator-net.md`] records as
/// still owed for the learned net. A per-hand table is the one estimator cheap
/// enough to get it for free.
fn benchmark(table: &Table, args: &Args) -> std::io::Result<()> {
    let boards = pons::pdd::load_slice(&args.deals, args.bench_skip, args.benchmark)?;
    let mut rng = StdRng::seed_from_u64(args.seed);
    let flags = args.label.flags();

    println!(
        "\n== benchmark: fixed N-S, shuffled E-W -- against the TRUE conditional moments ==\n  \
         {} pairs x {} shuffles = {} solves, label = {}",
        boards.len(),
        args.shuffles,
        boards.len() * args.shuffles,
        args.label.name(),
    );

    // (predicted mu, predictive sd, true mu, true sd, physical sd) per pair.
    let mut priced: Vec<(f64, f64, f64, f64, f64)> = Vec::with_capacity(boards.len());
    for (deal, _) in &boards {
        let (north, south) = (deal[Seat::North], deal[Seat::South]);
        let mut builder = Builder::new();
        builder[Seat::North] = north;
        builder[Seat::South] = south;
        let partial = builder
            .build_partial()
            .expect("a bank deal's N-S hands are disjoint and thirteen cards each");

        // One batched solve per pair, on the main thread: the solver is a
        // process-global lock and must never meet rayon (iron rule).
        let layouts: Vec<_> = fill_deals(&mut rng, partial).take(args.shuffles).collect();
        let mut truth = Moment::default();
        for solved in Solver::lock(None).solve_deals(&layouts, flags) {
            truth.push(f64::from(args.label.tricks(&solved)));
        }

        let cells = holdings(north, south, &table.canon);
        let (mu, var) = table.predict(&cells);
        let physical = table.predict_truth(&cells).map_or(f64::NAN, f64::sqrt);
        priced.push((mu, var.sqrt(), truth.mean(), truth.sd(), physical));
    }

    // Sampling noise floors. The true moments come from `shuffles` draws, so the
    // sample mean carries SE = sigma/sqrt(M) and the sample sd SE ~ sigma/sqrt(2M).
    // Both errors are roughly Gaussian and MAE = sqrt(2/pi)*SE, so the factors
    // cancel and MAEs deconvolve in QUADRATURE — never linearly. Getting that
    // wrong understated the learned net by ~45% once already.
    let m = args.shuffles as f64;
    let scale = (2.0 / std::f64::consts::PI).sqrt();
    let (mut mu_err, mut sd_err, mut spread) =
        (Moment::default(), Moment::default(), Moment::default());
    let (mut floor_mu, mut floor_sd) = (Moment::default(), Moment::default());
    for &(mu, sd, t_mu, t_sd, _) in &priced {
        mu_err.push((mu - t_mu).abs());
        sd_err.push((sd - t_sd).abs());
        spread.push(sd - t_sd);
        floor_mu.push(scale * t_sd / m.sqrt());
        floor_sd.push(scale * t_sd / (2.0 * m).sqrt());
    }
    let deconvolve = |err: f64, floor: f64| (err * err - floor * floor).max(0.0).sqrt();

    // Does a sigma column track truth at all? This is the number of record.
    let corr = |predicted: &dyn Fn(usize) -> f64| {
        let (mut a, mut b) = (Moment::default(), Moment::default());
        for (i, p) in priced.iter().enumerate() {
            a.push(predicted(i));
            b.push(p.3);
        }
        let cov: f64 = priced
            .iter()
            .enumerate()
            .map(|(i, p)| (predicted(i) - a.mean()) * (p.3 - b.mean()))
            .sum::<f64>()
            / priced.len() as f64;
        cov / (a.sd() * b.sd())
    };

    println!(
        "  mu   MAE {:.4}  (noise floor {:.4}, deconvolved {:.4})   bias {:+.4}",
        mu_err.mean(),
        floor_mu.mean(),
        deconvolve(mu_err.mean(), floor_mu.mean()),
        priced.iter().map(|p| p.0 - p.2).sum::<f64>() / priced.len() as f64,
    );
    println!(
        "  sd   MAE {:.4}  (noise floor {:.4}, deconvolved {:.4})   signed spread {:+.4} \
         (predicted {:.3} vs true {:.3})",
        sd_err.mean(),
        floor_sd.mean(),
        deconvolve(sd_err.mean(), floor_sd.mean()),
        spread.mean(),
        priced.iter().map(|p| p.1).sum::<f64>() / priced.len() as f64,
        priced.iter().map(|p| p.3).sum::<f64>() / priced.len() as f64,
    );
    println!(
        "  corr(predictive sd, true sd) = {:.3}",
        corr(&|i| priced[i].1)
    );
    if table.truth.is_some() {
        let (mut err, mut spread) = (Moment::default(), Moment::default());
        for p in &priced {
            err.push((p.4 - p.3).abs());
            spread.push(p.4 - p.3);
        }
        println!(
            "  PHYSICAL column: MAE {:.4} (deconvolved {:.4})  signed spread {:+.4}  \
             corr {:.3}",
            err.mean(),
            deconvolve(err.mean(), floor_sd.mean()),
            spread.mean(),
            corr(&|i| priced[i].4),
        );
    }

    // Sorted by TRUE sd, so the table reads as calibration rather than as fit:
    // a column that only reproduced the mean would be flat down the middle here.
    println!("\n  by true sigma quintile:");
    let mut sorted = priced.clone();
    sorted.sort_by(|a, b| a.3.total_cmp(&b.3));
    for (q, chunk) in sorted.chunks(priced.len().div_ceil(5)).enumerate() {
        let avg = |f: fn(&(f64, f64, f64, f64, f64)) -> f64| {
            chunk.iter().map(f).sum::<f64>() / chunk.len() as f64
        };
        let physical = if table.truth.is_some() {
            format!("  physical {:.3}", avg(|p| p.4))
        } else {
            String::new()
        };
        println!(
            "    Q{}  n={:>5}  true sigma {:.3}  predictive {:.3} ({:+.3}){physical}   \
             true mu {:.2}  predicted {:.2}",
            q + 1,
            chunk.len(),
            avg(|p| p.3),
            avg(|p| p.1),
            avg(|p| p.1 - p.3),
            avg(|p| p.2),
            avg(|p| p.0),
        );
    }
    Ok(())
}

// ---------------------------------------------------------------- artifacts

fn write_json(table: &Table, path: &str) -> std::io::Result<()> {
    let mut out = std::fs::File::create(path)?;
    writeln!(out, "{{")?;
    writeln!(out, "  \"label\": \"{}\",", table.label.name())?;
    writeln!(out, "  \"mean_const\": {:.4},", table.mean_const)?;
    writeln!(out, "  \"var_const\": {:.4},", table.var_const)?;
    if let Some((_, k)) = &table.truth {
        writeln!(out, "  \"physical_var_const\": {k:.4},")?;
    }
    writeln!(
        out,
        "  \"columns\": \"mu (tricks), predictive var (trick^2){}, deals\",",
        if table.truth.is_some() {
            ", physical var (trick^2)"
        } else {
            ""
        }
    )?;
    writeln!(
        out,
        "  \"gauge\": \"weights are excess vs an average holding; the table is defined up to \
         w_c -> w_c + a + b*size(c) with 8a + 26b = 0\","
    )?;
    writeln!(out, "  \"holdings\": {{")?;
    let live: Vec<usize> = (0..CELLS).filter(|&c| table.canon[c] == c).collect();
    for (i, &c) in live.iter().enumerate() {
        writeln!(
            out,
            "    \"{}\": [{:.4}, {:.4}{}, {}]{}",
            cell_name(c),
            table.mu[c],
            table.var[c],
            table
                .truth
                .as_ref()
                .map_or(String::new(), |(v, _)| format!(", {:.4}", v[c])),
            table.occupancy[c],
            if i + 1 == live.len() { "" } else { "," }
        )?;
    }
    writeln!(out, "  }}\n}}")
}

fn write_markdown(table: &Table, path: &str) -> std::io::Result<()> {
    let mut out = std::fs::File::create(path)?;
    writeln!(
        out,
        "# Binky Points with error bars — {}\n",
        table.label.name()
    )?;
    writeln!(
        out,
        "A hand's valuation is the sum of its four holdings' entries. A partnership sums \
         both hands. Then\n\n```\nmu     = {:.3} + sum of the 8 mu entries        (tricks)\n\
         sigma2 = {:.3} + sum of the 8 var entries       (trick^2)\nP(T >= k) = Phi((mu - k + 0.5) / sqrt(sigma2))\n```\n",
        table.mean_const, table.var_const
    )?;
    writeln!(
        out,
        "The label is {}. Weights are **excess versus an average \
         holding**: the fit is rank-deficient by two directions (`sum n = 8` and \
         `sum n*size = 26`), so the table is only defined up to \
         `w_c -> w_c + a + b*size(c)` with `8a + 26b = 0`, and it is pinned here by \
         `sum p_c w_c = 0` and `sum p_c size(c) w_c = 0`.\n",
        table.label.name()
    )?;
    let physical_header = if table.truth.is_some() {
        " physical var (trick²) |"
    } else {
        ""
    };
    writeln!(
        out,
        "| holding | mu (tricks) | predictive var (trick²) |{physical_header} deals |"
    )?;
    writeln!(
        out,
        "| --- | ---: | ---: |{} ---: |",
        if table.truth.is_some() { " ---: |" } else { "" }
    )?;
    let mut live: Vec<usize> = (0..CELLS).filter(|&c| table.canon[c] == c).collect();
    live.sort_by(|&a, &b| {
        cell_size(a)
            .total_cmp(&cell_size(b))
            .then(table.mu[b].total_cmp(&table.mu[a]))
    });
    for &c in &live {
        writeln!(
            out,
            "| {} | {:+.3} | {:+.3} |{} {} |",
            cell_name(c),
            table.mu[c],
            table.var[c],
            table
                .truth
                .as_ref()
                .map_or(String::new(), |(v, _)| format!(" {:+.3} |", v[c])),
            table.occupancy[c]
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
