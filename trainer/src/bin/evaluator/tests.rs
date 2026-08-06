use super::Arm;
use clap::ValueEnum as _;

/// Pin every arm's live column count, since a typo in an offset list would
/// silently change what the net sees and leave no trace in the metrics. The
/// widths here are transcribed independently of [`Arm::spec`]; the assert
/// inside [`Arm::mask`] is what ties the two together.
///
/// Also checks that the name we log matches the name `--arm` accepts.
#[test]
fn arm_live_widths() {
    for (arm, want) in [
        (Arm::Full, 79),
        (Arm::Baseline, 40),
        (Arm::Bits, 60),
        (Arm::BitsNohcp, 56),
        (Arm::Ben, 54),
        (Arm::BitsWidth, 75),
        (Arm::BaselineDropUpgrade, 39),
        (Arm::BaselineDropHcp, 39),
        (Arm::BaselineDropBoth, 38),
        (Arm::BenOracle, 62),
        (Arm::BaselineDropBothOracle, 46),
        (Arm::BenOracleQuality, 66),
        (Arm::BenOracleShortness, 66),
        (Arm::BenOracleControls, 78),
        (Arm::BenOracleStopper, 66),
        (Arm::BenAuction, 214),
        (Arm::BenCalls, 94),
        (Arm::ShapeControl, 94),
        (Arm::ShapeGauss, 94),
        (Arm::ShapeMass, 73),
        (Arm::ShapeGaussMass, 97),
        (Arm::ShapeHist, 238),
        (Arm::ShapeHistMass, 265),
        (Arm::ShapeHybrid, crate::SHAPE_FEATURES),
        (Arm::PtsControl, 97),
        (Arm::PtsGauss, 97),
        (Arm::PtsGaussMass, 100),
        (Arm::PtsBoth, 106),
        (Arm::PtsHcpEnds, 103),
        (Arm::PtsHcpBoth, 112),
        (Arm::PtsLenControl, 94),
        (Arm::PtsLenHcpEnds, 100),
    ] {
        let name = arm.name();
        assert_eq!(
            arm.mask().iter().filter(|&&k| k).count(),
            want,
            "arm {name}"
        );
        assert_eq!(
            arm.to_possible_value().expect("no skipped arms").get_name(),
            name
        );
    }
}
