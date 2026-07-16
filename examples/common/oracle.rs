//! Driving external bidding oracles as pons [`System`]s.
//!
//! [`BbaOracle`] wraps EPBot's C ABI (`libEPBot.so`, no Wine) — the BBA
//! reference bidder used by `bba-gen` and by `ben-gen --calibrate-epbot`.
//! [`next_call`]/[`bid_out`] are the `&dyn System` match drivers shared by
//! every oracle-vs-oracle generator (the `Stance`-based variants in
//! [`mod.rs`](super) serve the self-play harnesses instead).
//!
//! Moved verbatim out of `examples/bba-gen/main.rs`; the S.0 spike documents
//! the ABI discovery.

use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Level, Seat, Strain, Suit};
use libloading::Library;
use pons::bidding::System;
use pons::bidding::array::{Array, Logits};
use pons::bidding::context::relative;
use std::ffi::{CString, c_char, c_int, c_void};

use super::seat_to_act;

pub const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

/// System index 0 = "2/1GF - 2/1 Game Force" (verified via `epbot_system_name`)
pub const SYSTEM_2_OVER_1: c_int = 0;

// Confirmed C ABI (objdump + EPBotFFI decompile + empirical bid codes); the
// S.0 spike documents the discovery.  Handles are opaque pointers.
type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
// `epbot_set_bid(bot, position, bid, meaning)` — the 4th arg is the bid's
// meaning string (decompiled from EPBotFFI.SetBid); an empty string is fine,
// EPBot interprets each bid itself from the configured system.
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
// `epbot_set_conventions(bot, seat, name, on)` — per-seat convention toggle.
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;

/// EPBot 2/1 bidder behind pons's [`System`] trait.
///
/// Each [`System::classify`] call drives a *fresh* bot: it configures all four
/// seats to the chosen system, deals the actor's hand, replays the auction so
/// far with `set_bid`, and reads the actor's call with `get_bid`.  A fresh bot
/// per decision keeps `classify` a pure, stateless function of its arguments.
///
/// Cached raw function pointers (copied out of the [`Library`]) avoid a `dlsym`
/// per call; `_lib` is held so the pointers stay valid for the oracle's life.
pub struct BbaOracle {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    system: c_int,
    /// Named conventions forced to a value on all four seats of every fresh bot,
    /// applied after `set_system` (which loads the system's defaults).
    overrides: Vec<(CString, c_int)>,
}

impl BbaOracle {
    /// Load the EPBot library and bind the `epbot_*` symbols
    pub fn load(
        path: &str,
        system: c_int,
        overrides: Vec<(CString, c_int)>,
    ) -> anyhow::Result<Self> {
        // SAFETY: loading a trusted native library; its initializers run here.
        let lib = unsafe { Library::new(path) }?;
        // SAFETY: each symbol has the signature confirmed in the S.0 spike;
        // `*sym` copies the function pointer (it is `Copy` and does not borrow
        // the library, which we keep alive in `_lib`).
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                set_conv: *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
                _lib: lib,
                system,
                overrides,
            })
        }
    }
}

/// A BBA `.bbsa` convention card: the EPBot system id from its `System type`
/// header plus every remaining line as a convention toggle.
pub struct ConventionCard {
    pub system: c_int,
    pub toggles: Vec<(CString, c_int)>,
}

/// Load a `.bbsa` convention-card file (e.g. BEN's declared card
/// `vendor/ben/BEN-21GF.bbsa`) into overrides for [`BbaOracle::load`].
///
/// Mirrors BEN's own loader (`src/bba/BBA.py::load_ccs`): line 0
/// `System type = N` selects the system; every other `Name = value` line —
/// including the `Opponent type` / `Not defined` meta rows — is passed to
/// `epbot_set_conventions` verbatim.
pub fn load_bbsa(path: &str) -> anyhow::Result<ConventionCard> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("reading card `{path}`: {error}"))?;
    let mut system = None;
    let mut toggles = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .rsplit_once(" = ")
            .ok_or_else(|| anyhow::anyhow!("{path}:{}: expected `Name = value`", index + 1))?;
        let value: c_int = value
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("{path}:{}: non-integer value", index + 1))?;
        if index == 0 && name == "System type" {
            system = Some(value);
        } else {
            let name = CString::new(name)
                .map_err(|_| anyhow::anyhow!("{path}:{}: name has an interior NUL", index + 1))?;
            toggles.push((name, value));
        }
    }
    let system = system
        .ok_or_else(|| anyhow::anyhow!("{path}: missing the `System type = N` header line"))?;
    Ok(ConventionCard { system, toggles })
}

impl System for BbaOracle {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        // Canonicalize the dealer to position 0: the actor is the seat that has
        // bid `auction.len()` times after the dealer.  The relative seat and the
        // favorable/unfavorable vulnerability are preserved by the replayed calls
        // and the mapping below, so the bid is identical to the true seating.
        let actor = (auction.len() % 4) as c_int;
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();

        // SAFETY: a fresh bot used and destroyed within this call; all argument
        // types match the confirmed ABI.
        let code = unsafe {
            let bot = (self.create)();
            if bot.is_null() {
                return None;
            }
            for seat in 0..4 {
                (self.set_system)(bot, seat, self.system);
            }
            // Force any isolated convention(s) AFTER set_system loads defaults.
            for (name, value) in &self.overrides {
                for seat in 0..4 {
                    (self.set_conv)(bot, seat, name.as_ptr(), *value);
                }
            }
            (self.new_hand)(
                bot,
                actor,
                suits.as_ptr(),
                0,
                epbot_vulnerability(vul, actor),
                0,
                0,
            );
            for (index, &call) in auction.iter().enumerate() {
                (self.set_bid)(bot, (index % 4) as c_int, encode_call(call), empty);
            }
            let code = (self.get_bid)(bot);
            (self.destroy)(bot);
            code
        };

        decode_call(code).map(one_hot)
    }
}

/// The four holdings in EPBot's C,D,H,S order, newline-joined
///
/// [`Holding`][contract_bridge::Holding]'s `Display` renders ranks high-to-low
/// using `T` for the ten — exactly EPBot's canonical form.  EPBot counts
/// characters as cards, so every hand must be exactly 13; `full_deal` guarantees
/// that.  A void suit is an empty segment, which EPBot reads as zero cards.
fn hand_to_suits(hand: Hand) -> CString {
    use core::fmt::Write;
    let mut suits = String::with_capacity(20);
    for (index, suit) in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            suits.push('\n');
        }
        write!(suits, "{}", hand[suit]).expect("writing to a String never fails");
    }
    CString::new(suits).expect("a holding string never contains a NUL byte")
}

/// EPBot vulnerability code from the actor-relative vulnerability
///
/// EPBot seats even (0, 2) are North/South and odd (1, 3) East/West; its
/// vulnerability bits are 1 = N/S, 2 = E/W.  With the dealer canonicalized to
/// position 0, the actor's side is N/S iff `actor` is even.
fn epbot_vulnerability(vul: RelativeVulnerability, actor: c_int) -> c_int {
    let we = vul.contains(RelativeVulnerability::WE);
    let they = vul.contains(RelativeVulnerability::THEY);
    let (ns, ew) = if actor % 2 == 0 {
        (we, they)
    } else {
        (they, we)
    };
    c_int::from(ns) | (c_int::from(ew) << 1)
}

/// Encode a [`Call`] into EPBot's integer bid code
///
/// `0/1/2 = Pass/X/XX`; a bid is `5 + (level - 1) * 5 + strain` with strain
/// `0 = ♣ … 4 = NT`, matching [`Strain`]'s discriminant order.
fn encode_call(call: Call) -> c_int {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => 5 + (c_int::from(bid.level.get()) - 1) * 5 + strain_index(bid.strain),
    }
}

/// Decode EPBot's bid code back into a [`Call`], or [`None`] on an error code
fn decode_call(code: c_int) -> Option<Call> {
    match code {
        0 => Some(Call::Pass),
        1 => Some(Call::Double),
        2 => Some(Call::Redouble),
        5..=39 => {
            let index = code - 5;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let level = Level::new((index / 5 + 1) as u8);
            Some(Call::Bid(Bid {
                level,
                strain: STRAINS[(index % 5) as usize],
            }))
        }
        _ => None,
    }
}

/// Strains in EPBot/[`Strain`] discriminant order (♣ ♦ ♥ ♠ NT)
const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

/// The 0..=4 index of a strain
fn strain_index(strain: Strain) -> c_int {
    STRAINS
        .iter()
        .position(|&s| s == strain)
        .expect("every strain is in STRAINS") as c_int
}

/// One-hot logits: the chosen call finite, everything else impossible
pub fn one_hot(call: Call) -> Logits {
    Logits(Array::from_fn(|c| {
        if c == call { 0.0 } else { f32::NEG_INFINITY }
    }))
}

/// The highest-logit *legal* call, defaulting to a pass
pub fn next_call(
    system: &dyn System,
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let Some(logits) = system.classify(hand, relative(vul, seat), auction) else {
        return Call::Pass;
    };
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

/// Bid out one deal with `ours` on `ours_is_ns`'s side, `theirs` on the other
pub fn bid_out(
    ours: &dyn System,
    theirs: &dyn System,
    ours_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let system = if seat_is_ns == ours_is_ns {
            ours
        } else {
            theirs
        };
        auction.push(next_call(system, deal[seat], seat, vul, &auction));
    }
    auction
}

#[cfg(test)]
mod tests {
    /// The vendored BEN card parses to BEN's declared system: 2/1 (id 0) with
    /// its known toggle tweaks vs stock BBA 2/1 (see docs/ben-gap-campaign.md).
    #[test]
    fn ben_card_parses() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/ben/BEN-21GF.bbsa");
        let card = super::load_bbsa(path).expect("vendored card parses");
        assert_eq!(card.system, super::SYSTEM_2_OVER_1);
        assert_eq!(card.toggles.len(), 257);
        let get = |name: &str| {
            card.toggles
                .iter()
                .find(|(n, _)| n.to_str() == Ok(name))
                .unwrap_or_else(|| panic!("card has `{name}`"))
                .1
        };
        // All 10 toggle lines that differ from stock BBA-21GF.bbsa.
        assert_eq!(get("Blackwood 1430"), 1);
        assert_eq!(get("Blackwood 0314"), 0);
        assert_eq!(get("Leaping Michaels"), 1);
        assert_eq!(get("New Minor Forcing"), 0);
        assert_eq!(get("Two Way New Minor Forcing"), 1);
        assert_eq!(get("Strength Lawrence structure"), 1);
        assert_eq!(get("Shape Bergen structure"), 0);
        assert_eq!(get("1N-3M splinter"), 1);
        assert_eq!(get("Gerber only for NT openings"), 1);
        assert_eq!(get("Extended Stayman"), 0);
    }
}
