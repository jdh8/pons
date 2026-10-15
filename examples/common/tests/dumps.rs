use super::*;
use std::path::PathBuf;

struct Shards(PathBuf);

impl Shards {
    fn new(name: &str, change_second: impl FnOnce(&mut Dump)) -> Self {
        let path = std::env::temp_dir().join(format!("pons-dump-{name}-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        let dir = Self(path);
        let mut dump = Dump {
            our_label: "ours".into(),
            their_label: "reference".into(),
            vulnerability: AbsoluteVulnerability::NONE,
            seed: Some(1),
            gen_args: vec!["--seed".into(), "1".into()],
            boards: vec![Board {
                deal: seeded_deals(1, 1)[0],
                dealer: Seat::North,
                table_a: Auction::new(),
                table_b: Auction::new(),
            }],
        };
        // Write in reverse order to exercise the loader's filename sort.
        let first = serde_json::to_vec(&dump).unwrap();
        dump.seed = Some(2);
        dump.gen_args = vec!["--seed".into(), "2".into()];
        dump.boards[0].deal = seeded_deals(2, 1)[0];
        change_second(&mut dump);
        dir.write("shard-1.json", &serde_json::to_vec(&dump).unwrap());
        dir.write("shard-0.json", &first);
        dir
    }

    fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.0.join(name), bytes).unwrap();
    }

    fn load(&self) -> Dump {
        load_dump(self.0.to_str().unwrap())
    }
}

impl Drop for Shards {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn matching_shards_merge_in_filename_order_with_independent_seeds() {
    let shards = Shards::new("matching", |_| {});
    let dump = shards.load();
    assert_eq!(dump.boards.len(), 2);
    assert_eq!(dump.boards[0].deal, seeded_deals(1, 1)[0]);
    assert_eq!(dump.boards[1].deal, seeded_deals(2, 1)[0]);
    assert_eq!(dump.our_label, "ours");
    assert_eq!(dump.their_label, "reference");
    assert_eq!(dump.vulnerability, AbsoluteVulnerability::NONE);
}

#[test]
#[should_panic(expected = "shard-1.json has a different bidding vulnerability")]
fn a_later_shard_cannot_change_the_bidding_vulnerability() {
    Shards::new("vulnerability", |dump| {
        dump.vulnerability = AbsoluteVulnerability::ALL;
    })
    .load();
}

#[test]
#[should_panic(expected = "shard-1.json has a different reference bidder")]
fn a_later_shard_cannot_change_the_reference_bidder() {
    Shards::new("reference", |dump| {
        dump.their_label = "other reference".into()
    })
    .load();
}

#[test]
#[should_panic(expected = "shard-1.json has a different bidder for our pair")]
fn a_later_shard_cannot_change_our_bidder() {
    Shards::new("ours", |dump| dump.our_label = "other bidder".into()).load();
}
