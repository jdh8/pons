//! probe-trump-vs-nt — trump tricks vs notrump tricks on the same cards, per
//! combined HCP, for 8- and 9+-card fits (bridge-tutorial src/goals.md).
use contract_bridge::eval::hcp;
use contract_bridge::{Seat, Strain, Suit};

fn main() {
    // key: (fit class 8 / 9, major?, hcp) -> [cases, nt>=9, t>=10, t>=11, d>=1, d>=2, sum d]
    let mut cells = std::collections::BTreeMap::<(usize, bool, bool, u32), [u64; 11]>::new();
    for path in std::env::args().skip(1) {
    for (deal, table) in pons::pdd::load(&path).unwrap() {
        let (deal, table) = (&deal, &table);
        for (a, b) in [(Seat::North, Seat::South), (Seat::East, Seat::West)] {
            let h: u32 = Suit::ASC.iter().map(|&s| hcp::<u32>(deal[a][s]) + hcp::<u32>(deal[b][s])).sum();
            let best = |st: Strain| u8::from(table[st].get(a)).max(u8::from(table[st].get(b))) as i32;
            let nt = best(Strain::Notrump);
            // no singleton or void in either hand: the hands where 3NT is a real choice
            let flat = Suit::ASC.iter().all(|&s| deal[a][s].len() >= 2 && deal[b][s].len() >= 2);
            for &s in &Suit::ASC {
                let len = deal[a][s].len() + deal[b][s].len();
                if len < 8 { continue; }
                let t = best(Strain::from(s));
                let d = t - nt;
                let c = cells.entry((len.min(9), matches!(s, Suit::Hearts | Suit::Spades), flat, h)).or_default();
                c[0] += 1;
                c[1] += (nt >= 9) as u64;
                c[2] += (t >= 10) as u64;
                c[3] += (t >= 11) as u64;
                c[4] += (d >= 1) as u64;
                c[5] += (d >= 2) as u64;
                c[6] = c[6].wrapping_add(d as i64 as u64);
                // swing boards: exactly one of the suit game and 3NT makes
                c[7] += (t >= 10 && nt < 9) as u64;
                c[8] += (t < 10 && nt >= 9) as u64;
                c[9] += (t >= 11 && nt < 9) as u64;
                c[10] += (t < 11 && nt >= 9) as u64;
            }
        }
    }
    }
    println!("fit\tmajor\tflat\thcp\tcases\tnt9\tt10\tt11\td1\td2\tsumd\tonly10\tnt_not10\tonly11\tnt_not11");
    for ((f, m, flat, h), c) in cells {
        println!("{f}\t{m}\t{flat}\t{h}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", c[0], c[1], c[2], c[3], c[4], c[5], c[6] as i64, c[7], c[8], c[9], c[10]);
    }
}
