use dds_bridge::{Builder, Card, Hand, Rank, Seat, PartialDeal, Suit};
use pons::deck::fill_deals;
use pons::{Deck, full_deal};

#[test]
fn test_deck_empty() {
    let deck = Deck::EMPTY;
    assert_eq!(deck.len(), 0);
    assert!(deck.is_empty());
}

#[test]
fn test_deck_default() {
    let deck = Deck::default();
    assert!(deck.is_empty());
}

#[test]
fn test_deck_all() {
    let deck = Deck::ALL;
    assert_eq!(deck.len(), 52);
    assert!(!deck.is_empty());
}

#[test]
fn test_deck_all_unique() {
    let mut deck = Deck::ALL;
    let hand = deck.take();
    assert_eq!(hand.len(), 52);
}

#[test]
fn test_deck_insert() {
    let mut deck = Deck::EMPTY;
    let card = Card {
        suit: Suit::Clubs,
        rank: Rank::A,
    };
    assert!(deck.insert(card));
    assert_eq!(deck.len(), 1);
}

#[test]
fn test_deck_insert_duplicate() {
    let mut deck = Deck::EMPTY;
    let card = Card {
        suit: Suit::Clubs,
        rank: Rank::A,
    };
    assert!(deck.insert(card));
    assert!(!deck.insert(card));
    assert_eq!(deck.len(), 1);
}

#[test]
fn test_deck_clear() {
    let mut deck = Deck::ALL;
    deck.clear();
    assert!(deck.is_empty());
}

#[test]
fn test_deck_take() {
    let mut deck = Deck::EMPTY;
    deck.insert(Card {
        suit: Suit::Clubs,
        rank: Rank::A,
    });
    deck.insert(Card {
        suit: Suit::Diamonds,
        rank: Rank::K,
    });
    let hand = deck.take();
    assert_eq!(hand.len(), 2);
    assert!(deck.is_empty());
}

#[test]
fn test_deck_draw() {
    let mut deck = Deck::ALL;
    let rng = &mut rand::rng();
    let hand = deck.draw(rng, 13);
    assert_eq!(hand.len(), 13);
    assert_eq!(deck.len(), 39);
}

#[test]
fn test_deck_draw_all() {
    let mut deck = Deck::ALL;
    let rng = &mut rand::rng();
    let hand = deck.draw(rng, 100);
    assert_eq!(hand.len(), 52);
    assert!(deck.is_empty());
}

#[test]
fn test_deck_pop() {
    let mut deck = Deck::ALL;
    let rng = &mut rand::rng();
    let card = deck.pop(rng);
    assert!(card.is_some());
    assert_eq!(deck.len(), 51);
}

#[test]
fn test_deck_pop_empty() {
    let mut deck = Deck::EMPTY;
    let rng = &mut rand::rng();
    assert!(deck.pop(rng).is_none());
}

#[test]
fn test_deck_pop_singleton() {
    let mut deck = Deck::EMPTY;
    deck.insert(Card {
        suit: Suit::Clubs,
        rank: Rank::A,
    });
    let rng = &mut rand::rng();
    let card = deck.pop(rng);
    assert_eq!(
        card,
        Some(Card {
            suit: Suit::Clubs,
            rank: Rank::A
        })
    );
    assert!(deck.is_empty());
}

#[test]
fn test_deck_from_hand() {
    let hand: Hand = [
        Card {
            suit: Suit::Clubs,
            rank: Rank::A,
        },
        Card {
            suit: Suit::Hearts,
            rank: Rank::K,
        },
    ]
    .into_iter()
    .collect();
    let deck = Deck::from(hand);
    assert_eq!(deck.len(), 2);
}

#[test]
fn test_full_deal_has_52_cards() {
    let rng = &mut rand::rng();
    let deal = full_deal(rng);
    let total: usize = Seat::ALL.iter().map(|&s| deal[s].len()).sum();
    assert_eq!(total, 52);
}

#[test]
fn test_full_deal_each_hand_13_cards() {
    let rng = &mut rand::rng();
    let deal = full_deal(rng);
    for seat in Seat::ALL {
        assert_eq!(deal[seat].len(), 13);
    }
}

#[test]
fn test_full_deal_all_unique() {
    let rng = &mut rand::rng();
    let deal = full_deal(rng);
    let all: Hand = Seat::ALL
        .iter()
        .fold(Hand::default(), |acc, &s| acc | deal[s]);
    assert_eq!(all.len(), 52);
}

#[test]
fn test_fill_deals_from_full_deal() {
    let rng = &mut rand::rng();
    let deal = full_deal(rng);
    let filled = fill_deals(rng, deal.into()).next().unwrap();
    assert_eq!(filled, deal);
}

#[test]
fn test_fill_deals_from_empty() {
    let rng = &mut rand::rng();
    let filled = fill_deals(rng, PartialDeal::EMPTY).next().unwrap();
    for seat in Seat::ALL {
        assert_eq!(filled[seat].len(), 13);
    }
    let all: Hand = Seat::ALL
        .iter()
        .fold(Hand::default(), |acc, &s| acc | filled[s]);
    assert_eq!(all.len(), 52);
}

#[test]
fn test_fill_deals_preserves_known_cards() {
    let rng = &mut rand::rng();
    let north: Hand = [
        Card {
            suit: Suit::Spades,
            rank: Rank::A,
        },
        Card {
            suit: Suit::Spades,
            rank: Rank::K,
        },
    ]
    .into_iter()
    .collect();
    let mut builder = Builder::default();
    builder[Seat::North] = north;
    let subset = builder.build_partial().unwrap();
    let filled = fill_deals(rng, subset).next().unwrap();
    assert!(filled[Seat::North].into_iter().any(|c| c
        == Card {
            suit: Suit::Spades,
            rank: Rank::A
        }));
    assert!(filled[Seat::North].into_iter().any(|c| c
        == Card {
            suit: Suit::Spades,
            rank: Rank::K
        }));
}

#[test]
fn test_fill_deals_iterator_is_infinite() {
    let rng = &mut rand::rng();
    let deals: Vec<_> = fill_deals(rng, PartialDeal::EMPTY).take(5).collect();
    assert_eq!(deals.len(), 5);
}
