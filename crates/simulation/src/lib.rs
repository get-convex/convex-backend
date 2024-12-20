#![feature(async_closure)]
#![feature(lazy_cell)]
#![feature(try_blocks)]

#[cfg(test)]
mod tests;

#[cfg(test)]
pub mod test_helpers;

// TODO:
//
// models
// - [ ] mutations executed exactly once
// - [ ] convergence with consistency check
// - [ ] causal+ consistency
//
// performance
// - [ ] load fewer JS files
// - [ ] don't load any JS files and use rust server tx
// - [ ] reuse application across multiple tests
// - [ ] stack overflows?
//
// References:
// - elle: https://github.com/jepsen-io/elle
// - list append from https://www.youtube.com/watch?v=ecZp6cWhDjg
// - porcupine: https://github.com/anishathalye/porcupine
// - TiPocket: https://github.com/pingcap/tipocket
