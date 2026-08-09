//! Support doubles and redoubles — opener shows exactly three-card support
//!
//! After a minor opening and a major response, opener's `X`/`XX` promises
//! exactly three of responder's major.  Gated by
//! [`set_major_support_double`].
use super::*;

thread_local! {
    /// Whether opener's support double/redouble extends to the major-major
    /// auction `1♥ - 1♠ (X / overcall below 2♠)`. The minor-opening
    /// pairs are always on (shipped). **Default on** — measured vs BBA 2/1
    /// (204.8k boards/arm/vul): plain DD wash (−0.0004/+0.0004, CIs straddle
    /// 0), perfect-defense +0.97/+1.69 IMPs/fired NV/vul (vul CI excludes 0)
    /// — the plain-wash + PD-gain ship row (~0.10% fired).
    static MAJOR_SUPPORT_DOUBLE: Cell<bool> = const { Cell::new(true) };
}

/// Extend support doubles to `1♥ - 1♠` for books built *after* this
/// call (thread-local)
///
/// **Default on** (`--no-ns-major-support-double` in `bba-gen` for the off
/// arm).
pub fn set_major_support_double(on: bool) {
    MAJOR_SUPPORT_DOUBLE.with(|cell| cell.set(on));
}

/// Whether the major-major support double is engaged
pub fn major_support_double() -> bool {
    MAJOR_SUPPORT_DOUBLE.with(Cell::get)
}

/// Opener's support double/redouble rules showing three-card support for major M
///
/// `Call::Double` with exactly 3 (support double); `2M` with 4+ (natural raise);
/// Pass as the catch-all.
fn support_rules(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Call::Double, 150, support(3..=3))
        .alert(SUPPORT_DOUBLE)
        .rule(Bid::new(2, m), 140, support(4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Section 3 as a row package: support doubles and redoubles
///
/// The four minor-major pairs always; `1♥ - 1♠` behind
/// [`set_major_support_double`][super::set_major_support_double] (default on).
/// The double answers an overcall at most one step below our major
/// ([`OvercallAtMost`]); the redouble answers their takeout double.
pub(super) fn support_double_package() -> Package {
    Package {
        name: "support-double",
        gate: |_| true,
        entries: |_| {
            let mut support_pairs = vec![
                (Suit::Clubs, Suit::Hearts),
                (Suit::Clubs, Suit::Spades),
                (Suit::Diamonds, Suit::Hearts),
                (Suit::Diamonds, Suit::Spades),
            ];
            if major_support_double() {
                support_pairs.push((Suit::Hearts, Suit::Spades));
            }
            let mut entries = Vec::new();
            for (opening, major) in support_pairs {
                let m = Strain::from(major);
                let key = format!("P* 1{} - 1{m}", Strain::from(opening));
                let just_below = if major == Suit::Hearts {
                    Bid::new(2, Strain::Diamonds)
                } else {
                    Bid::new(2, Strain::Hearts)
                };

                // Support double: they overcall at most `just_below`
                entries.extend(rows_of(
                    Pattern::up_to(&key, &just_below.to_string()),
                    support_rules(major),
                ));

                // Support redouble: they doubled
                entries.extend(rows_of(
                    Pattern::after(&key, "(X)"),
                    Rules::new()
                        .rule(Call::Redouble, 150, support(3..=3))
                        .alert(SUPPORT_DOUBLE)
                        .rule(Bid::new(2, m), 140, support(4..))
                        .rule(Call::Pass, 0, hcp(0..)),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
