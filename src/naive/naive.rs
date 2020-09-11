use std::collections::HashSet;
use std::str::CharIndices;

use super::super::automaton::{Automaton, Label};
use super::super::mapping::{Mapping, Marker, SpannerEnumerator};

/// Enumerate all the matches of a variable automata over a text.
///
/// ** For this naive implementation, the runtime and delay are only bounded by
/// the number of accepting runs in the NFA. **
pub struct NaiveEnum<'t> {
    automaton: &'t Automaton,
    text: &'t str,
}

pub struct NaiveEnumIterator<'t> {
    automaton: &'t Automaton,
    text: &'t str,
    num_vars: usize,

    /// Holds current positions of the runs as a stack of:
    ///  - current state on the automata
    ///  - current index on the word
    ///  - assignations that have been done so far
    curr_state: Vec<(usize, CharIndices<'t>, Vec<(&'t Marker, usize)>)>,

    /// Keep track of already outputed values
    curr_output: HashSet<Mapping<'t>>,
}

impl<'t> NaiveEnum<'t> {
    pub fn new(automaton: &'t Automaton, text: &'t str) -> NaiveEnum<'t> {
        NaiveEnum { automaton, text }
    }
}

impl<'t> SpannerEnumerator<'t> for NaiveEnum<'t> {
    fn iter<'i>(&'i self) -> Box<dyn Iterator<Item = Mapping<'t>> + 'i> {
        Box::new(NaiveEnumIterator {
            automaton: self.automaton,
            num_vars: self.automaton.num_vars(),
            text: self.text,
            curr_state: vec![(0, self.text.char_indices(), Vec::new())],
            curr_output: HashSet::new(),
        })
    }

    /// Do nothing
    fn preprocess(&mut self) {}
}

impl<'t> Iterator for NaiveEnumIterator<'t> {
    type Item = Mapping<'t>;

    fn next(&mut self) -> Option<Mapping<'t>> {
        while let Some((state, index, assigns)) = self.curr_state.pop() {
            let curr_char = index.clone().next();

            for (label, target) in &self.automaton.get_adj()[state] {
                match **label {
                    Label::Atom(ref atom) if curr_char != None => {
                        if let Some((_, curr_char)) = curr_char {
                            if !atom.is_match(&curr_char) {
                                continue;
                            }
                        }
                        let mut new_index = index.clone();
                        new_index.next();

                        self.curr_state.push((*target, new_index, assigns.clone()));
                    }
                    Label::Assignation(ref marker) => {
                        let mut new_assigns = assigns.clone();
                        let pos = match curr_char {
                            None => self.text.len(),
                            Some((pos, _)) => pos,
                        };
                        new_assigns.push((marker, pos));
                        self.curr_state.push((*target, index.clone(), new_assigns));
                    }
                    _ => (),
                }
            }

            if curr_char == None && self.automaton.finals.contains(state) {
                let mapping = Mapping::from_markers(
                    self.text,
                    assigns
                        .into_iter()
                        .map(|(marker, pos)| (marker.clone(), pos)),
                    self.num_vars,
                );

                if !self.curr_output.contains(&mapping) {
                    self.curr_output.insert(mapping.clone());
                    return Some(mapping);
                }
            }
        }

        None
    }
}
