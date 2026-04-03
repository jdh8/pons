use core::num::Wrapping;
use dds_bridge::contract::{Bid, Contract, Penalty, Strain};
use dds_bridge::deal::{Deal, Hand, Seat, SmallSet as _};
use dds_bridge::deck::Deck;
use dds_bridge::solver::{self, Error, StrainFlags, SystemError, Vulnerability};
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

/// Calculate average NS par score from the provided deals.
///
/// This idea is inspired by [Cuebids](https://cuebids.com/).
///
/// # Errors
///
/// A [`dds_bridge::solver::SystemError`] propagated from DDS or a
/// [`std::sync::PoisonError`]
pub fn average_ns_par(
    deals: &[Deal],
    vul: Vulnerability,
    dealer: Seat,
    n: usize,
) -> Result<(f64, Option<(Contract, Seat)>), Error> {
    const BID_VARIANTS: usize = 7 * 5;

    /// Encode a bid to an array index
    const fn encode_bid(bid: Bid) -> usize {
        (bid.level as usize - 1) * 5 + bid.strain as usize
    }

    /// Decode an array index back to the bid
    #[allow(clippy::cast_possible_truncation)]
    const fn decode_bid(code: usize) -> Bid {
        let level = (code / 5) as u8 + 1;
        let strain = Strain::ASC[code % 5];
        Bid { level, strain }
    }

    // Check at compile time that `encode_bid` and `decode_bid` cancel each other
    const _: () = {
        let mut code = 0;

        while code < BID_VARIANTS {
            let bid = decode_bid(code);
            assert!(code == encode_bid(bid));
            code += 1;
        }
    };

    // seat -> strain -> tricks -> frequency
    let histogram = solver::solve_deals(deals, StrainFlags::all())?
        .into_iter()
        .fold([[[0usize; 14]; 5]; 4], |mut hist, tricks| {
            for seat in Seat::ALL {
                for strain in Strain::ASC {
                    hist[seat as usize][strain as usize][usize::from(tricks[strain].get(seat))] +=
                        1;
                }
            }
            hist
        });

    // seat -> bid -> (score, contract)
    let scores = Seat::ALL.map(|seat| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        const fn score(contract: Contract, hist: [usize; 14], vul: bool) -> i64 {
            let mut sum = 0;
            let mut tricks = 0;

            while tricks <= 13 {
                sum += (hist[tricks] as i64) * contract.score(tricks as u8, vul) as i64;
                tricks += 1;
            }
            sum
        }

        let side = match seat {
            Seat::North | Seat::South => Vulnerability::NS,
            Seat::East | Seat::West => Vulnerability::EW,
        };

        let mut table: [_; BID_VARIANTS] = core::array::from_fn(|bid| {
            let bid = decode_bid(bid);
            let normal = Contract {
                bid,
                penalty: Penalty::None,
            };
            let doubled = Contract {
                bid,
                penalty: Penalty::Doubled,
            };
            let hist = histogram[seat as usize][bid.strain as usize];
            let normal = (score(normal, hist, vul.contains(side)), normal);
            let doubled = (score(doubled, hist, vul.contains(side)), doubled);
            normal.min(doubled)
        });

        for bid in (0..BID_VARIANTS - 1).rev() {
            table[bid] = table[bid].max(table[bid + 1]);
        }

        match seat {
            Seat::North | Seat::South => table,
            Seat::East | Seat::West => table.map(|(score, contract)| (-score, contract)),
        }
    });

    let mut par_score = 0;
    let mut par_contract: Option<(Contract, Seat)> = None;
    let mut improve_for = |seat: Seat| {
        let bid = par_contract.map_or(0, |(contract, _)| encode_bid(contract.bid));
        let (score, contract) = scores[seat as usize][bid];
        let is_improved = match seat {
            Seat::North | Seat::South => score > par_score,
            Seat::East | Seat::West => score < par_score,
        };
        if is_improved {
            par_score = score;
            par_contract.replace((contract, seat));
        }
    };
    improve_for(dealer);
    improve_for(dealer - Wrapping(1));
    improve_for(dealer - Wrapping(2));
    improve_for(dealer - Wrapping(3));
    improve_for(dealer);

    #[allow(clippy::cast_precision_loss)]
    Ok((par_score as f64 / n as f64, par_contract))
}
