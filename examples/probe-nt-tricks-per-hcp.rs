//! probe-nt-tricks-per-hcp — notrump trick histogram per combined HCP, for
//! balanced opposite balanced and for all pairs (bridge-tutorial src/opening.md:
//! how many HCP is one trick near 3NT).
use contract_bridge::eval::hcp;
use contract_bridge::{Hand, Seat, Strain, Suit};

fn balanced(h: Hand) -> bool {
    let mut l: Vec<usize> = Suit::ASC.iter().map(|&s| h[s].len()).collect();
    l.sort_unstable();
    l[0] >= 2 && l[1] >= 3 // 4333, 4432, 5332
}

fn main() {
    // [balanced?][hcp][tricks]
    let mut cells = vec![[[0u64; 14]; 41]; 2];
    for path in std::env::args().skip(1) {
        for (deal, table) in pons::pdd::load(&path).unwrap() {
            for (a, b) in [(Seat::North, Seat::South), (Seat::East, Seat::West)] {
                let h: usize = Suit::ASC
                    .iter()
                    .map(|&s| hcp::<usize>(deal[a][s]) + hcp::<usize>(deal[b][s]))
                    .sum();
                let nt = table[Strain::Notrump];
                let t = usize::from(u8::from(nt.get(a)).max(u8::from(nt.get(b))));
                cells[0][h][t] += 1;
                if balanced(deal[a]) && balanced(deal[b]) {
                    cells[1][h][t] += 1;
                }
            }
        }
        eprintln!("{path}");
    }
    println!(
        "bal\thcp\t{}",
        (0..14)
            .map(|t| format!("t{t}"))
            .collect::<Vec<_>>()
            .join("\t")
    );
    for (bal, rows) in cells.iter().enumerate() {
        for (h, row) in rows.iter().enumerate() {
            if row.iter().sum::<u64>() > 0 {
                println!(
                    "{bal}\t{h}\t{}",
                    row.iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join("\t")
                );
            }
        }
    }
}
