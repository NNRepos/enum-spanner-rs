//< Implementations for naive algorithms that output all matching subwords of a
//< regex.
//<
//< Note that these algorithms are not as powerful as other algorithms of this
//< project as they can't handle defined groups.

use std::ops;

use super::super::automaton::Automaton;
use super::super::regex;
use super::super::mapping::{Mapping,SpannerEnumerator};



//  _   _       _              ___                  _           _   _
// | \ | | __ _(_)_   _____   / _ \ _   _  __ _  __| |_ __ __ _| |_(_) ___
// |  \| |/ _` | \ \ / / _ \ | | | | | | |/ _` |/ _` | '__/ _` | __| |/ __|
// | |\  | (_| | |\ V /  __/ | |_| | |_| | (_| | (_| | | | (_| | |_| | (__
// |_| \_|\__,_|_| \_/ \___|  \__\_\\__,_|\__,_|\__,_|_|  \__,_|\__|_|\___|
//

// TODO: this algorithm probably doesn't return matches aligned with the last
// character.

// TODO: this algorithm doens't handle epsilon transitions (we just need to
// follow assignations after each step).

pub struct NaiveEnumQuadratic<'t> {
    automaton: Automaton,
    text:      &'t str,
}

pub struct NaiveEnumQuadraticIterator<'t> {
    automaton: Automaton,
    text:      &'t str,
    // Current state of the iteration
    curr_states:         Vec<bool>,
    char_iterator_end:   std::str::CharIndices<'t>,
    char_iterator_start: std::str::CharIndices<'t>,
}

impl<'t> NaiveEnumQuadratic<'t> {
    pub fn new(regex_str: &str, text: &'t str) -> NaiveEnumQuadratic<'t> {
        let automaton = regex::compile_raw(regex_str);

        NaiveEnumQuadratic {
            automaton,
            text,
        }
    }
}

impl<'t> SpannerEnumerator<'t> for NaiveEnumQuadratic<'t> {
    fn preprocess(&mut self) {}

    fn iter<'i>(&'i self) -> Box<dyn Iterator<Item = Mapping<'t>> +'i> {
        // Init automata states
        let mut initial_states = vec![false; self.automaton.nb_states];
        initial_states[self.automaton.get_initial()] = true;

        Box::new(NaiveEnumQuadraticIterator {
            automaton: self.automaton.clone(),
            text: self.text,
            curr_states: initial_states,
            char_iterator_end: self.text.char_indices(),
            char_iterator_start: self.text.char_indices(),
        })
    }
}

impl<'t> Iterator for NaiveEnumQuadraticIterator<'t> {
    type Item = Mapping<'t>;

    fn next(&mut self) -> Option<Mapping<'t>> {
        while let Some((curr_start, _)) = self.char_iterator_start.clone().next() {
            while let Some((curr_end, next_char)) = self.char_iterator_end.next() {
                // Check if current state results in a match
                if !self.curr_states.iter().any(|x| *x) {
                    break;
                }

                let is_match = self
                    .automaton
                    .finals
                    .iter()
                    .any(|state| self.curr_states[state]);

                // Read transitions and updates states in consequence
                let nb_states = self.automaton.nb_states;
                let adj = self.automaton.get_adj_for_char(next_char);

                let mut new_states = vec![false; nb_states];

                for i in 0..nb_states {
                    if self.curr_states[i] {
                        for &j in &adj[i] {
                            new_states[j] = true;
                        }
                    }
                }

                self.curr_states = new_states;

                // Output
                if is_match {
                    return Some(Mapping::from_single_match(
                        self.text,
                        ops::Range {
                            start: curr_start,
                            end:   curr_end,
                        },
                    ));
                }
            }

            // Move the start cursor to the next char.
            self.char_iterator_start.next();
            self.char_iterator_end = self.char_iterator_start.clone();

            // Reset automata states
            self.curr_states = vec![false; self.automaton.nb_states];
            self.curr_states[self.automaton.get_initial()] = true;
        }

        None
    }
}
