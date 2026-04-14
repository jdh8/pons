use dds_bridge::{Bid, Level, Penalty, Strain};
use pons::bidding::array::Logits;
use pons::bidding::{Array, Auction, Call, IllegalCall, Map, RelativeVulnerability};

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn one_club() -> Call {
    bid(1, Strain::ASC[0])
}

fn seven_nt() -> Call {
    bid(7, Strain::ASC[4])
}

// ===== Auction =====

#[test]
fn test_auction_new_is_empty() {
    let auction = Auction::new();
    assert!(auction.is_empty());
    assert!(!auction.has_ended());
}

#[test]
fn test_auction_pass_out() {
    let mut auction = Auction::new();
    for _ in 0..4 {
        auction.push(Call::Pass);
    }
    assert!(auction.has_ended());
    assert_eq!(auction.declarer(), None);
}

#[test]
fn test_auction_three_passes_not_ended() {
    let mut auction = Auction::new();
    for _ in 0..3 {
        auction.push(Call::Pass);
    }
    assert!(!auction.has_ended());
}

#[test]
fn test_auction_simple_bid_sequence() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    assert!(auction.has_ended());
    assert_eq!(auction.declarer(), Some(0));
}

#[test]
fn test_auction_declarer_same_strain_partner() {
    // Dealer bids 1C, partner raises to 2C -> declarer is dealer (index 0)
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs)); // index 0 (dealer)
    auction.push(Call::Pass); // index 1
    auction.push(bid(2, Strain::Clubs)); // index 2 (partner)
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    assert_eq!(auction.declarer(), Some(0));
}

#[test]
fn test_auction_declarer_different_strain() {
    // Pass, 1H, Pass, 2H -> declarer is index 1 (first to bid hearts)
    let mut auction = Auction::new();
    auction.push(Call::Pass); // index 0
    auction.push(bid(1, Strain::Hearts)); // index 1
    auction.push(Call::Pass); // index 2
    auction.push(bid(2, Strain::Hearts)); // index 3
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    assert_eq!(auction.declarer(), Some(1));
}

#[test]
fn test_auction_double() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    auction.try_push(Call::Double).unwrap();
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    assert!(auction.has_ended());
}

#[test]
fn test_auction_redouble() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    auction.push(Call::Double);
    auction.try_push(Call::Redouble).unwrap();
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    assert!(auction.has_ended());
}

#[test]
fn test_auction_insufficient_bid() {
    let mut auction = Auction::new();
    auction.push(bid(2, Strain::Clubs));
    let result = auction.try_push(bid(1, Strain::Hearts));
    assert!(matches!(result, Err(IllegalCall::InsufficientBid { .. })));
}

#[test]
fn test_auction_equal_bid_is_insufficient() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    let result = auction.try_push(bid(1, Strain::Clubs));
    assert!(matches!(result, Err(IllegalCall::InsufficientBid { .. })));
}

#[test]
fn test_auction_double_own_bid_is_inadmissible() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    auction.push(Call::Pass);
    let result = auction.try_push(Call::Double);
    assert!(matches!(
        result,
        Err(IllegalCall::InadmissibleDouble(Penalty::Doubled))
    ));
}

#[test]
fn test_auction_double_without_bid() {
    let mut auction = Auction::new();
    let result = auction.try_push(Call::Double);
    assert!(matches!(
        result,
        Err(IllegalCall::InadmissibleDouble(Penalty::Doubled))
    ));
}

#[test]
fn test_auction_redouble_without_double() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    let result = auction.try_push(Call::Redouble);
    assert!(matches!(
        result,
        Err(IllegalCall::InadmissibleDouble(Penalty::Redoubled))
    ));
}

#[test]
fn test_auction_call_after_final_pass() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    auction.push(Call::Pass);
    let result = auction.try_push(Call::Pass);
    assert_eq!(result, Err(IllegalCall::AfterFinalPass));
}

#[test]
fn test_auction_pop() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    assert_eq!(auction.pop(), Some(bid(1, Strain::Clubs)));
    assert!(auction.is_empty());
    assert_eq!(auction.pop(), None);
}

#[test]
fn test_auction_truncate() {
    let mut auction = Auction::new();
    auction.push(bid(1, Strain::Clubs));
    auction.push(Call::Pass);
    auction.push(bid(2, Strain::Clubs));
    auction.truncate(1);
    assert_eq!(auction.len(), 1);
}

#[test]
fn test_auction_try_extend() {
    let mut auction = Auction::new();
    auction
        .try_extend([bid(1, Strain::Clubs), Call::Pass, Call::Pass, Call::Pass])
        .unwrap();
    assert!(auction.has_ended());
}

#[test]
fn test_auction_try_extend_partial_failure() {
    let mut auction = Auction::new();
    let result = auction.try_extend([
        bid(2, Strain::Clubs),
        bid(1, Strain::Hearts), // insufficient
    ]);
    assert!(result.is_err());
    // The first bid should still be in the auction
    assert_eq!(auction.len(), 1);
}

#[test]
fn test_auction_into_vec() {
    let mut auction = Auction::new();
    auction.push(Call::Pass);
    auction.push(bid(1, Strain::Clubs));
    let v: Vec<Call> = auction.into();
    assert_eq!(v, vec![Call::Pass, bid(1, Strain::Clubs)]);
}

#[test]
fn test_auction_into_iter() {
    let mut auction = Auction::new();
    auction.push(Call::Pass);
    auction.push(bid(1, Strain::Clubs));
    let calls: Vec<_> = auction.into_iter().collect();
    assert_eq!(calls, vec![Call::Pass, bid(1, Strain::Clubs)]);
}

#[test]
fn test_call_from_bid() {
    let b = Bid {
        level: Level::new(1),
        strain: Strain::Clubs,
    };
    let call: Call = b.into();
    assert_eq!(call, Call::Bid(b));
}

#[test]
fn test_relative_vulnerability_constants() {
    assert_eq!(RelativeVulnerability::NONE, RelativeVulnerability::empty());
    assert_eq!(RelativeVulnerability::ALL, RelativeVulnerability::all());
    assert!(RelativeVulnerability::ALL.contains(RelativeVulnerability::WE));
    assert!(RelativeVulnerability::ALL.contains(RelativeVulnerability::THEY));
}

// ===== Array =====

#[test]
fn test_array_from_fn_and_get() {
    let arr = Array::from_fn(|call| match call {
        Call::Pass => 42,
        _ => 0,
    });
    assert_eq!(*arr.get(Call::Pass), 42);
    assert_eq!(*arr.get(Call::Double), 0);
    assert_eq!(*arr.get(one_club()), 0);
}

#[test]
fn test_array_get_mut() {
    let mut arr = Array::from_fn(|_| 0);
    *arr.get_mut(Call::Pass) = 99;
    assert_eq!(*arr.get(Call::Pass), 99);
}

#[test]
fn test_array_index() {
    let arr = Array::from_fn(|call| match call {
        Call::Double => 7,
        _ => 0,
    });
    assert_eq!(arr[Call::Double], 7);
}

#[test]
fn test_array_index_mut() {
    let mut arr = Array::from_fn(|_| 0);
    arr[Call::Redouble] = 5;
    assert_eq!(arr[Call::Redouble], 5);
}

#[test]
fn test_array_repeat() {
    let arr = Array::repeat(3);
    for (_, &v) in &arr {
        assert_eq!(v, 3);
    }
}

#[test]
fn test_array_map() {
    let arr: Array<i32> = Array::from_fn(|_| 1);
    let doubled = arr.map(|_, v| v * 2);
    for (_, &v) in &doubled {
        assert_eq!(v, 2);
    }
}

#[test]
fn test_array_try_map_ok() {
    let arr = Array::from_fn(|_| 0i32);
    let result: Result<Array<i32>, &str> = arr.try_map(|_, v| Ok(v + 1));
    assert!(result.is_ok());
}

#[test]
fn test_array_try_map_err() {
    let arr = Array::from_fn(|call| match call {
        Call::Pass => 10,
        _ => 0,
    });
    let result: Result<Array<i32>, &str> =
        arr.try_map(|_, v| if v > 5 { Err("too big") } else { Ok(v) });
    assert_eq!(result, Err("too big"));
}

#[test]
fn test_array_iter_count() {
    let arr = Array::from_fn(|_| 0u8);
    // Pass, Double, Redouble + 7 levels × 5 strains
    assert_eq!(arr.iter().count(), 3 + 7 * 5);
}

#[test]
fn test_array_values_count() {
    let arr = Array::from_fn(|_| 0u8);
    assert_eq!(arr.values().count(), 3 + 7 * 5);
}

#[test]
fn test_array_into_values_count() {
    let arr = Array::from_fn(|_| 0u8);
    assert_eq!(arr.into_values().count(), 3 + 7 * 5);
}

#[test]
fn test_array_each_ref() {
    let arr = Array::from_fn(|call| match call {
        Call::Pass => 0,
        _ => 1,
    });
    let refs = arr.each_ref();
    assert_eq!(*refs.get(Call::Pass), &0);
}

#[test]
fn test_array_each_mut() {
    let mut arr = Array::from_fn(|_| 0);
    let mut mutables = arr.each_mut();
    **mutables.get_mut(Call::Pass) = 42;
    drop(mutables);
    assert_eq!(arr[Call::Pass], 42);
}

#[test]
fn test_array_range_index() {
    let arr = Array::from_fn(|_| 0u8);
    let slice = &arr[Call::Pass..Call::Redouble];
    assert_eq!(slice.len(), 2); // Pass, Double
}

#[test]
fn test_array_range_from_index() {
    let arr = Array::from_fn(|_| 0u8);
    let slice = &arr[seven_nt()..];
    assert_eq!(slice.len(), 1);
}

#[test]
fn test_array_range_inclusive_index() {
    let arr = Array::from_fn(|_| 0u8);
    let slice = &arr[Call::Pass..=Call::Redouble];
    assert_eq!(slice.len(), 3);
}

#[test]
fn test_array_full_range_index() {
    let arr = Array::from_fn(|_| 0u8);
    assert_eq!(arr[..].len(), 3 + 7 * 5);
}

#[test]
fn test_array_default() {
    let arr: Array<i32> = Array::default();
    for (_, &v) in &arr {
        assert_eq!(v, 0);
    }
}

#[test]
fn test_array_option_new() {
    let arr: Array<Option<i32>> = Array::new();
    for (_, v) in &arr {
        assert!(v.is_none());
    }
}

#[test]
fn test_logits_new() {
    let logits = Logits::new();
    for &v in logits.values() {
        assert_eq!(v, f32::NEG_INFINITY);
    }
}

#[test]
fn test_logits_default() {
    let logits = Logits::default();
    assert_eq!(logits, Logits::new());
}

#[test]
fn test_logits_softmax_uniform() {
    // All NEG_INFINITY -> uniform distribution
    let logits = Logits::new();
    let probs = logits.softmax();
    let first = probs[Call::Pass];
    for (_, &p) in &probs {
        assert!((p - first).abs() < 1e-6);
    }
    let sum: f32 = probs.values().copied().sum();
    assert!((sum - 1.0).abs() < 1e-5);
}

#[test]
fn test_logits_softmax_one_hot() {
    // Only Pass has logit 0, rest NEG_INFINITY -> Pass gets ~1.0
    let mut logits = Logits::new();
    *logits.get_mut(Call::Pass) = 0.0;
    let probs = logits.softmax();
    assert!((probs[Call::Pass] - 1.0).abs() < 1e-6);
    assert!(probs[Call::Double].abs() < 1e-6);
}

#[test]
fn test_logits_softmax_equal() {
    // All logits equal -> uniform
    let logits = Logits(Array::repeat(5.0f32));
    let probs = logits.softmax();
    let first = probs[Call::Pass];
    for (_, &p) in &probs {
        assert!((p - first).abs() < 1e-6);
    }
    let sum: f32 = probs.values().copied().sum();
    assert!((sum - 1.0).abs() < 1e-5);
}

// ===== Map =====

#[test]
fn test_map_new_is_empty() {
    let map: Map<i32> = Map::new();
    assert!(map.get(Call::Pass).is_none());
    assert!(map.get(Call::Double).is_none());
    assert!(map.get(one_club()).is_none());
}

#[test]
fn test_map_default() {
    let map: Map<i32> = Map::default();
    assert!(map.get(Call::Pass).is_none());
}

#[test]
fn test_map_insert_and_get() {
    let mut map = Map::new();
    assert!(map.insert(Call::Pass, 42).is_none());
    assert_eq!(map.get(Call::Pass), Some(&42));
}

#[test]
fn test_map_insert_replace() {
    let mut map = Map::new();
    map.insert(Call::Pass, 42);
    let old = map.insert(Call::Pass, 99);
    assert_eq!(old, Some(42));
    assert_eq!(map.get(Call::Pass), Some(&99));
}

#[test]
fn test_map_entry() {
    let mut map: Map<i32> = Map::new();
    *map.entry(Call::Double) = Some(7);
    assert_eq!(map.get(Call::Double), Some(&7));
}

#[test]
fn test_map_keys() {
    let mut map = Map::new();
    map.insert(Call::Pass, 1);
    map.insert(Call::Double, 2);
    let keys: Vec<_> = map.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&Call::Pass));
    assert!(keys.contains(&Call::Double));
}

#[test]
fn test_map_values() {
    let mut map = Map::new();
    map.insert(Call::Pass, 10);
    map.insert(Call::Double, 20);
    let mut values: Vec<_> = map.values().copied().collect();
    values.sort();
    assert_eq!(values, vec![10, 20]);
}

#[test]
fn test_map_values_mut() {
    let mut map = Map::new();
    map.insert(Call::Pass, 10);
    for v in map.values_mut() {
        *v += 1;
    }
    assert_eq!(map.get(Call::Pass), Some(&11));
}

#[test]
fn test_map_into_values() {
    let mut map = Map::new();
    map.insert(Call::Pass, 10);
    map.insert(Call::Double, 20);
    let mut values: Vec<_> = map.into_values().collect();
    values.sort();
    assert_eq!(values, vec![10, 20]);
}

#[test]
fn test_map_from_iterator() {
    let map: Map<i32> = [(Call::Pass, 1), (Call::Double, 2)].into_iter().collect();
    assert_eq!(map.get(Call::Pass), Some(&1));
    assert_eq!(map.get(Call::Double), Some(&2));
}

#[test]
fn test_map_extend() {
    let mut map = Map::new();
    map.extend([(Call::Pass, 1), (Call::Double, 2)]);
    assert_eq!(map.get(Call::Pass), Some(&1));
    assert_eq!(map.get(Call::Double), Some(&2));
}

#[test]
fn test_map_extend_ref() {
    let mut map: Map<i32> = Map::new();
    map.extend([(&Call::Pass, &1)]);
    assert_eq!(map.get(Call::Pass), Some(&1));
}

#[test]
fn test_map_iter() {
    let mut map = Map::new();
    map.insert(Call::Pass, 1);
    let pairs: Vec<_> = map.iter().collect();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], (Call::Pass, &1));
}

#[test]
fn test_map_iter_mut() {
    let mut map = Map::new();
    map.insert(Call::Pass, 5);
    for (_, v) in map.iter_mut() {
        *v *= 2;
    }
    assert_eq!(map.get(Call::Pass), Some(&10));
}

#[test]
fn test_map_into_iter() {
    let mut map = Map::new();
    map.insert(Call::Pass, 42);
    let pairs: Vec<_> = map.into_iter().collect();
    assert_eq!(pairs, vec![(Call::Pass, 42)]);
}

#[test]
fn test_map_from_array() {
    let arr = Array::from_fn(|_| 0i32);
    let map = Map::from(arr);
    assert_eq!(map.get(Call::Pass), Some(&0));
}

#[test]
fn test_map_try_into_array_ok() {
    let arr = Array::from_fn(|_| 0i32);
    let map = Map::from(arr);
    let result: Result<Array<i32>, Call> = map.try_into();
    assert!(result.is_ok());
}

#[test]
fn test_map_try_into_array_err() {
    let mut map: Map<i32> = Map::new();
    map.insert(Call::Pass, 1);
    // Missing many entries -> should fail
    let result: Result<Array<i32>, Call> = map.try_into();
    assert!(result.is_err());
}

#[test]
fn test_map_unwrap_or_default() {
    let mut map: Map<i32> = Map::new();
    map.insert(Call::Pass, 42);
    let arr = map.unwrap_or_default();
    assert_eq!(arr[Call::Pass], 42);
    assert_eq!(arr[Call::Double], 0);
}

#[test]
fn test_map_unwrap_or_else() {
    let mut map: Map<i32> = Map::new();
    map.insert(Call::Pass, 42);
    let arr = map.unwrap_or_else(|_| -1);
    assert_eq!(arr[Call::Pass], 42);
    assert_eq!(arr[Call::Double], -1);
}
