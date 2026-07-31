//! Predictive census for BBA's Kickback trigger rule: self-play EPBot-vs-EPBot
//! (system 0, `Kickback 1430 = 1` both sides) over constrained random deals,
//! and for every 4♦/4♥/4♠/4NT call compare the engine's own systemic label
//! (`epbot_get_info_meaning`) against a predictor implementing the trigger
//! conditions read from the decompiled `get_kolor_kickback`, evaluated on the
//! same shown-length state the engine exposes (`epbot_get_info_min/max_length`).
//! Agreement means the extracted rule set predicts the engine; every mismatch
//! is printed for triage.  Deals are rejection-sampled so one side holds a
//! combined 24+ HCP and an eight-card fit (slam-ish density).
//!
//! Run from the repo root, main thread only (EPBot segfaults under `cargo test`):
//!
//! ```text
//! cargo run --release --example probe-bba-kickback-census -- [boards] [seed]
//! ```

use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;
type GetInfoFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_char, c_int) -> c_int;
type GetArrFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int, c_int, *mut c_int) -> c_int;

struct Engine {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    get_meaning: GetInfoFn,
    get_min_length: GetArrFn,
    get_max_length: GetArrFn,
    get_feature: GetArrFn,
}

impl Engine {
    fn load() -> anyhow::Result<Self> {
        let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
        // SAFETY: loading the trusted vendored engine; `_lib` outlives the
        // copied `Copy` function pointers.
        let lib = unsafe { Library::new(&path) }?;
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                set_conv: *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
                get_meaning: *lib.get::<GetInfoFn>(b"epbot_get_info_meaning\0")?,
                get_min_length: *lib.get::<GetArrFn>(b"epbot_get_info_min_length\0")?,
                get_max_length: *lib.get::<GetArrFn>(b"epbot_get_info_max_length\0")?,
                get_feature: *lib.get::<GetArrFn>(b"epbot_get_info_feature\0")?,
                _lib: lib,
            })
        }
    }
}

/// Decode EPBot's bid code (0/1/2 = Pass/X/XX; bid = 5 + (level-1)*5 + strain).
fn decode(code: c_int) -> String {
    const STRAIN: [&str; 5] = ["♣", "♦", "♥", "♠", "NT"];
    match code {
        0 => "P".into(),
        1 => "X".into(),
        2 => "XX".into(),
        5..=39 => {
            let i = code - 5;
            format!("{}{}", i / 5 + 1, STRAIN[(i % 5) as usize])
        }
        other => format!("?{other}"),
    }
}

// ponytail: xorshift64* instead of a rand dependency — census-grade shuffling.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Uniform in `0..n` (n ≤ 52, modulo bias negligible for a census).
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// One seat's holding as suit strings in ♠♥♦♣ order, plus HCP and suit lengths.
struct SeatHand {
    suits: [String; 4],
    hcp: i32,
    len: [i32; 4],
}

/// Deal four hands from a shuffled deck.  Card `c`: suit `c / 13` in ♣♦♥♠
/// order (EPBot strain order), rank `c % 13` descending from the ace.
fn deal(rng: &mut Rng) -> [SeatHand; 4] {
    const RANK: [char; 13] = [
        'A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2',
    ];
    let mut deck: [u8; 52] = std::array::from_fn(|i| i as u8);
    for i in (1..52).rev() {
        deck.swap(i, rng.below(i + 1));
    }
    std::array::from_fn(|seat| {
        let mut cards: Vec<u8> = deck[seat * 13..(seat + 1) * 13].to_vec();
        cards.sort_unstable();
        let mut suits = [const { String::new() }; 4];
        let mut hcp = 0;
        let mut len = [0; 4];
        for card in cards {
            let (strain, rank) = ((card / 13) as usize, (card % 13) as usize);
            // suits[] is ♠♥♦♣ for display; strain order ♣♦♥♠ reverses it.
            suits[3 - strain].push(RANK[rank]);
            hcp += 4 - rank.min(4) as i32;
            len[strain] += 1;
        }
        SeatHand { suits, hcp, len }
    })
}

/// EPBot's C\nD\nH\nS hand string from a ♠♥♦♣ suit array.
fn hand_string(suits: &[String; 4]) -> CString {
    CString::new(format!(
        "{}\n{}\n{}\n{}",
        suits[3], suits[2], suits[1], suits[0]
    ))
    .unwrap()
}

/// The predictor's inputs for one candidate call: both acting-side seats'
/// shown min lengths and the actor's shown max lengths, per strain ♣♦♥♠.
struct Shown {
    actor_min: [i32; 4],
    partner_min: [i32; 4],
    actor_max: [i32; 4],
}

/// The trigger conditions transcribed from `get_kolor_kickback` +
/// `get_kolor_domniemany`'s retro-agreement fallback (decompiled EPBot64):
/// does a 4-level bid in `strain` (♦/♥/♠) read as Kickback for `strain - 1`?
/// `partner_cue` is the engine's own mid-cue flag on the answerer
/// (`Item[partner].feature[144]`), the first arm of the cue suppression.
/// Still unmodeled (triaged per mismatch): the opponents'-suit and HCP gates,
/// splinter/forcing-1NT collisions, and interpretation-dispatch order
/// (natural slam-try / self-sufficient-suit readings that win first).
fn predict_kickback(shown: &Shown, strain: usize, partner_cue: bool) -> bool {
    let trump = strain - 1;
    let guard = shown.actor_min[strain] < 4 && shown.partner_min[strain] < 4;
    let sum = shown.actor_min[trump] + shown.partner_min[trump];
    let fit = sum >= 8 || (sum == 7 && shown.partner_min[trump] >= 2);
    let retro = (shown.actor_min[trump] >= 4 || shown.partner_min[trump] >= 4)
        && shown.partner_min[trump] + shown.actor_max[trump] >= 8;
    !partner_cue && guard && (fit || retro)
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let boards: usize = args.next().map_or(3000, |a| a.parse().expect("boards"));
    let seed: u64 = args.next().map_or(1, |a| a.parse().expect("seed"));
    let engine = Engine::load()?;
    let empty = c"".as_ptr();
    let kickback = CString::new("Kickback 1430").unwrap();
    let mut rng = Rng(seed.max(1));

    let mut auctions = 0_usize;
    let mut calls = 0_usize;
    // Per candidate strain ♦♥♠ + 4NT: [label says kickback][predictor says] counts.
    let mut confusion = [[0_usize; 2]; 2];
    let mut fired_by_trump = [0_usize; 4];
    let mut notrump_labels: Vec<(String, usize)> = Vec::new();
    let mut mismatches = Vec::new();

    for board in 0..boards {
        // Rejection-sample: some side holds 24+ combined HCP and an 8-card fit.
        let hands = loop {
            let hands = deal(&mut rng);
            let strong = |a: usize, b: usize| {
                hands[a].hcp + hands[b].hcp >= 24
                    && (0..4).any(|s| hands[a].len[s] + hands[b].len[s] >= 8)
            };
            if strong(0, 2) || strong(1, 3) {
                break hands;
            }
        };

        // SAFETY: four fresh bots for this deal, destroyed at the end; the
        // bidding ABI is the confirmed S.1 surface, the meaning/array getters
        // the recovered ones (buffer sizes in bytes).
        unsafe {
            let bots: [*mut c_void; 4] = std::array::from_fn(|_| (engine.create)());
            for &bot in &bots {
                assert!(!bot.is_null());
                for seat in 0..4 {
                    (engine.set_system)(bot, seat, 0);
                }
                for side in 0..2 {
                    (engine.set_conv)(bot, side, kickback.as_ptr(), 1);
                }
            }
            for (seat, bot) in bots.iter().enumerate() {
                let hand = hand_string(&hands[seat].suits);
                (engine.new_hand)(*bot, seat as c_int, hand.as_ptr(), 0, 0, 0, 0);
            }

            let read_meaning = |bot: *mut c_void, slot: c_int| {
                let mut buf = [0_u8; 1024];
                (engine.get_meaning)(bot, slot, buf.as_mut_ptr().cast::<c_char>(), 1024);
                let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
                String::from_utf8_lossy(&buf[..end]).into_owned()
            };
            let read_lengths = |get: GetArrFn, bot: *mut c_void, slot: c_int| {
                let mut buf = [0_i32; 16];
                let mut count: c_int = 0;
                let bytes = (buf.len() * size_of::<c_int>()) as c_int;
                (get)(bot, slot, buf.as_mut_ptr(), bytes, &mut count);
                [buf[0], buf[1], buf[2], buf[3]]
            };

            let mut history: Vec<c_int> = Vec::new();
            loop {
                let actor = history.len() % 4;
                let partner = (actor + 2) % 4;
                let code = (engine.get_bid)(bots[actor]);
                calls += 1;

                // Candidate 4♦/4♥/4♠: evaluate the predictor on the shown
                // state BEFORE the call lands, from the answerer's bot.
                let partner_cue = {
                    let mut buf = [0_i32; 512];
                    let mut count: c_int = 0;
                    let bytes = (buf.len() * size_of::<c_int>()) as c_int;
                    (engine.get_feature)(
                        bots[partner],
                        partner as c_int,
                        buf.as_mut_ptr(),
                        bytes,
                        &mut count,
                    );
                    buf[144] > 0
                };
                let predicted = (21..=23).contains(&code).then(|| {
                    let strain = (code - 5) as usize % 5;
                    let shown = Shown {
                        actor_min: read_lengths(
                            engine.get_min_length,
                            bots[partner],
                            actor as c_int,
                        ),
                        partner_min: read_lengths(
                            engine.get_min_length,
                            bots[partner],
                            partner as c_int,
                        ),
                        actor_max: read_lengths(
                            engine.get_max_length,
                            bots[partner],
                            actor as c_int,
                        ),
                    };
                    (strain, predict_kickback(&shown, strain, partner_cue))
                });

                for &bot in &bots {
                    (engine.set_bid)(bot, actor as c_int, code, empty);
                }
                history.push(code);

                // The answerer's interpretation decides the auction's fate.
                let label = read_meaning(bots[partner], actor as c_int);
                let is_kickback = label.contains("Kickback");
                if let Some((strain, predicted)) = predicted {
                    confusion[usize::from(is_kickback)][usize::from(predicted)] += 1;
                    if is_kickback {
                        fired_by_trump[strain - 1] += 1;
                    }
                    if is_kickback != predicted {
                        let auction: Vec<String> = history.iter().map(|&c| decode(c)).collect();
                        mismatches.push(format!(
                            "board {board} seat {actor}: {} labeled {label:?}, predicted {predicted} (partner_cue {partner_cue})\n  auction: {}\n  hand: ♠{} ♥{} ♦{} ♣{}",
                            decode(code),
                            auction.join(" "),
                            hands[actor].suits[0],
                            hands[actor].suits[1],
                            hands[actor].suits[2],
                            hands[actor].suits[3],
                        ));
                    }
                } else if code == 24 {
                    notrump_labels.push((label.clone(), board));
                }
                if is_kickback && code == 24 {
                    fired_by_trump[3] += 1;
                }

                let n = history.len();
                let opening_passes = n == 4 && history.iter().all(|&c| c == 0);
                let settled = n > 4 && history[n - 3..].iter().all(|&c| c == 0);
                if opening_passes || settled || n > 30 {
                    break;
                }
            }
            auctions += 1;
            for bot in bots {
                (engine.destroy)(bot);
            }
        }
    }

    println!("census: {auctions} auctions, {calls} calls, seed {seed}");
    println!(
        "4♦/4♥/4♠ candidates: {} labeled kickback vs {} not; fired by trump ♣/♦/♥ = {:?}, 4NT-labeled = {}",
        confusion[1][0] + confusion[1][1],
        confusion[0][0] + confusion[0][1],
        &fired_by_trump[..3],
        fired_by_trump[3],
    );
    println!(
        "predictor agreement: {} / {} ({} false-positive, {} false-negative)",
        confusion[0][0] + confusion[1][1],
        confusion.iter().flatten().sum::<usize>(),
        confusion[0][1],
        confusion[1][0],
    );
    let quantitative = notrump_labels
        .iter()
        .filter(|(l, _)| !l.contains("Blackwood"))
        .count();
    println!(
        "4NT calls: {} total, {} labeled Blackwood, {} other (first few: {:?})",
        notrump_labels.len(),
        notrump_labels.len() - quantitative,
        quantitative,
        notrump_labels
            .iter()
            .filter(|(l, _)| !l.contains("Blackwood"))
            .take(5)
            .collect::<Vec<_>>(),
    );
    println!("\n{} mismatches:", mismatches.len());
    for m in mismatches.iter().take(50) {
        println!("{m}\n");
    }
    if mismatches.len() > 50 {
        println!("... and {} more", mismatches.len() - 50);
    }
    Ok(())
}
