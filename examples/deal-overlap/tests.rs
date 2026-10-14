#[test]
#[should_panic(expected = "no deals in seeded:1:0")]
fn rejects_empty_source() {
    super::deals("seeded:1:0");
}
