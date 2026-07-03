# Bidding theorems

Anecdotal bridge-bidding principles and observations, jotted as they come up.
Not authored conventions — raw theory. Promote one into the system (or the
AI-bidder corpus) when it earns its keep.

- The [useful space principle](https://en.wikipedia.org/wiki/Useful_space_principle)
  is an axiom for allocating meaning to calls: spend cheaper bids on the more
  frequent / more space-needing meanings.
- Whenever there is a debate between forcing and non-forcing responses, choose
  transfer responses!
  - I summarize [Terrorist’s article](https://www.ptt.cc/man/BridgeClub/D6D1/D49B/D823/M.1042682810.A.3EF.html) on generalized Rubens advances
- Direct XX somehow bears the meaning of natural notrump. Consider using NT as a
  cuebid to X.
- Suit-selection style is a *partnership-wide inference contract*: every rule of
  the form "with two biddable suits, bid X first" is also a promise partner
  reads.  Our minor-opening responses over-apply up-the-line beyond 4-4 — the
  1♥ rule fires on any four-plus hearts, so 5♠4♥ and 6♠5♥ respond 1♥
  (`minor_responses`, `responses.rs`) — which breaks longest-first inference
  and forced the M6.4 control-bid classifier to read `1♣–1♥–2♣–4♠` as natural
  (the naive "shown another suit ⟹ can't be longest" reading lost 6 IMPs per
  fired board on real 6♠5♥ traffic).  The 1NT/2NT transfer tables had the same
  disease and were fixed (`set_transfer_longer_major`: transfer the longer
  major; equal 5-5 splits weak→hearts, INV/min-FG→3♦, slam-try→spades).
  Promoting longest-first into the minor-opening responses must move three
  things together: the response rule, the rebid structure it implies, and the
  classifier's bypass rule — piecemeal changes desynchronize bidder and
  reader.  Built as that trio in `set_longer_major_response` (response pair in
  `minor_responses`, opener's `1♠` rebid under `set_up_the_line`, and the
  discipline-gated bypass swap in `classify_high_bid`) and measured by
  `ab-minor-continuations` (2026-07): a **null** — plain-DD wash alone, and a
  small consistent *negative* marginal (−0.003..−0.005 IMPs/board) on top of
  the shipped xyz + up-the-line package.  The lesson: opener's up-the-line
  `1♠` rebid recovers the concealed 4-4 spade fits more cheaply than
  re-siding the response, and longest-first pays a level on the heart fits.
  Hearts-first stays the default; the knob remains opt-in.
