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

// The bilans-engine surface (docs/ai-bidder/bba-floor.md §5-6).  Signatures are
// confirmed twice over: the managed `EPBotFFI` shim's ECMA-335 metadata in
// `vendor/bba/Native-libraries/wasm/EPBotWasm.dll` carries them with parameter
// names, and BEN's independent ctypes binding (`src/bba/BBA.py`) agrees.
//
// `epbot_get_info_alerting(bot, position)` — a value getter, returning directly.
type GetIntFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
// `epbot_set_scoring(bot, value)` — IMP vs MP; Stage 4 is scoring-form-aware.
type SetIntFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
// `epbot_get_scoring(bot)`
type GetNoArgFn = unsafe extern "C" fn(*mut c_void) -> c_int;
// `epbot_get_probable_levels(bot, buf, buf_bytes, count_out)` — Stage 4's output.
// The per-strain scalar `epbot_get_probable_level(bot, strain)` is deliberately
// NOT bound: this returns the same numbers in one call rather than five, and it
// is what revealed the array is 9 entries long, not the 5 strains we assumed.
type GetFlatArrFn = unsafe extern "C" fn(*mut c_void, *mut c_int, c_int, *mut c_int) -> c_int;
// `epbot_get_info_<field>(bot, position, buf, buf_bytes, count_out)` — note the
// buffer size is in BYTES, not elements (BBA.py's `_int_array_call`).
type GetArrFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int, c_int, *mut c_int) -> c_int;
// `epbot_set_info_<field>(bot, position, data, count)`
type SetArrFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_int, c_int) -> c_int;

/// The eight `info_*` array fields, in [`SeatInfo`]/[`InfoField`] order
///
/// Each has an `epbot_get_info_<name>` and an `epbot_set_info_<name>` export
/// sharing one signature, so they bind as a pair of arrays rather than sixteen
/// named fields.
const INFO_FIELDS: [&str; 8] = [
    "min_length",
    "max_length",
    "probable_length",
    "strength",
    "stoppers",
    "honors",
    "suit_power",
    "feature",
];

/// Index into [`INFO_FIELDS`], naming which `info_*` array to read or write
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoField {
    MinLength = 0,
    MaxLength = 1,
    ProbableLength = 2,
    Strength = 3,
    Stoppers = 4,
    Honors = 5,
    SuitPower = 6,
    Feature = 7,
}

/// Positions in the `info_*` block
///
/// BBA's own documentation: *"Positions from 0 to 3 contain public information
/// based on the auction.  Positions from 4 to 7 are calculated probable hands
/// of the players."*  So 4..8 is the output of §5's Stage 1 — the reconstruction
/// this whole binding exists to read.
pub const INFO_POSITIONS: usize = 8;

/// Longest `info_*` array: `feature` is 512, every other field is 4 (one per
/// suit, in EPBot's C,D,H,S order).
// ponytail: one fixed buffer for every read, so the `-3` (buffer too small)
// status can never occur and BEN's grow-and-retry loop is unnecessary.  If a
// future field exceeds this, the `debug_assert` in `read_info` fires.
const INFO_CAPACITY: usize = 512;

/// Entries `epbot_get_probable_levels` returns
///
/// Measured, not assumed: 5 strains plus 4 undecoded trailing slots.
pub const PROBABLE_LEVELS: usize = 9;

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
    // The bilans surface, bound but only used by `probe`/`probe_with`.
    get_probable_levels: GetFlatArrFn,
    get_scoring: GetNoArgFn,
    set_scoring: SetIntFn,
    get_info_alerting: GetIntFn,
    get_info: [GetArrFn; INFO_FIELDS.len()],
    set_info: [SetArrFn; INFO_FIELDS.len()],
    system: c_int,
    /// Named conventions forced to a value on all four seats of every fresh bot,
    /// applied after `set_system` (which loads the system's defaults).
    overrides: Vec<(CString, c_int)>,
    /// Scoring form forced on every fresh bot, or [`None`] to keep BBA's default
    ///
    /// Stage 4 picks its level by expected score, so this changes its answers.
    scoring: Option<c_int>,
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
        // SAFETY: each symbol has the signature confirmed in the S.0 spike (the
        // original seven) or in the `EPBotFFI` metadata parse recorded in
        // docs/ai-bidder/bba-floor.md §6 (the bilans surface); `*sym` copies the
        // function pointer (it is `Copy` and does not borrow the library, which
        // we keep alive in `_lib`).
        unsafe {
            // A fn pointer is non-null, so an uninitialized array is not an
            // option; collect and convert instead.
            let mut get_info = Vec::with_capacity(INFO_FIELDS.len());
            let mut set_info = Vec::with_capacity(INFO_FIELDS.len());
            for field in INFO_FIELDS {
                let getter = format!("epbot_get_info_{field}\0");
                let setter = format!("epbot_set_info_{field}\0");
                get_info.push(*lib.get::<GetArrFn>(getter.as_bytes())?);
                set_info.push(*lib.get::<SetArrFn>(setter.as_bytes())?);
            }
            let get_info = get_info.try_into().expect("one getter per INFO_FIELD");
            let set_info = set_info.try_into().expect("one setter per INFO_FIELD");
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                set_conv: *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
                get_probable_levels: *lib.get::<GetFlatArrFn>(b"epbot_get_probable_levels\0")?,
                get_scoring: *lib.get::<GetNoArgFn>(b"epbot_get_scoring\0")?,
                set_scoring: *lib.get::<SetIntFn>(b"epbot_set_scoring\0")?,
                get_info_alerting: *lib.get::<GetIntFn>(b"epbot_get_info_alerting\0")?,
                get_info,
                set_info,
                _lib: lib,
                system,
                overrides,
                scoring: None,
            })
        }
    }

    /// Force a scoring form on every fresh bot (see [`BbaState::scoring`])
    #[must_use]
    pub fn with_scoring(mut self, scoring: Option<c_int>) -> Self {
        self.scoring = scoring;
        self
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

impl BbaOracle {
    /// Build a fresh bot for one decision, run `read` on it, and destroy it
    ///
    /// Configures all four seats to the chosen system, deals the actor's hand,
    /// and replays the auction so far with `set_bid` — the shared setup behind
    /// both [`System::classify`] and [`BbaOracle::probe`].  A fresh bot per
    /// decision keeps both a pure, stateless function of their arguments.
    ///
    /// Returns [`None`] only if EPBot fails to allocate a bot.
    fn with_bot<T>(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        read: impl FnOnce(*mut c_void) -> T,
    ) -> Option<T> {
        // Canonicalize the dealer to position 0: the actor is the seat that has
        // bid `auction.len()` times after the dealer.  The relative seat and the
        // favorable/unfavorable vulnerability are preserved by the replayed calls
        // and the mapping below, so the bid is identical to the true seating.
        let actor = (auction.len() % 4) as c_int;
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();

        // SAFETY: a fresh bot used and destroyed within this call; all argument
        // types match the confirmed ABI.
        unsafe {
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
            if let Some(scoring) = self.scoring {
                (self.set_scoring)(bot, scoring);
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
            let value = read(bot);
            (self.destroy)(bot);
            Some(value)
        }
    }

    /// Read one `info_*` array for one position
    ///
    /// # Safety
    /// `bot` must be a live bot from [`BbaOracle::with_bot`].
    unsafe fn read_info(&self, bot: *mut c_void, field: InfoField, position: c_int) -> Vec<c_int> {
        let mut buffer = [0_i32; INFO_CAPACITY];
        let mut count: c_int = 0;
        // The buffer size is in BYTES, not elements.
        let bytes = (INFO_CAPACITY * size_of::<c_int>()) as c_int;
        // SAFETY: the buffer and the count out-param outlive the call, and the
        // argument types match the confirmed ABI.
        let status = unsafe {
            (self.get_info[field as usize])(bot, position, buffer.as_mut_ptr(), bytes, &mut count)
        };
        debug_assert!(status >= 0, "epbot_get_info_* failed with {status}");
        let count = usize::try_from(count).unwrap_or(0).min(INFO_CAPACITY);
        buffer[..count].to_vec()
    }

    /// Read BBA's whole bilans state for one decision
    ///
    /// This is the [`System::classify`] call plus everything the engine computed
    /// on the way there: its target level per strain, and its reconstruction of
    /// all four hands.  See `docs/ai-bidder/bba-floor.md` §5-6.
    pub fn probe(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<BbaState> {
        self.with_bot(hand, vul, auction, |bot| self.read_state(bot))
    }

    /// [`BbaOracle::probe`], but overwriting part of BBA's reconstruction first
    ///
    /// Each `(position, field, values)` triple is written with `set_info_*`
    /// *before* the call is read, so Stages 2-4 (evaluation, trick counting,
    /// level choice) can be exercised against a hand model we supply rather than
    /// the one Stage 1 inferred.
    pub fn probe_with(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        inject: &[(c_int, InfoField, Vec<c_int>)],
    ) -> Option<BbaState> {
        self.with_bot(hand, vul, auction, |bot| {
            for (position, field, values) in inject {
                let count = c_int::try_from(values.len()).expect("an info array fits in c_int");
                // SAFETY: `bot` is live; `values` outlives the call.
                let status = unsafe {
                    (self.set_info[*field as usize])(bot, *position, values.as_ptr(), count)
                };
                debug_assert!(status >= 0, "epbot_set_info_* failed with {status}");
            }
            self.read_state(bot)
        })
    }

    /// Read the full state off a live bot
    ///
    /// `get_bid` comes **first** on purpose: it is what drives EPBot's
    /// `calculated bid` path, so the level and reconstruction blocks are only
    /// populated once it has run.  Reading them first yields stale state.
    fn read_state(&self, bot: *mut c_void) -> BbaState {
        // SAFETY: `bot` is a live bot from `with_bot`, and every call below
        // matches the confirmed ABI.
        let call = decode_call(unsafe { (self.get_bid)(bot) });
        let scoring = unsafe { (self.get_scoring)(bot) };
        let probable_level = {
            let mut buffer = [0_i32; PROBABLE_LEVELS];
            let mut count: c_int = 0;
            let bytes = (PROBABLE_LEVELS * size_of::<c_int>()) as c_int;
            // SAFETY: buffer and count out-param outlive the call.
            let status =
                unsafe { (self.get_probable_levels)(bot, buffer.as_mut_ptr(), bytes, &mut count) };
            debug_assert!(
                status >= 0,
                "epbot_get_probable_levels failed with {status}"
            );
            debug_assert_eq!(
                count as usize, PROBABLE_LEVELS,
                "probable_levels changed length; re-read its semantics"
            );
            buffer
        };
        let seats = core::array::from_fn(|position| {
            let position = position as c_int;
            let suits = |field| {
                // Every field but `feature` is one entry per suit; pad a short
                // read rather than panicking, so a surprise is visible in the
                // dump instead of aborting a long run.
                let values = unsafe { self.read_info(bot, field, position) };
                core::array::from_fn(|suit| values.get(suit).copied().unwrap_or(0))
            };
            SeatInfo {
                alerting: unsafe { (self.get_info_alerting)(bot, position) },
                min_length: suits(InfoField::MinLength),
                max_length: suits(InfoField::MaxLength),
                probable_length: suits(InfoField::ProbableLength),
                strength: suits(InfoField::Strength),
                stoppers: suits(InfoField::Stoppers),
                honors: suits(InfoField::Honors),
                suit_power: suits(InfoField::SuitPower),
                // Sparse: ~500 of the 512 feature slots are undecoded and almost
                // all zero, so keeping the nonzero ones is lossless and small.
                features: unsafe { self.read_info(bot, InfoField::Feature, position) }
                    .into_iter()
                    .enumerate()
                    .filter(|&(_, value)| value != 0)
                    .map(|(index, value)| (index as u16, value))
                    .collect(),
            }
        });
        BbaState {
            call,
            scoring,
            probable_level,
            seats,
        }
    }
}

impl System for BbaOracle {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        // SAFETY: `bot` is live for the duration of the closure.
        let code = self.with_bot(hand, vul, auction, |bot| unsafe { (self.get_bid)(bot) })?;
        decode_call(code).map(one_hot)
    }
}

/// BBA's model of one position's hand
///
/// Suit-indexed arrays are in EPBot's **C, D, H, S** order.  Measured against
/// the actor's own (exactly known) slot over the 21039-row 7A recon dump:
///
/// * `strength` is the suit's HCP, exactly.
/// * `honors` is a bitmask: A = 16, K = 8, Q = 4, J = 2, T = 1, exactly.
/// * `probable_length` is **effectively unused** — nonzero on under 1% of rows.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SeatInfo {
    pub alerting: c_int,
    pub min_length: [c_int; 4],
    pub max_length: [c_int; 4],
    pub probable_length: [c_int; 4],
    pub strength: [c_int; 4],
    pub stoppers: [c_int; 4],
    pub honors: [c_int; 4],
    pub suit_power: [c_int; 4],
    /// The nonzero entries of the 512-slot `feature` array, by index
    ///
    /// Decoded indices (BEN's `find_info`): 402/403 min/max HCP, 406 aces,
    /// 407 kings, 319 queen (BBA swaps the meaning of −1 and 0), 424 trump
    /// strain, 425 asking-bid code.  The rest are undecoded.
    pub features: std::collections::BTreeMap<u16, c_int>,
}

impl SeatInfo {
    /// The reconstructed HCP band, from `feature[402]`/`feature[403]`
    #[must_use]
    pub fn hcp_range(&self) -> (c_int, c_int) {
        let get = |index| self.features.get(&index).copied().unwrap_or(0);
        (get(402), get(403))
    }
}

/// BBA's internal state for one decision — §5's bilans engine, read out
///
/// Positions 0..4 of `seats` hold public information deduced from the auction
/// (always a band, never exact).  Positions 4..8 hold BBA's *calculated probable
/// hands*, i.e. Stage 1's output, and are indexed by the same canonicalized
/// seat: **`seats[4 + auction.len() % 4]` is the actor's own hand, exactly**,
/// on all 21039 rows of the 7A recon dump.  Offsets 1, 2 and 3 from there are
/// LHO, partner and RHO, with mean HCP bands 17.9, 15.4 and 12.4 wide — RHO is
/// the best known, its call being the most recent evidence.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BbaState {
    /// The call EPBot chose, or [`None`] on an error code
    pub call: Option<Call>,
    /// The scoring form in force; Stage 4 picks its level by expected score
    pub scoring: c_int,
    /// Stage 4's output: 9 entries, of which 0..5 are the strains ♣ ♦ ♥ ♠ NT
    ///
    /// Values are **not** a 1-7 contract level: they span −6..7 and go negative
    /// before the auction says much (a flat 16 opening reads
    /// `[-1, -2, -1, -2, -1]`), climbing as it firms up (`[5, 5, 6, 7, 7]` in a
    /// slam auction).  They behave like an offset from a baseline rather than a
    /// bid, and decoding the exact scale is open work for session C.
    ///
    /// The strain ordering *is* confirmed: `argmax` over entries 0..4 picks the
    /// actor's own longest suit on 51.3% of rows against a ~27% chance rate.
    /// Do not correlate these against the final contract without splitting by
    /// declaring side — half the time the opponents declare, which drives the
    /// naive agreement rate *below* chance.
    ///
    /// Entry 5 is live but undecoded: it is 0 until partner has bid, then rises
    /// with auction depth.  Entries 6..9 are **always 0**.  See
    /// `docs/ai-bidder/bba-floor.md` §6.
    pub probable_level: [c_int; PROBABLE_LEVELS],
    pub seats: [SeatInfo; INFO_POSITIONS],
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
    // NOTE: there is deliberately no test here that drives EPBot.  Its NativeAOT
    // runtime segfaults when called from a `cargo test` thread — the
    // pre-existing 7-symbol `classify` path does too, which is why this module
    // has only ever tested pure parsing.  The ABI self-check that would live
    // here runs on the main thread instead, as
    // `cargo run --example probe-bba-bilans -- --self-check`.

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
