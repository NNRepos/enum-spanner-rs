use lib_regex;

use std::ops;

use super::super::mapping::{Mapping, SpannerEnumerator};

//  _   _       _              ____      _     _
// | \ | | __ _(_)_   _____   / ___|   _| |__ (_) ___
// |  \| |/ _` | \ \ / / _ \ | |  | | | | '_ \| |/ __|
// | |\  | (_| | |\ V /  __/ | |__| |_| | |_) | | (__
// |_| \_|\__,_|_| \_/ \___|  \____\__,_|_.__/|_|\___|
//

// TODO: this algorithm probably doesn't return matches aligned with the last
// character.

pub struct NaiveEnumCubic<'t> {
    regex: lib_regex::Regex,
    text: &'t str,
}

pub struct NaiveEnumCubicIterator<'t> {
    regex: lib_regex::Regex,
    text: &'t str,
    // Current state of the iteration
    char_iterator_start: std::str::CharIndices<'t>,
    char_iterator_end: std::str::CharIndices<'t>,
}

impl<'t> NaiveEnumCubic<'t> {
    pub fn new(regex: &str, text: &'t str) -> Result<NaiveEnumCubic<'t>, lib_regex::Error> {
        Ok(NaiveEnumCubic {
            regex: lib_regex::Regex::new(&format!("^{}$", regex))?,
            text,
        })
    }
}

impl<'t> SpannerEnumerator<'t> for NaiveEnumCubic<'t> {
    fn preprocess(&mut self) {}

    fn iter<'i>(&'i self) -> Box<dyn Iterator<Item = Mapping<'t>> + 'i> {
        Box::new(NaiveEnumCubicIterator {
            regex: self.regex.clone(),
            text: self.text,
            char_iterator_start: self.text.char_indices(),
            char_iterator_end: self.text.char_indices(),
        })
    }
}

impl<'t> Iterator for NaiveEnumCubicIterator<'t> {
    type Item = Mapping<'t>;

    fn next(&mut self) -> Option<Mapping<'t>> {
        while let Some((curr_start, _)) = self.char_iterator_start.next() {
            while let Some((curr_end, _)) = self.char_iterator_end.next() {
                let is_match = self.regex.is_match(&self.text[curr_start..curr_end]);

                if is_match {
                    return Some(Mapping::from_single_match(
                        self.text,
                        ops::Range {
                            start: curr_start,
                            end: curr_end,
                        },
                    ));
                }
            }

            // Move the start cursor to the next char.
            self.char_iterator_end = self.char_iterator_start.clone();
        }

        None
    }
}
