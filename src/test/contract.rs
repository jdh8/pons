use crate::contract::*;
use Penalty::*;
use rand::distributions::{Distribution, Standard};
use rand::Rng;

#[test]
fn made_contracts() {
    use Strain::*;

    assert_eq!(Contract::new(1, Clubs, Passed).score(9, false), 110);
    assert_eq!(Contract::new(1, Hearts, Passed).score(9, false), 140);
    assert_eq!(Contract::new(1, Notrump, Passed).score(7, false), 90);

    assert_eq!(Contract::new(3, Notrump, Passed).score(9, false), 400);
    assert_eq!(Contract::new(3, Notrump, Passed).score(9, true), 600);
    assert_eq!(Contract::new(4, Hearts, Passed).score(10, false), 420);
    assert_eq!(Contract::new(4, Spades, Passed).score(10, true), 620);
    assert_eq!(Contract::new(5, Clubs, Passed).score(11, false), 400);
    assert_eq!(Contract::new(5, Diamonds, Passed).score(11, true), 600);

    assert_eq!(Contract::new(6, Spades, Passed).score(12, true), 1430);
    assert_eq!(Contract::new(6, Notrump, Passed).score(12, false), 990);

    assert_eq!(Contract::new(2, Clubs, Doubled).score(8, false), 180);
    assert_eq!(Contract::new(2, Clubs, Doubled).score(9, false), 280);
    assert_eq!(Contract::new(2, Clubs, Doubled).score(9, true), 380);

    assert_eq!(Contract::new(1, Notrump, Redoubled).score(8, true), 1160);
    assert_eq!(Contract::new(7, Spades, Redoubled).score(13, false), 2240);
}

impl Distribution<Strain> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Strain {
        unsafe { core::mem::transmute(rng.gen_range(0..5) as u8) }
    }
}

#[test]
fn set_contracts() {
    let level = rand::thread_rng().gen_range(1..=7);
    let undoubled = Contract::new(level, rand::random(), Passed);

    for tricks in 0..level + 6 {
        let undertricks = (level + 6 - tricks) as i32;
        assert_eq!(undoubled.score(tricks, false), -50 * undertricks);
        assert_eq!(undoubled.score(tricks, true), -100 * undertricks);
    }

    let doubled = Contract::new(level, rand::random(), Doubled);

    assert_eq!(doubled.score(level + 5, false), -100);
    assert_eq!(doubled.score(level + 4, false), -300);
    assert_eq!(doubled.score(level + 3, false), -500);
    assert_eq!(doubled.score(level + 2, false), -800);
    
    assert_eq!(doubled.score(level + 5, true), -200);
    assert_eq!(doubled.score(level + 4, true), -500);
    assert_eq!(doubled.score(level + 3, true), -800);
    assert_eq!(doubled.score(level + 2, true), -1100);

    let redoubled = Contract::new(level, rand::random(), Redoubled);

    for tricks in 0..level + 6 {
        assert_eq!(redoubled.score(tricks, false), 2 * doubled.score(tricks, false));
        assert_eq!(redoubled.score(tricks, true), 2 * doubled.score(tricks, true));
    }
}