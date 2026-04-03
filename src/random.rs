use core::num::Wrapping;
use dds_bridge::deal::{Deal, Hand, Seat, SmallSet as _};
use dds_bridge::deck::Deck;
use dds_bridge::solver::SystemError;
use rand::Rng;

pub use dds_bridge::deck::full_deal;

/// Given a deal, randomly fill the remaining cards and filter the results.
///
/// The filter is applied before collecting the results, so the resulting vector
/// still has `n` deals.
///
/// - `deal`: The initial deal with some cards already assigned.
/// - `n`: The number of deals to generate.
/// - `filter`: A constraint to filter deals.
///
/// # Errors
///
/// [`dds_bridge::solver::SystemError`] if `deal` is invalid, such as
///
/// - Any hand having more than 13 cards.
/// - Duplicate cards across hands.
pub fn fill_n_filtered_deals(
    rng: &mut (impl Rng + ?Sized),
    deal: &Deal,
    n: usize,
    filter: impl FnMut(&Deal) -> bool,
) -> Result<Vec<Deal>, SystemError> {
    if deal.0.into_iter().any(|hand| hand.len() > 13) {
        return Err(SystemError::TooManyCards);
    }
    if deal.0.into_iter().reduce(Hand::intersection) != Some(Hand::EMPTY) {
        return Err(SystemError::DuplicateCards);
    }

    let deck = Deck::from(
        deal.0
            .into_iter()
            .fold(Hand::ALL, Hand::symmetric_difference),
    );

    #[allow(clippy::missing_panics_doc)]
    let shortest = Seat::ALL
        .into_iter()
        .min_by_key(|&seat| deal[seat].len())
        .expect("Seat::ALL shall not be empty");

    Ok(core::iter::repeat_with(|| {
        let mut deck = deck.clone();
        let mut deal = *deal;

        for i in 1..=3 {
            let hand = &mut deal[shortest + Wrapping(i)];
            *hand = *hand | deck.partial_shuffle(rng, 13 - hand.len());
        }
        deal[shortest] = deal[shortest] | deck.collect();
        deal
    })
    .filter(filter)
    .take(n)
    .collect())
}

/// Given existing cards in a deal, randomly fill the remaining cards.
///
/// - `deal`: The initial deal with some cards already assigned.
/// - `n`: The number of deals to generate.
///
/// # Errors
///
///
/// [`dds_bridge::solver::SystemError`] if `deal` is invalid, such as
///
/// - Any hand having more than 13 cards.
/// - Duplicate cards across hands.
pub fn fill_n_deals(
    rng: &mut (impl Rng + ?Sized),
    deal: &Deal,
    n: usize,
) -> Result<Vec<Deal>, SystemError> {
    fill_n_filtered_deals(rng, deal, n, |_| true)
}
