# Disclosure sensitivity of `cards/American.bbsa`

8406 BBA decisions replayed from `ab-results/anchor/2026-07-26-eb02d9d/none/shard-0.json`, `ab-results/anchor/2026-07-26-eb02d9d/both/shard-0.json`; each row flipped to 0 and to 1 on the seats pons occupies, counting decisions whose call moved. Replay self-check: 100.0% of recorded calls reproduced with nothing disclosed.

## Live (33 rows)

| row | card value | decisions moved |
| --- | --- | --- |
| Cappelletti | 0 | 38 |
| Weak natural 2M | 1 | 36 |
| Raptor 1NT | 0 | 21 |
| 1NT opening natural | 0 | 16 |
| 1NT opening NT style | 1 | 16 |
| Ghestem | 0 | 14 |
| 1NT opening range 12-14 | 0 | 13 |
| 1NT opening range 13-15 | 0 | 13 |
| 1NT opening range 14-16 | 0 | 13 |
| Michaels Cuebid | 1 | 12 |
| 1D opening with 5 cards | 0 | 10 |
| Benjamin 2D | 0 | 10 |
| French 2D | 0 | 10 |
| Multi-Landy | 0 | 10 |
| Polish two suiters | 0 | 10 |
| Landy | 0 | 6 |
| 1D opening with 4 cards | 0 | 4 |
| Checkback | 0 | 4 |
| Garbage Stayman | 1 | 3 |
| 1N-2N transfer to clubs | 0 | 2 |
| 1N-2N transfer to diamonds | 1 | 2 |
| Fit showing jumps | 0 | 2 |
| Fourth suit game force | 1 | 2 |
| Gazzilli | 0 | 2 |
| Mini Splinter | 0 | 2 |
| Responsive double | 1 | 2 |
| Unusual 2NT | 1 | 2 |
| Weak Jump Shifts 2 | 0 | 2 |
| Kickback 0123 | 0 | 1 |
| Kickback 0314 | 0 | 1 |
| Kickback 1430 | 0 | 1 |
| Mixed raise | 0 | 1 |
| Support double redouble | 1 | 1 |

## Cosmetic (224 rows)

> **"Cosmetic" means the sweep never reached the node, not that the row is
> inert.** `Transfers if RHO bids clubs` sits here at 0 of 8406 decisions moved,
> yet driving BBA directly at `1NT (2♣) ?` shows it governing a real structure
> — see [bba-1nt-counter-defense.md](bba-1nt-counter-defense.md). Anchor dumps
> are dominated by uncontested auctions, so a row that only fires under a
> specific interference call cannot show up here. Treat this table as a
> frequency floor, and probe the node before concluding a row does nothing.

| row | card value | decisions moved |
| --- | --- | --- |
| (1X)-1Y-(1Z)-2Z natural | 0 | 0 |
| 1m opening allows 5M | 0 | 0 |
| 1M-3M blocking | 0 | 0 |
| 1M-3M inviting | 1 | 0 |
| 1N-2S Minor Suit Stayman | 0 | 0 |
| 1N-2S transfer to clubs | 1 | 0 |
| 1N-3C transfer to diamonds | 0 | 0 |
| 1N-3C Puppet Stayman | 1 | 0 |
| 1N-3D majors | 1 | 0 |
| 1N-3D minors | 0 | 0 |
| 1N-3D natural | 0 | 0 |
| 1N-3D splinter | 0 | 0 |
| 1N-3M splinter | 0 | 0 |
| 1NT opening range 15-17 | 1 | 0 |
| 1NT opening shape 4441 | 0 | 0 |
| 1NT opening shape 5422 | 1 | 0 |
| 1NT opening shape 6 minor | 1 | 0 |
| 1X-(Y)-2Z forcing | 1 | 0 |
| 1X-(1Y)-2Z strong | 0 | 0 |
| 1X-(1Y)-2Z weak | 0 | 0 |
| 2N-3C-3N both majors | 0 | 0 |
| 2N-3C Puppet Stayman | 0 | 0 |
| 4NT opening | 0 | 0 |
| 5431 after 1NT | 0 | 0 |
| 5NT pick a slam | 0 | 0 |
| Bergen | 0 | 0 |
| Blackwood 0123 | 0 | 0 |
| Blackwood 0314 | 0 | 0 |
| Blackwood 1430 | 1 | 0 |
| Blackwood without K and Q | 0 | 0 |
| BROMAD | 1 | 0 |
| Crosswood 0123 | 0 | 0 |
| Crosswood 0314 | 0 | 0 |
| Crosswood 1430 | 0 | 0 |
| Cue bid | 1 | 0 |
| DEPO | 0 | 0 |
| Direct Jump Cuebid | 0 | 0 |
| DOPI | 1 | 0 |
| Drury | 0 | 0 |
| Exclusion | 1 | 0 |
| Extended Stayman | 0 | 0 |
| Extended acceptance after NT | 1 | 0 |
| Forcing 1NT | 1 | 0 |
| Fourth suit | 1 | 0 |
| Gambling | 0 | 0 |
| Gerber | 0 | 0 |
| Gerber only for NT openings | 0 | 0 |
| Imposible 2S | 0 | 0 |
| Inverted minors | 1 | 0 |
| Inviting Jump Shifts | 0 | 0 |
| Jacoby 2NT | 1 | 0 |
| Jordan Truscott 2NT | 1 | 0 |
| King ask by 5NT | 1 | 0 |
| King ask by 5NT inviting | 0 | 0 |
| King ask by available bid | 0 | 0 |
| Leaping Michaels | 1 | 0 |
| Lebensohl after 1NT | 1 | 0 |
| Lebensohl after 1m | 0 | 0 |
| Lebensohl after double | 0 | 0 |
| Maximal Doubles | 1 | 0 |
| Minor Suit Slam Try after 2NT | 0 | 0 |
| Minor Suit Stayman after 2NT | 0 | 0 |
| Minor Suit Transfers after 2NT | 1 | 0 |
| Multi | 0 | 0 |
| Namyats | 0 | 0 |
| Natural 3N entering style | 0 | 0 |
| New Minor Forcing | 0 | 0 |
| Non-Leaping Michaels | 0 | 0 |
| Ogust | 1 | 0 |
| Quantitative 4NT | 1 | 0 |
| Reverse Bergen | 0 | 0 |
| Reverse drury | 0 | 0 |
| ROPI | 1 | 0 |
| Rubensohl after 1NT | 0 | 0 |
| Rubensohl after 1m | 0 | 0 |
| Rubensohl after double | 1 | 0 |
| Semi forcing 1NT | 0 | 0 |
| Shape Bergen structure | 0 | 0 |
| SMOLEN | 1 | 0 |
| Snapdragon Double | 0 | 0 |
| Soloway Jump Shifts | 0 | 0 |
| Soloway Jump Shifts Extended | 0 | 0 |
| Splinter | 1 | 0 |
| Strength Lawrence structure | 1 | 0 |
| Super acceptance after NT | 1 | 0 |
| Support 1NT | 1 | 0 |
| Surplus pass | 0 | 0 |
| Texas | 1 | 0 |
| Transfers if RHO passes | 0 | 0 |
| Transfers if RHO doubles | 0 | 0 |
| Transfers if RHO bids clubs | 1 | 0 |
| Two suit takeout double | 1 | 0 |
| Two way game tries | 1 | 0 |
| Two Way New Minor Forcing | 1 | 0 |
| Unusual 1NT | 1 | 0 |
| Unusual 3NT | 0 | 0 |
| Unusual 4NT | 1 | 0 |
| Weak Jump Shifts 3 | 0 | 0 |
| Weak natural 2D | 1 | 0 |
| Wilkosz | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Not defined | 0 | 0 |
| Opponent type | 0 | 0 |

