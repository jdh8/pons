use contract_bridge::deck::full_deal;
use dds_bridge::solver::*;
use dds_bridge_sys as sys;

#[test]
#[cfg_attr(miri, ignore = "dds-bridge-sys performs FFI which Miri cannot execute")]
fn test_solving_deals() {
    const N: usize = sys::MAXNOOFBOARDS as usize * 2;
    let deals: [_; N] = core::array::from_fn(|_| full_deal(&mut rand::rng()));
    let mut solver = Solver::default();
    let array = deals.map(|x| solver.solve_deal(x));
    let vec = solve_deals(&deals);
    assert_eq!(array, vec.as_slice());
}
