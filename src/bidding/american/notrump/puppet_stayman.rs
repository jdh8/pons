//! Puppet Stayman — `1NT - 3♣` asking for a five-card major
//!
//! Opener's answers, the denial, and the Smolen completion below it.  Built
//! only under [`PUPPET`](super::PUPPET), the shipped minor scheme.
use super::*;

/// Opener's answer to Puppet Stayman: a five-card major, else 3♦ to deny
///
/// Puppet is balanced and game-forcing, so opener always cooperates — name a
/// five-card major (`3♥`/`3♠`), otherwise bid `3♦`, denying five but possibly
/// holding a four-card major for the Smolen-style 4-4 hunt below.
fn puppet_answers(agreements: &Agreements) -> Rules {
    let completion_alerts = agreements.decision.reading.completion_alerts;
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 5..))
        .alert_if(completion_alerts, COMPLETION)
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            len(Suit::Spades, 5..) & len(Suit::Hearts, ..5),
        )
        .alert_if(completion_alerts, COMPLETION)
        .rule(
            Bid::new(3, Strain::Diamonds),
            50,
            len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .alert_if(completion_alerts, COMPLETION)
}

/// Responder's rebid after opener names a five-card major over Puppet
///
/// Three-card support is an eight-card fit — bid game in the major so opener
/// declares; otherwise opener's major was responder's short one, so settle in
/// 3NT.  Puppet hands are balanced, so there is no splinter slam-try here (that
/// tool lives in the shapely 2♠ club structure).
fn puppet_major_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 100, len(major, 3..))
        .rule(Bid::new(3, Strain::Notrump), 50, len(major, ..3))
}

/// Responder's rebid after opener denies a five-card major (`1NT - 3♣ - 3♦`)
///
/// Smolen-style: a four-card major (so responder is 4-3) bids the *shorter*
/// three-card major to show four in the longer, right-siding game to opener.
/// With no four-card major (3-3, or three and a short major) there is no 4-4 to
/// find — settle in 3NT.
fn puppet_deny_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
            len(Suit::Spades, 4..=4) & len(Suit::Hearts, 3..=3),
        )
        .alert(SMOLEN)
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, 3..=3),
        )
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

/// Opener completes the Puppet 4-4 hunt: game in responder's shown major, or 3NT
///
/// Responder's short-major bid named four cards in `shown_major`; raise to game
/// with four-card support, else 3NT.
fn puppet_smolen_completion(shown_major: Suit, agreements: &Agreements) -> Rules {
    let completion_alerts = agreements.decision.reading.completion_alerts;
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(shown_major)),
            100,
            len(shown_major, 4..),
        )
        .alert_if(completion_alerts, COMPLETION)
        .rule(Bid::new(3, Strain::Notrump), 50, len(shown_major, ..4))
        .alert_if(completion_alerts, COMPLETION)
}

/// Puppet Stayman responses and continuations after the 1NT - 3♣ ask
pub(crate) fn puppet() -> Package {
    Package {
        name: "puppet-stayman",
        gate: |agreements| puppet_scheme(agreements),
        entries: |agreements| {
            let mut entries = rows_of(Pattern::node("P* 1NT - 3♣ -"), puppet_answers(agreements));
            entries.extend(expand(
                "P* 1NT - 3♣ - 3M -",
                |_| true,
                |b| puppet_major_rebid(b.suit('M')),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 3♣ - 3♦ -"),
                puppet_deny_rebid(),
            ));

            // The shorter-major bid shows four cards in the other major, so
            // these derived-suit completions stay literal.
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 3♣ - 3♦ - 3♥ -"),
                puppet_smolen_completion(Suit::Spades, agreements),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 3♣ - 3♦ - 3♠ -"),
                puppet_smolen_completion(Suit::Hearts, agreements),
            ));
            entries
        },
    }
}
