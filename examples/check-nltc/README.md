Check validity of NLTC

Usage: `cargo run --example check-nltc [n]`  
where `n` is the number of deals to analyze  
(default: 100).

This example generates `n` random deals and find their best suit contracts.
Hand evaluations of the declaring side are compared with the optimal suit
tricks.  See [`EVALUATORS`] in the source code for the list of evaluators.