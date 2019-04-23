pub mod naive;

use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use rand;

//  __  __                   _
// |  \/  | __ _ _ __  _ __ (_)_ __   __ _
// | |\/| |/ _` | '_ \| '_ \| | '_ \ / _` |
// | |  | | (_| | |_) | |_) | | | | | (_| |
// |_|  |_|\__,_| .__/| .__/|_|_| |_|\__, |
//              |_|   |_|            |___/

/// Map a set of variables to spans [i, i'> over a text.
#[derive(Debug)]
pub struct Mapping<'a> {
    text: &'a str,
    maps: HashMap<Variable, (usize, usize)>,
}

impl<'a> fmt::Display for Mapping<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (var, (start, end)) in self.maps.iter() {
            write!(f, "{}: {} ", var, &self.text[*start..*end]).unwrap();
        }

        Ok(())
    }
}

impl<'a> Mapping<'a> {
    pub fn from_markers<T>(text: &'a str, marker_assigns: T) -> Mapping
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

            dict.insert(marker.into_variable(), span);
        }

        let maps = dict
            .into_iter()
            .map(|(key, span)| match span {
                (Some(i), Some(j)) if i <= j => (key, (i, j)),
                _ => panic!("Invalid mapping ordering"),
            })
            .collect();

        Mapping { text, maps }
    }
}

// __     __         _       _     _
// \ \   / /_ _ _ __(_) __ _| |__ | | ___
//  \ \ / / _` | '__| |/ _` | '_ \| |/ _ \
//   \ V / (_| | |  | | (_| | |_) | |  __/
//    \_/ \__,_|_|  |_|\__,_|_.__/|_|\___|
//

#[derive(Clone, Debug)]
pub struct Variable {
    id: u64,
    name: String,
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
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

impl Variable {
    pub fn new(name: String) -> Variable {
        Variable {
            id: rand::random(),
            name: name,
        }
    }
}

//  __  __            _
// |  \/  | __ _ _ __| | _____ _ __
// | |\/| |/ _` | '__| |/ / _ \ '__|
// | |  | | (_| | |  |   <  __/ |
// |_|  |_|\__,_|_|  |_|\_\___|_|
//

#[derive(Clone, Debug)]
pub enum Marker {
    Close(Variable),
    Open(Variable),
}

impl Marker {
    pub fn variable(&self) -> &Variable {
        match self {
            Marker::Open(var) | Marker::Close(var) => var,
        }
    }

    pub fn into_variable(self) -> Variable {
        match self {
            Marker::Open(var) | Marker::Close(var) => var,
        }
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