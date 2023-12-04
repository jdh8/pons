use crate::contract::Strain;
use crate::deal::{Holding, Hand, SmallSet};

pub trait HandEvaluator {
    fn eval(&self, hand: Hand) -> f64;
}

pub struct SimpleEvaluator<F: Fn(Holding) -> f64>(F);

impl<F: Fn(Holding) -> f64> HandEvaluator for SimpleEvaluator<F> {
    fn eval(&self, hand: Hand) -> f64 {
        self.0(hand[Strain::Clubs]) +
        self.0(hand[Strain::Diamonds]) +
        self.0(hand[Strain::Hearts]) +
        self.0(hand[Strain::Spades])
    }
}

fn hcp(holding: Holding) -> f64 {
    f64::from(
        4 * i32::from(holding.contains(14)) +
        3 * i32::from(holding.contains(13)) +
        2 * i32::from(holding.contains(12)) +
        i32::from(holding.contains(11)))
}

fn fifths(holding: Holding) -> f64 {
    (if holding.contains(14) { 4.0 } else { 0.0 } +
     if holding.contains(13) { 2.8 } else { 0.0 } +
     if holding.contains(12) { 1.8 } else { 0.0 } +
     if holding.contains(11) { 1.0 } else { 0.0 } +
     if holding.contains(10) { 0.4 } else { 0.0 })
}

const HCP: SimpleEvaluator<fn(Holding) -> f64> = SimpleEvaluator(hcp);
const FIFTHS: SimpleEvaluator<fn(Holding) -> f64> = SimpleEvaluator(fifths);