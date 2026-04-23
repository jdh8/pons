use dds_bridge::solver::*;
use dds_bridge_sys as sys;
use pons::full_deal;

#[test]
#[cfg_attr(miri, ignore = "dds-bridge-sys performs FFI which Miri cannot execute")]
fn test_solving_deals() {
    const N: usize = sys::MAXNOOFBOARDS as usize * 2;
    let deals: [_; N] = core::array::from_fn(|_| full_deal(&mut rand::rng()));
    let solver = Solver::lock();
    let array = deals.map(|x| solver.solve_deal(x));
    let vec = solver.solve_deals(&deals, NonEmptyStrainFlags::ALL);
    core::mem::drop(solver);
    assert_eq!(array, vec.as_slice());
}
