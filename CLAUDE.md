# pons

This crate provides tools for analyzing and simulating hands in contract bridge.
Compared to [dds-bridge](https://crates.io/crates/dds-bridge), `pons` focuses
higher-level abstractions and utilities for working with bridge data, rather
than the core double-dummy solving algorithms.

Lately, most development goes into the `bidding` module.  Please feel free to
search online for authoritative sources on bridge bidding, and to ask me
questions about bridge bidding theory and practice.  I am not an expert (yet),
but I have been playing for a long time and have read a lot about it.  For
5-card major systems, please take a look at my
[Strawberry Polish Club](https://polish.club/).

After updating the codebase, please

- Format the code with `cargo fmt`.
- Run the tests with `cargo test --all-features`.
- Update [CHANGELOG.md](CHANGELOG.md) with a summary of the changes and their impact on users.
- Propose a clear and descriptive commit message.
