pub mod indexed_dag;
pub mod naive;

mod jump;
mod levelset;

use std::cmp;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::rc::Rc;

pub use indexed_dag::IndexedDag;

//  __  __                   _
// |  \/  | __ _ _ __  _ __ (_)_ __   __ _
// | |\/| |/ _` | '_ \| '_ \| | '_ \ / _` |
// | |  | | (_| | |_) | |_) | | | | | (_| |
// |_|  |_|\__,_| .__/| .__/|_|_| |_|\__, |
//              |_|   |_|            |___/

/// Map a set of variables to spans [i, i'> over a text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mapping<'t> {
    text: &'t str,
    maps: Vec<Option<(Variable, Range<usize>)>>,
}

impl<'t> Mapping<'t> {
    /// Returns a span that contains the whole matching area
    pub fn main_span(&self) -> Option<Range<usize>> {
        self.maps.iter().fold(None, |acc, range| match (&acc,range) {
            (acc, None) => acc.clone(),
            (None, Some((_,range))) => Some(range.clone()),
            (Some(acc_range), Some((_,range))) => Some(Range {
                start: cmp::min(range.start, acc_range.start),
                end:   cmp::max(range.end, acc_range.end),
            }),
        })
    }

    pub fn iter_groups(&self) -> impl Iterator<Item = (&str, Range<usize>)> {
        self.maps
            .iter()
            .filter_map(|x| match x {
                Some((key, range)) => Some((key.get_name(), range.clone())),
                None => None
            })
    }

    pub fn iter_groups_text(&self) -> impl Iterator<Item = (&str, &str)> {
        self.maps
            .iter()
            .filter_map(move |x| match x {
                Some((key, range)) => Some((key.get_name(), &self.text[range.clone()])),
                None => None
            })
    }

    /// Return a canonical mapping for a classic semantic with no group, which
    /// will assign the whole match to a group called "match".
    pub fn from_single_match(text: &'t str, range: Range<usize>) -> Mapping<'t> {
        let maps: Vec<Option<(Variable, Range<usize>)>> = vec![Some((Variable::new("match".to_string(), 0), range))];
        Mapping { text, maps }
    }

    pub fn from_markers<T>(text: &'t str, marker_assigns: T, num_vars: usize) -> Mapping<'t>
    where
        T: Iterator<Item = (Marker, usize)>,
    {
        let mut maps: Vec<Option<(Variable, Range<usize>)>> = vec![None;num_vars];

        for (marker, pos) in marker_assigns {
            let span = match &maps[marker.variable().get_id()] {
                None => std::usize::MAX..std::usize::MAX,
                Some((_,x)) => x.clone(),
            };

            maps[marker.variable().get_id()] = Some((marker.variable().clone(),match marker {
                Marker::Open(_) =>  pos..span.end,
                Marker::Close(_) => span.start..pos,
            }));
        }

        Mapping { text, maps }
    }
}


impl<'t> fmt::Display for Mapping<'t> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for span in self.maps.iter() {
            match span {
                Some((var, range)) => { write!(f, "{}: ({}, {}) ", var, range.start, range.end)?; },
                None => {},
            }
        }

        Ok(())
    }
}

impl<'t> std::hash::Hash for Mapping<'t> {
    fn hash<'m, H: Hasher>(&'m self, state: &mut H) {
        for assignment in &self.maps {
            assignment.hash(state);
        }
    }
}
 



// __     __         _       _     _
// \ \   / /_ _ _ __(_) __ _| |__ | | ___
//  \ \ / / _` | '__| |/ _` | '_ \| |/ _ \
//   \ V / (_| | |  | | (_| | |_) | |  __/
//    \_/ \__,_|_|  |_|\__,_|_.__/|_|\___|
//

#[derive(Clone, Debug, PartialOrd, Ord)]
pub struct Variable {
    id:   usize,
    name: String,
}

impl Variable {
    pub fn new(name: String, id: usize) -> Variable {
        Variable { id, name }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

	pub fn get_id(&self) -> usize {
		self.id
	}
}

impl Hash for Variable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Variable {}
impl PartialEq for Variable {
    fn eq(&self, other: &Variable) -> bool {
        self.id == other.id
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

//  __  __            _
// |  \/  | __ _ _ __| | _____ _ __
// | |\/| |/ _` | '__| |/ / _ \ '__|
// | |  | | (_| | |  |   <  __/ |
// |_|  |_|\__,_|_|  |_|\_\___|_|
//
#[derive(Clone, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum Marker {
    Open(Rc<Variable>),
    Close(Rc<Variable>),
}

impl Marker {
    pub fn variable(&self) -> &Variable {
        match self {
            Marker::Open(var) | Marker::Close(var) => var,
        }
    }

	pub fn get_id(&self) -> usize {
		match self {
			Marker::Open(var) => var.get_id()*2,
			Marker::Close(var) => var.get_id()*2+1,
		}
	}
}

impl fmt::Debug for Marker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Marker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Marker::Open(var) => write!(f, "⊢{}", var),
            Marker::Close(var) => write!(f, "{}⊣", var),
        }
    }
}

//  _____         _
// |_   _|__  ___| |_ ___
//   | |/ _ \/ __| __/ __|
//   | |  __/\__ \ |_\__ \
//   |_|\___||___/\__|___/
//

#[cfg(test)]
mod tests;
