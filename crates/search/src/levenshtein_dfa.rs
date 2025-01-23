use std::sync::LazyLock;

use levenshtein_automata::{
    Distance,
    LevenshteinAutomatonBuilder,
    DFA,
};

/// Does a transposition count as one levenshtein distance or two?
const TRANSPOSITION_COST_ONE: bool = false;

static LEVENSHTEIN_DFAS: LazyLock<[LevenshteinAutomatonBuilder; 3]> = LazyLock::new(|| {
    [
        LevenshteinAutomatonBuilder::new(0, TRANSPOSITION_COST_ONE),
        LevenshteinAutomatonBuilder::new(1, TRANSPOSITION_COST_ONE),
        LevenshteinAutomatonBuilder::new(2, TRANSPOSITION_COST_ONE),
    ]
});

#[derive(Clone)]
pub struct LevenshteinDfaWrapper<'a>(pub &'a DFA);

// Implementation copied from Tantivy with some renaming, since the
// implementation is not public. https://github.com/quickwit-oss/tantivy/blob/bff7c58497964f947dc94e2e45dfe9962e1d10c3/src/query/fuzzy_query.rs
//
// https://github.com/quickwit-oss/tantivy/blob/main/LICENSE - MIT License
// Copyright (c) 2018 by the project authors, as listed in the AUTHORS file.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
impl tantivy::fst::automaton::Automaton for LevenshteinDfaWrapper<'_> {
    type State = u32;

    fn start(&self) -> Self::State {
        self.0.initial_state()
    }

    fn is_match(&self, state: &Self::State) -> bool {
        match self.0.distance(*state) {
            Distance::Exact(_) => true,
            Distance::AtLeast(_) => false,
        }
    }

    fn can_match(&self, state: &u32) -> bool {
        *state != levenshtein_automata::SINK_STATE
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        self.0.transition(*state, byte)
    }
}

#[fastrace::trace]
pub fn build_fuzzy_dfa(query: &str, distance: u8, prefix: bool) -> DFA {
    assert!(distance <= 2);
    let dfa_builder = &LEVENSHTEIN_DFAS[distance as usize];
    if prefix {
        dfa_builder.build_prefix_dfa(query)
    } else {
        dfa_builder.build_dfa(query)
    }
}
