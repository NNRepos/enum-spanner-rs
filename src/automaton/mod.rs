pub mod atom;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::rc::Rc;
use bit_set::BitSet;

use super::mapping::Marker;

//     _         _                        _
//    / \  _   _| |_ ___  _ __ ___   __ _| |_ ___  _ __
//   / _ \| | | | __/ _ \| '_ ` _ \ / _` | __/ _ \| '_ \
//  / ___ \ |_| | || (_) | | | | | | (_| | || (_) | | | |
// /_/   \_\__,_|\__\___/|_| |_| |_|\__,_|\__\___/|_| |_|
//
#[derive(Clone, Debug)]
pub struct Automaton {
    pub nb_states:   usize,
    pub transitions: Vec<(usize, Rc<Label>, usize)>,
    pub finals:      BitSet,

    // Redundant caching structures
    adj: Vec<Vec<(Rc<Label>, usize)>>,
    adj_for_char: HashMap<char, Vec<Vec<usize>>>,
    adj_for_char_with_closure: HashMap<char, Vec<Vec<usize>>>,
    rev_adj_for_char_with_closure: HashMap<char, Vec<Vec<usize>>>,
    assignations: Vec<Vec<(Rc<Label>, usize)>>,
    rev_assignations: Vec<Vec<(Rc<Label>, usize)>>,
    closure_for_assignations: Vec<Vec<usize>>,
    closure_for_rev_assignations: Vec<Vec<usize>>,
	jump_states: BitSet,
}

impl Automaton {
    pub fn new<T, U>(nb_states: usize, transitions: T, finals: U) -> Automaton
    where
        T: Iterator<Item = (usize, Rc<Label>, usize)>,
        U: Iterator<Item = usize>,
    {
        let mut automaton = Automaton {
            nb_states,
            transitions: transitions.collect(),
            finals: finals.collect(),

            adj: Vec::new(),
            adj_for_char: HashMap::new(),
            adj_for_char_with_closure: HashMap::new(),
            rev_adj_for_char_with_closure: HashMap::new(),
            assignations: Vec::new(),
            rev_assignations: Vec::new(),
            closure_for_assignations: Vec::new(),
            closure_for_rev_assignations: Vec::new(),
			jump_states: BitSet::new(),
        };

        automaton.adj = automaton.init_adj();
        automaton.rev_assignations = automaton.init_rev_assignations();
        automaton.assignations = automaton.init_assignations();
        automaton.closure_for_assignations = automaton.init_closure_for_assignations();
        automaton.closure_for_rev_assignations = automaton.init_closure_for_rev_assignations();
		automaton.jump_states = automaton.init_jump_states();

        automaton
    }

    pub fn get_initial(&self) -> usize {
        0
    }

    pub fn get_nb_states(&self) -> usize {
        self.nb_states
    }

    pub fn get_adj(&self) -> &Vec<Vec<(Rc<Label>, usize)>> {
        &self.adj
    }

    /// Get the adjacency list representing transitions of the automaton that
    /// can be used when reading a given char.
    pub fn get_adj_for_char(&mut self, x: char) -> &Vec<Vec<usize>> {
        self.adj_for_char.get(&x).unwrap()
    }

    pub fn get_rev_adj_for_char_with_closure(&self, x: char) -> &Vec<Vec<usize>> {
        self.rev_adj_for_char_with_closure.get(&x).unwrap()
    }

    pub fn get_adj_for_char_with_closure(&mut self, x:char) -> &Vec<Vec<usize>> {
        let nb_states = self.get_nb_states();
        let adj_for_char = &mut self.adj_for_char;
        let transitions = &self.transitions;
        let closure_for_assignations = &self.closure_for_assignations;
        let closure_for_rev_assignations = &self.closure_for_rev_assignations;
        let adj_for_char_with_closure = &mut self.adj_for_char_with_closure;
        let rev_adj_for_char_with_closure = &mut self.rev_adj_for_char_with_closure;

        adj_for_char_with_closure.entry(x).or_insert_with(|| {
            let mut res = vec![Vec::new(); nb_states];
            let mut res_closure = vec![Vec::new(); nb_states];
            let mut res_rev_closure = vec![Vec::new(); nb_states];

            for (source, label, target) in transitions {
                if let Label::Atom(atom) = &**label {
                    if atom.is_match(&x) {
                        res[*source].push(*target);
                        res_closure[*source].push(*target);
                        res_rev_closure[*target].push(*source);

                        for &ttarget in &closure_for_assignations[*target] {
                            res_closure[*source].push(ttarget);
                        }

                        for &ssource in &closure_for_rev_assignations[*source] {
                            res_rev_closure[*target].push(ssource);
                        }
                    }
                }
            }

            for targets in &mut res_closure {
                targets.sort();
                targets.dedup();
            }

            for sources in &mut res_rev_closure {
                sources.sort();
                sources.dedup();
            }

            rev_adj_for_char_with_closure.insert(x,res_rev_closure);

            adj_for_char.insert(x,res);


            res_closure
        })
    }

    /// Get adjacency lists labeled with the corresponding marker for
    /// transitions labeled with an assignation.
    pub fn get_assignations(&self) -> &Vec<Vec<(Rc<Label>, usize)>> {
        &self.assignations
    }

    /// Get the reverse of assignations as defined in
    /// `Automata::get_assignations`.
    pub fn get_rev_assignations(&self) -> &Vec<Vec<(Rc<Label>, usize)>> {
        &self.rev_assignations
    }

    /// Get the closure as adjacency lists for transitions labeled with an
    /// assignation.
    pub fn get_closure_for_assignations(&self) -> &Vec<Vec<usize>> {
        &self.closure_for_assignations
    }

    /// Render the automaton as a dotfile for later rendering with graphviz.
    pub fn render(&self, filename: &str) -> std::io::Result<()> {
        let mut buf = File::create(filename)?;
        buf.write(b"digraph automaton {\n")?;

        // Use doublecircles for final states
        buf.write(b"\tnode [shape=doublecircle]\n")?;

        for state in &self.finals {
            let node = format!("\tq{}\n", state);
            buf.write(node.as_bytes())?;
        }

        // Draw edges
        buf.write(b"\n\tnode [shape=circle]\n")?;

        for (source, label, target) in &self.transitions {
            let mut label_str = format!("{}", label).escape_debug().to_string();

            if label_str.chars().count() > 10 {
                label_str = String::from("[...]");
            }

            let edge = format!("\tq{} -> q{} [label=\" {} \"]\n", source, target, label_str);
            buf.write(edge.as_bytes())?;
        }

        // Add an arrow towards initial state
        buf.write(b"\n\tnode [shape=point]\n")?;
        buf.write(b"\tbefore_q0 -> q0\n")?;

        buf.write(b"}\n")?;
        Ok(())
    }

    fn init_adj(&self) -> Vec<Vec<(Rc<Label>, usize)>> {
        let mut ret = vec![Vec::new(); self.nb_states];

        for (source, label, target) in &self.transitions {
            ret[*source].push((label.clone(), *target));
        }

        ret
    }

    fn init_assignations(&self) -> Vec<Vec<(Rc<Label>, usize)>> {
        // Compute adjacency list
        let mut adj = vec![Vec::new(); self.get_nb_states()];

        for (source, label, target) in &self.transitions {
            if let Label::Assignation(_) = **label {
				if source > target {
					panic!("Source must be smaller then target in transition ({},{},{})", source, label, target);
				}
                adj[*source].push((label.clone(), *target))
            }
        }

        adj
    }

    fn init_rev_assignations(&self) -> Vec<Vec<(Rc<Label>, usize)>> {
        // Compute adjacency list
        let mut adj = vec![Vec::new(); self.get_nb_states()];

        for (source, label, target) in &self.transitions {
            if let Label::Assignation(_) = **label {
                adj[*target].push((label.clone(), *source))
            }
        }

        adj
    }

    fn init_closure_for_assignations(&self) -> Vec<Vec<usize>> {
        // Compute adjacency list
        let assignations = self.get_assignations();
        let adj: Vec<Vec<usize>> = (0..self.get_nb_states())
            .map(|i| assignations[i].iter().map(|(_, j)| *j).collect())
            .collect();

        // Compute closure
        let mut closure = vec![Vec::new(); self.get_nb_states()];

        for state in 0..self.get_nb_states() {
            let mut heap = vec![state];
            let mut seen = HashSet::new();
            seen.insert(state);

            while let Some(source) = heap.pop() {
                for target in &adj[source] {
                    closure[state].push(*target);

                    if !seen.contains(target) {
                        heap.push(*target);
                        seen.insert(*target);
                    }
                }
            }
        }

        closure
    }

    fn init_closure_for_rev_assignations(&self) -> Vec<Vec<usize>> {
        let closure_for_assignations = &self.closure_for_assignations;
        let mut rev_closure = vec![Vec::new(); self.get_nb_states()];
        for (source,targets) in closure_for_assignations.iter().enumerate() {
            for &target in targets {
                rev_closure[target].push(source);
            }
        }

        rev_closure
    }

	pub fn get_jump_states(&self) -> &BitSet {
		&self.jump_states
	} 
	
	fn init_jump_states(&self) -> BitSet {
		self.transitions.clone().into_iter().filter_map(|(_,l,q)| match *l {
			Label::Assignation(_) => Some(q),
			Label::Atom(_) => None
		}).collect::<BitSet>()
	}
}

//  _          _          _
// | |    __ _| |__   ___| |
// | |   / _` | '_ \ / _ \ |
// | |__| (_| | |_) |  __/ |
// |_____\__,_|_.__/ \___|_|
//

#[derive(Debug)]
pub enum Label {
    Atom(atom::Atom),
    Assignation(Marker),
}

impl Label {
    pub fn get_marker(&self) -> Result<&Marker, &str> {
        match self {
            Label::Assignation(marker) => Ok(marker),
            Label::Atom(_) => Err("Can't get a marker out of an atom label."),
        }
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Label::Assignation(marker) => write!(f, "{}", marker),
            Label::Atom(atom) => write!(f, "{}", atom),
        }
    }
}
