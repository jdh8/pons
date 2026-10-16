use super::*;
use std::path::PathBuf;

struct Shards(PathBuf);

impl Shards {
    fn empty(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("pons-dump-{name}-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    fn new(name: &str, change_second: impl FnOnce(&mut Dump)) -> Self {
        let dir = Self::empty(name);
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

#[test]
fn per_shard_arguments_may_differ() {
    Shards::new("per-shard-args", |dump| {
        dump.gen_args = ["--count", "9", "--port", "8086", "-o", "b/shard-1.json"]
            .into_iter()
            .chain(["--seed=2", "--start-board", "313", "--note", "assembled"])
            .map(String::from)
            .collect();
    })
    .load();
}

#[test]
#[should_panic(expected = "shard-1.json was generated with different arguments")]
fn a_later_shard_cannot_change_the_configuration() {
    Shards::new("config", |dump| {
        dump.gen_args.push("--ns-defensive-seam-split".into());
    })
    .load();
}

#[test]
fn shard_config_keeps_configuration_and_drops_per_shard_values() {
    let args: Vec<String> = [
        "--count",
        "8",
        "--seed",
        "1",
        "--port",
        "8085",
        "-t",
        "f",
        "-v",
        "none",
        "--note",
        "tier=f",
        "-o",
        "x.json",
        "--our-card=c.bbsa",
        "--output=y.json",
        "--flag",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(
        shard_config(&args),
        ["-t", "f", "-v", "none", "--our-card=c.bbsa", "--flag"]
    );
}

/// A shard written through [`write_atomically`] round-trips through
/// [`verify_shard`], which rejects a wrong board count, a wrong seed and a
/// truncated file; a failed write leaves neither the shard nor its temp file.
#[test]
fn written_shard_verifies_and_truncation_does_not() {
    let dir = Shards::empty("verify");
    let path = dir.0.join("shard-0.json");
    let path = path.to_str().unwrap();
    let dump = Dump {
        our_label: "us".into(),
        their_label: "them".into(),
        vulnerability: AbsoluteVulnerability::NONE,
        seed: Some(7),
        gen_args: Vec::new(),
        boards: Vec::new(),
    };
    write_atomically(path, |out| Ok(serde_json::to_writer(out, &dump)?)).unwrap();
    // The temp file was renamed away, not left beside the shard.
    assert_eq!(dir.names(), ["shard-0.json"]);

    verify_shard(path, 0, Some(7)).unwrap();
    verify_shard(path, 0, None).unwrap();
    assert!(verify_shard(path, 1, Some(7)).is_err(), "board count");
    assert!(verify_shard(path, 0, Some(8)).is_err(), "seed");

    let bytes = std::fs::read(path).unwrap();
    std::fs::write(path, &bytes[..bytes.len() - 1]).unwrap();
    assert!(verify_shard(path, 0, Some(7)).is_err(), "truncated");

    std::fs::remove_file(path).unwrap();
    let failed = write_atomically(path, |out| {
        out.write_all(b"{\"partial\":")?;
        anyhow::bail!("worker died mid-write")
    });
    assert!(failed.is_err());
    assert!(
        dir.names().is_empty(),
        "a failed write left {:?}",
        dir.names()
    );
}
