pub mod indexed_dag;

#[cfg(test)]
pub mod naive;

mod jump;
mod levelset;

use std::cmp::Ord;
use std::collections::HashMap;
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
#[derive(Debug, Eq, PartialEq)]
pub struct Mapping<'t> {
    text: &'t str,
    maps: HashMap<Variable, Range<usize>>,
}

impl<'t> Mapping<'t> {
    pub fn iter_groups(&self) -> impl Iterator<Item = (&str, Range<usize>)> {
        self.maps
            .iter()
            .map(|(key, range)| (key.get_name(), range.clone()))
    }

    pub fn iter_groups_text(&self) -> impl Iterator<Item = (&str, &str)> {
        self.maps
            .iter()
            .map(move |(key, range)| (key.get_name(), &self.text[range.clone()]))
    }

    pub fn from_markers<T>(text: &'t str, marker_assigns: T) -> Mapping<'t>
    where
        T: Iterator<Item = (Marker, usize)>,
    {
        let mut dict: HashMap<Variable, (Option<usize>, Option<usize>)> = HashMap::new();

        for (marker, pos) in marker_assigns {
            let span = match dict.get(marker.variable()) {
                None => (None, None),
                Some(x) => *x,
            };

            let span = match marker {
                Marker::Open(_) => match span.0 {
                    None => (Some(pos), span.1),
                    Some(old_pos) => panic!(
                        "Can't assign {} at position {}, already assigned to {}",
                        marker, pos, old_pos
                    ),
                },
                Marker::Close(_) => match span.1 {
                    None => (span.0, Some(pos)),
                    Some(old_pos) => panic!(
                        "Can't assign {} at position {}, already assigned to {}",
                        marker, pos, old_pos
                    ),
                },
            };

            dict.insert(marker.variable().clone(), span);
        }

        let maps = dict
            .into_iter()
            .map(|(key, span)| match span {
                (Some(i), Some(j)) if i <= j => (key, i..j),
                _ => panic!("Invalid mapping ordering"),
            })
            .collect();

        Mapping { text, maps }
    }
}

impl<'t> std::hash::Hash for Mapping<'t> {
    fn hash<'m, H: Hasher>(&'m self, state: &mut H) {
        self.text.hash(state);

        let mut assignments: Vec<_> = self.maps.iter().collect();
        assignments.sort_by(|&a, &b| {
            let key = |x: (&'m Variable, &Range<usize>)| (x.0, x.1.start, x.1.end);
            key(a).cmp(&key(b))
        });

        for assignment in assignments {
            assignment.hash(state);
        }
    }
}

impl<'t> fmt::Display for Mapping<'t> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (var, range) in self.maps.iter() {
            // write!(f, "{}: {} ", var, &self.text[*start..*end]).unwrap();
            write!(f, "{}: ({}, {}) ", var, range.start, range.end)?;
        }

        Ok(())
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
    id: u64,
    name: String,
}

impl Variable {
    pub fn new(name: String, id: u64) -> Variable {
        Variable { id, name }
    }

    pub fn get_name(&self) -> &str {
        &self.name
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
