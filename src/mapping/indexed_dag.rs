use std::collections::{HashMap, VecDeque};
use std::iter;

use super::super::automaton::Automaton;
use super::super::mapping::{Mapping, Marker};
use super::super::progress::Progress;
use super::jump::Jump;
use bit_set::BitSet;

//  ___           _                   _ ____
// |_ _|_ __   __| | _____  _____  __| |  _ \  __ _  __ _
//  | || '_ \ / _` |/ _ \ \/ / _ \/ _` | | | |/ _` |/ _` |
//  | || | | | (_| |  __/>  <  __/ (_| | |_| | (_| | (_| |
// |___|_| |_|\__,_|\___/_/\_\___|\__,_|____/ \__,_|\__, |
//                                                  |___/

/// DAG built from the product automaton of a variable automaton and a text.
///
/// The structure allows to enumerate efficiently  all the distinct matches of
/// the input automata over the input text (polynomial preprocessing and
/// constant delay iteration).
pub struct IndexedDag<'t> {
    automaton:    Automaton,
    text:         &'t str,
    jump:         Jump,
    char_offsets: Vec<usize>,
}

#[derive(Eq, PartialEq)]
pub enum ToggleProgress {
    Enabled,
    Disabled,
}

impl<'t> IndexedDag<'t> {
    /// Compute the index of matches of an automaton over input text.
    pub fn compile(
        mut automaton: Automaton,
        text: &str,
        toggle_progress: ToggleProgress,
    ) -> IndexedDag {
        // Index utf8 chars, the ith char being represented by
        // `text[char_offsets[i]..char_offsets[i+1]]`
        let char_offsets = text
            .char_indices()
            .map(|(index, _)| index)
            .chain(iter::once(text.len()))
            .collect();

        // Compute the jump function
        let mut jump = Jump::new(
            iter::once(automaton.get_initial()),
            automaton.get_closure_for_assignations(),
			automaton.get_jump_states(),
			text.len() + 1,
			automaton.get_nb_states()
        );

        let closure_for_assignations = automaton.get_closure_for_assignations().clone();

        let chars: Vec<_> = text.chars().collect();
        let mut progress = Progress::from_iter(chars.into_iter())
            .auto_refresh(toggle_progress == ToggleProgress::Enabled);

        while let Some(curr_char) = progress.next() {
            let adj_for_char = automaton.get_adj_for_char(curr_char);
            jump.init_next_level(adj_for_char, &closure_for_assignations);

            if jump.is_disconnected() {
                break;
            }
        }

//		println!("Levelset: {:#?}", jump);

		jump.trim_last_level(&automaton.finals, &closure_for_assignations);
	

		if !jump.is_disconnected() {

//		println!("Levelset: {:#?}", jump);
			
			let chars: Vec<_> = text.chars().collect();
			let nb_levels = chars.len();
			let mut level = nb_levels;
	        let mut progress = Progress::from_iter(chars.into_iter().rev())
   	        .auto_refresh(toggle_progress == ToggleProgress::Enabled);

            while let Some(curr_char) = progress.next() {
                let adj_for_char = automaton.get_adj_for_char(curr_char);
                jump.trim_level(level, adj_for_char, &closure_for_assignations);
                level -= 1;
            }

//		println!("Levelset: {:#?}", jump);

        let chars: Vec<_> = text.chars().collect();
        let mut progress = Progress::from_iter(chars.into_iter())
            .auto_refresh(toggle_progress == ToggleProgress::Enabled);
		let mut level = 1;

        	while let Some(curr_char) = progress.next() {
            	let adj_for_char = automaton.get_adj_for_char(curr_char);
            	jump.init_reach(level, adj_for_char, &closure_for_assignations);
				level+=1;
	        }
		}

//		println!("Levelset: {:#?}", jump);

        IndexedDag {
            automaton,
            text,
            jump,
            char_offsets,
        }
    }

    pub fn iter<'i>(&'i self) -> impl Iterator<Item = Mapping<'t>> + 'i {
        IndexedDagIterator::init(self)
    }

    fn next_level<'a>(&'a self, gamma: BitSet) -> NextLevelIterator<'a> {
        let adj = self.automaton.get_rev_assignations();

        // Get list of variables that are part of the level.
        // UODO: It might still be slower to just using the list of all variables in the
        // automaton?
        let mut k = BitSet::new();
		let mut expected_markers = Vec::<&'a Marker>::new();
        let mut states = gamma.clone();
    	let mut new_states = gamma.clone();


		while !new_states.is_empty() {
			let source=new_states.iter().next().unwrap();
			new_states.remove(source);
           	for (label, target) in &adj[source] {
				let label_id = label.get_marker().unwrap().get_id();
				if !k.contains(label_id) {
                   	expected_markers.push(&label.get_marker().unwrap());
                   	k.insert(label_id);
				}
				if !states.contains(*target) {
           			states.insert(*target);
					new_states.insert(*target);
				}
           	}
		}

        NextLevelIterator::explore(&self.automaton, expected_markers, gamma)
    }
}

//  ___           _                   _
// |_ _|_ __   __| | _____  _____  __| |
//  | || '_ \ / _` |/ _ \ \/ / _ \/ _` |
//  | || | | | (_| |  __/>  <  __/ (_| |
// |___|_| |_|\__,_|\___/_/\_\___|\__,_|
//  ____
// |  _ \  __ _  __ _
// | | | |/ _` |/ _` |
// | |_| | (_| | (_| |
// |____/ \__,_|\__, |
//              |___/

struct IndexedDagIterator<'i, 't> {
    indexed_dag: &'i IndexedDag<'t>,
    stack:       Vec<(usize, BitSet, Vec<(&'i Marker, usize)>)>,

    curr_level:      usize,
    curr_mapping:    Vec<(&'i Marker, usize)>,
    curr_next_level: NextLevelIterator<'i>,
}

impl<'i, 't> IndexedDagIterator<'i, 't> {
    fn init(indexed_dag: &'i IndexedDag<'t>) -> IndexedDagIterator<'i, 't> {
        let mut start = indexed_dag
            .jump
            .finals().clone();
		start.intersect_with(&indexed_dag.automaton.finals);

        IndexedDagIterator {
            indexed_dag,
            stack: vec![(indexed_dag.text.chars().count(), start, Vec::new())],

            // `curr_next_level` is initialized empty, thus theses values will
            // be replaced before the first iteration.
            curr_next_level: NextLevelIterator::empty(&indexed_dag.automaton),
            curr_level: usize::default(),
            curr_mapping: Vec::default(),
        }
    }
}

impl<'i, 't> Iterator for IndexedDagIterator<'i, 't> {
    type Item = Mapping<'t>;

    fn next(&mut self) -> Option<Mapping<'t>> {
        loop {
            // First, consume curr_next_level.
            while let Some((s_p, new_gamma)) = self.curr_next_level.next() {
                if new_gamma.is_empty() {
                    continue;
                }

//				print!("NLI output: ");
//				for m in s_p.iter() {
//					print!("{} ", m);
//				}
//				for q in new_gamma.iter() {
//					print!{"q{} ", q};
//				}
//				println!("");

                let mut new_mapping = self.curr_mapping.clone();
                for marker in s_p {
                    new_mapping.push((marker, self.curr_level));
                }

                if self.curr_level == 0
                    && new_gamma.contains(self.indexed_dag.automaton.get_initial())
                {
                    // Re-align level indexes with utf8 coding
                    let aligned_markers = new_mapping
                        .into_iter()
                        .map(|(marker, pos)| (marker.clone(), self.indexed_dag.char_offsets[pos]));

                    // Create the new mapping
                    return Some(Mapping::from_markers(
                        self.indexed_dag.text,
                        aligned_markers,
                    ));
                } else if let Some((jump_level, jump_gamma)) = self
                    .indexed_dag
                    .jump
                    .jump(self.curr_level, new_gamma)
                {
                    if !jump_gamma.is_empty() {
                        self.stack.push((jump_level, jump_gamma, new_mapping));
                    }
                }
            }

            // Overwise, read next element of the stack and init the new
            // `curr_next_level` before restarting the process.
            match self.stack.pop() {
                None => return None,
                Some((level, gamma, mapping)) => {
                    self.curr_level = level;
                    self.curr_mapping = mapping;
                    self.curr_next_level = self.indexed_dag.next_level(gamma)
                }
            }
        }
    }
}

//  _   _           _   _                   _
// | \ | | _____  _| |_| |    _____   _____| |
// |  \| |/ _ \ \/ / __| |   / _ \ \ / / _ \ |
// | |\  |  __/>  <| |_| |__|  __/\ V /  __/ |
// |_| \_|\___/_/\_\\__|_____\___| \_/ \___|_|
//  ___ _                 _
// |_ _| |_ ___ _ __ __ _| |_ ___  _ __
//  | || __/ _ \ '__/ _` | __/ _ \| '__|
//  | || ||  __/ | | (_| | || (_) | |
// |___|\__\___|_|  \__,_|\__\___/|_|
//

/// Explore all feasible variable associations in a level from a set of states
/// and resulting possible states reached for theses associations.
struct NextLevelIterator<'a> {
    automaton: &'a Automaton,

    /// Set of markers that can be reached in this level.
    expected_markers: Vec<&'a Marker>,

    /// Set of states we start the run from.
    gamma: BitSet,

    /// The current state of the iterator
    stack: Vec<(BitSet, BitSet, Vec<&'a Marker>)>,

	/// finished enumerating
	done: bool,

	/// the only partial mapping to return is the empty one
	almost_done: bool,
}

impl<'a> NextLevelIterator<'a> {
    /// An empty iterator.
    fn empty(automaton: &'a Automaton) -> NextLevelIterator<'a> {
        NextLevelIterator {
            stack: Vec::new(), // Initialized with an empty stack to stop iteration instantly.
            automaton,
            expected_markers: Vec::new(),
            gamma: BitSet::new(),
			done: true,
			almost_done: true,
        }
    }

    /// Start the exporation from the input set of states `gamma`.
    fn explore(
        automaton: &'a Automaton,
        expected_markers: Vec<&'a Marker>,
        gamma: BitSet,
    ) -> NextLevelIterator<'a> {
        NextLevelIterator {
            automaton,
            expected_markers: expected_markers.into_iter().collect(),
            gamma,
            stack: vec![(BitSet::new(), BitSet::new(), Vec::new())],
			done: false,
			almost_done: false,
        }
    }

    fn follow_sp_sm(
        &self,
        gamma: &BitSet,
        s_p: &BitSet,
        s_m: &BitSet,
    ) -> BitSet {
        let adj = self.automaton.get_rev_assignations();
        let mut path_set: HashMap<usize, Option<BitSet>> = HashMap::new();

        for state in gamma.iter() {
            path_set.insert(state, Some(BitSet::new()));
        }

        // Check if two sets are incomparable
        let are_incomparable =
            |set1: &BitSet, set2: &BitSet| !set1.is_subset(&set2) && !set2.is_subset(&set1);

        // states 
        let mut queue: VecDeque<_> = gamma.iter().collect();

        while let Some(source) = queue.pop_front() {
            for (label, target) in &adj[source] {
                let label = label.get_marker().unwrap();

                if s_m.contains(label.get_id()) {
                    continue;
                }

                if !path_set.contains_key(target) {
                    queue.push_back(*target);
                }

                let mut new_ps = path_set[&source].clone().unwrap();

                if s_p.contains(label.get_id()) {
                    new_ps.insert(label.get_id());
                }

                path_set
                    .entry(*target)
                    .and_modify(|entry| {
                        if let Some(old_ps) = entry {
                            if are_incomparable(&new_ps, old_ps) {
                                *entry = None;
                            } else {
                                *entry = Some(new_ps.clone());
                            }
                        }
                    })
                    .or_insert(Some(new_ps));
            }
        }

        path_set
            .iter()
            .filter_map(|(vertex, vertex_ps)| match vertex_ps {
                Some(vertex_ps) if vertex_ps.len() == s_p.len() => Some(*vertex),
                _ => None,
            })
            .collect()
    }
}

impl<'a> Iterator for NextLevelIterator<'a> {
    type Item = (Vec<&'a Marker>, BitSet);

    fn next(&mut self) -> Option<(Vec<&'a Marker>, BitSet)> {
		if self.done {
			return None
		}

		if self.almost_done || self.expected_markers.is_empty() {
			self.done = true;
			return Some((Vec::new(),self.gamma.clone()))
		}

		if self.expected_markers.len()==1 {
			let mut markers = Vec::new();
	        let adj = self.automaton.get_rev_assignations();
			let mut gamma2 = BitSet::new();
			let marker = self.expected_markers[0];
			let gamma = self.gamma.clone();

			markers.push(marker);

            for source in gamma.iter() {
				for (_, target) in &adj[source] {
                    gamma2.insert(*target);
                }
			}

			self.almost_done = true;

			return Some((markers, gamma2))
		}


        while let Some((mut s_p, mut s_m, mut markers)) = self.stack.pop() {
            let mut gamma2 = Some(self.follow_sp_sm(&self.gamma, &s_p, &s_m));

            if gamma2.as_ref().unwrap().is_empty() {
                continue;
            }

            while s_p.len() + s_m.len() < self.expected_markers.len() {
                let depth = s_p.len() + s_m.len();
				let next_marker = self.expected_markers[depth].get_id();
                s_p.insert(next_marker);
                gamma2 = Some(self.follow_sp_sm(&self.gamma, &s_p, &s_m));


                if !gamma2.as_ref().unwrap().is_empty() {
                    // If current pair Sp/Sm is feasible, add the other branch
                    // to the stack.
                    let mut new_s_p = s_p.clone();
                    let mut new_s_m = s_m.clone();
                    new_s_m.insert(next_marker);
                    new_s_p.remove(next_marker);
					let new_markers = markers.clone();
                    self.stack.push((new_s_p, new_s_m, new_markers));

					//only modify after unmodified markers is pushed to the stack
					markers.push(&self.expected_markers[depth]);
                } else {
                    // Overwise, the other branch has to be feasible.
                    s_p.remove(next_marker);
                    s_m.insert(next_marker);
                    gamma2 = None;
                }
            }

            let gamma2 = match gamma2 {
                None => self.follow_sp_sm(&self.gamma, &s_p, &s_m),
                Some(val) => val,
            };

            return Some((markers, gamma2));
        }

        None
    }
}
