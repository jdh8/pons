# WBF bidding abbreviations — the corpus tag vocabulary

The description corpus (M0.1/M0.2) needs a *controlled vocabulary* for its
`tags` field so Component A learns canonical bridge terms instead of ad-hoc
ones we invent. We adopt the World Bridge Federation's standard abbreviations,
the same set the [Strawberry Polish Club](https://polish.club/) notes use.

**Source:** *Guide to Completion of the System Card* (WBF), §1.3 "Abbreviations",
`http://www.worldbridge.org/wp-content/uploads/2017/04/Guidetocompletion.pdf`
(retrieved 2026-06-13). The guide's rule — "USE RECOMMENDED ABBREVIATIONS,
don't invent your own" — is our rule for the corpus too.

## How we use it

- **`tags` are drawn from this list** (the `ABBR` column), lower-/mixed-case as
  written. Auto-derived tags (M0.2) map structural facts to these codes:
  a Jacoby/red-suit response over 1NT → `TRF`; a direct double of a suit opening
  → `T/O`; a 2♣ response to 1NT → `STAY`; a new suit at the cheapest level →
  `NAT`; a double jump in a new suit → `SPL`; a game-forcing 2/1 → `FG`/`F`; a
  weak two / preempt → `PRE`/`WK`; a balanced opener → `BAL`; etc.
- **`description`** (the prose field) stays plain English; the abbreviations are
  for the machine-readable `tags`, mirroring how a convention card keys terse
  codes to fuller meanings.
- Seat terms (`LHO`, `RHO`, `PH`, `ADV`, `RESP`) already correspond to our
  `Context`/`Inferences` relative seats — useful when a tag is seat-specific.

## The list

Shape notation (suit-count templates; spades-first):

| ABBR | Meaning |
|------|---------|
| `(5431)` | Any hand with that distribution, suits unknown |
| `5431` | Five spades, four hearts, three diamonds, one club |
| `5[4](31)` | Five spades, four hearts, and 3-1 or 1-3 in the minors |
| `54(xx)` | Five spades, four hearts, other two suits unspecified |

Terms:

| ABBR | Meaning | ABBR | Meaning |
|------|---------|------|---------|
| `AGG` | Aggressor — first to double/overcall for the defence | `INV` | Invitational |
| `ADV` | Advancer — aggressor's partner | `INQ` | Inquiry |
| `ASK` | Asking bid | `KCB` | Keycard Blackwood |
| `ART` | Artificial | `L/D` | Lead-directing |
| `ATT` | Attitude | `LEB` | lebensohl |
| `B` | Black suit(s) | `LHO` | The opponent on your left |
| `BAL` | Balanced | `LIM` | Limit raise |
| `BW` | Blackwood | `L/S` | Long suit |
| `CB` | Checkback | `L/T` | Less than (length or strength) |
| `COMP` | Competitive | `M` / `MM` | Major / majors |
| `CONC` | Concentrated (values in the bid suits) | `m` / `mm` | Minor / minors |
| `CONST` | Constructive | `MAX` | Maximum, Maximal |
| `CTRL` | Control | `MIN` | Minimum |
| `CUE` | Cue-bid | `NAT` | Natural |
| `DBL` / `X` | Double | `NEG` | Negative |
| `DISC` | Discourage(ing) | `NEU` | Neutral |
| `E` | Even | `NF` | Nonforcing |
| `ENC` | Encourage(ing) | `NT` | No Trump |
| `FRAG` | Fragment | `NV` | Nonvulnerable |
| `F` | Forcing | `oM` | The other major |
| `F1` | Forcing 1 round | `om` | The other minor |
| `F2NT` | Forcing to 2NT | `OPPT` | Opponent(s) |
| `FG` | Forcing to game | `OPT` | Optional |
| `4SF` | 4th suit forcing (`4SFG`, `4SF1`) | `O/S` | Outside |
| `FREQ` | Frequent | `O/C` | Overcall |
| `G/T` | Game try | `P/C` | Pass or correct |
| `H` | Honour (A, K, or Q) | `PEN` | Penalty |
| `HCP` | High Card Points | `PH` | Passed hand |
| `PRE` | Pre-emptive | `S/T` | Slam try |
| `PUP` | Puppet to (e.g. 2♣ demands 2♦) | `STAY` | Stayman |
| `QUANT` | Quantitative | `STR` | Strong |
| `R` | Red suit(s) | `SUPP` | Support |
| `(R)` | Relay (asks for a description) | `T/O` | Takeout |
| `RDBL` / `RD` | Redouble | `TRF` | Transfer |
| `RESP` | Responder; Response; Responsive | `UNT` | Unusual No Trump |
| `REV` | Reverse | `VUL` / `V` | Vulnerable |
| `RHO` | The opponent on your right | `w/` | With |
| `RKCB` | Roman Keycard Blackwood | `w/o` | Without |
| `R/O` | Reopening | `WJO` | Weak jump overcall |
| `S/P` | Suit preference | `WJS` | Weak jump shift |
| `S/A` | Suit agreement | `WK` | Weak |
| `S/O` | Signoff, shutout | `SOL` | Solid (suit) |
| `S/S` | Short suit | `S-SOL` | Semi-solid (suit) |
| `SPL` | Splinter, or short suit | | |
