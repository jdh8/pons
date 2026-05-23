use contract_bridge::deck::full_deal;
use ddss::{NonEmptyStrainFlags, Solver};

#[test]
#[cfg_attr(miri, ignore = "ddss-sys performs FFI which Miri cannot execute")]
fn test_solving_deals() {
    // Twice the per-chunk capacity of `Solver::solve_deals` with all five
    // strains selected, forcing the batch path to cross at least one chunk
    // boundary.  Heap-allocated because the array is far too large for the
    // default thread stack.
    const N: usize = ddss_sys::MAXNOOFBOARDS as usize / 5 * 2;
    let deals: Vec<_> = (0..N).map(|_| full_deal(&mut rand::rng())).collect();
    let solver = Solver::lock();
    let array: Vec<_> = deals.iter().map(|&x| solver.solve_deal(x)).collect();
    let vec = solver.solve_deals(&deals, NonEmptyStrainFlags::ALL);
    assert_eq!(array, vec);
}
