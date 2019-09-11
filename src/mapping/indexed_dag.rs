use std::collections::{HashMap, HashSet, VecDeque};
use std::iter;

use super::super::automaton::Automaton;
use super::super::mapping::{Mapping, Marker};
use super::super::progress::Progress;
use super::jump::Jump;

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
			text.len() + 1,
			automaton.get_nb_states()
        );

        let closure_for_assignations = automaton.get_closure_for_assignations().clone();

        let chars: Vec<_> = text.chars().collect();
        let mut progress = Progress::from_iter(chars.into_iter())
            .auto_refresh(toggle_progress == ToggleProgress::Enabled);
        let mut curr_level = 0;

        while let Some(curr_char) = progress.next() {
            let adj_for_char = automaton.get_adj_for_char(curr_char);
            jump.init_next_level(adj_for_char, &closure_for_assignations);
            progress.extra_msg(format!("{} levels", jump.get_nb_levels()));

            curr_level += 1;

            if jump.is_disconnected() {
                break;
            }
        }

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

    pub fn get_nb_levels(&self) -> usize {
        self.jump.get_nb_levels()
    }

    fn next_level<'a>(&'a self, gamma: Vec<usize>) -> NextLevelIterator<'a> {
        let adj = self.automaton.get_rev_assignations();

        // Get list of variables that are part of the level.
        // UODO: It might still be slower to just using the list of all variables in the
        // automaton?
        let mut k = HashSet::new();
        let mut stack = gamma.clone();
        let mut marks = HashSet::new();

        for &x in &gamma {
            marks.insert(x);
        }

        while let Some(source) = stack.pop() {
            for (label, target) in &adj[source] {
                k.insert(label.get_marker().unwrap());

                if !marks.contains(target) {
                    marks.insert(*target);
                    stack.push(*target);
                }
            }
        }

        NextLevelIterator::explore(&self.automaton, k, gamma)
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
    stack:       Vec<(usize, Vec<usize>, Vec<(&'i Marker, usize)>)>,

    curr_level:      usize,
    curr_mapping:    Vec<(&'i Marker, usize)>,
    curr_next_level: NextLevelIterator<'i>,
}

impl<'i, 't> IndexedDagIterator<'i, 't> {
    fn init(indexed_dag: &'i IndexedDag<'t>) -> IndexedDagIterator<'i, 't> {
        let start = indexed_dag
            .jump
            .finals()
            .intersection(&indexed_dag.automaton.finals.iter().map(|x| *x).collect())
            .map(|x| *x)
            .collect();

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

                let mut new_mapping = self.curr_mapping.clone();
                for marker in s_p {
                    new_mapping.push((marker, self.curr_level));
                }

                if self.curr_level == 0
                    && new_gamma.contains(&self.indexed_dag.automaton.get_initial())
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
                    .jump(self.curr_level, new_gamma.into_iter())
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
    gamma: Vec<usize>,

    /// The current state of the iterator
    stack: Vec<(HashSet<&'a Marker>, HashSet<&'a Marker>)>,
}

impl<'a> NextLevelIterator<'a> {
    /// An empty iterator.
    fn empty(automaton: &'a Automaton) -> NextLevelIterator<'a> {
        NextLevelIterator {
            stack: Vec::new(), // Initialized with an empty stack to stop iteration instantly.
            automaton,
            expected_markers: Vec::default(),
            gamma: Vec::default(),
        }
    }

    /// Start the exporation from the input set of states `gamma`.
    fn explore(
        automaton: &'a Automaton,
        expected_markers: HashSet<&'a Marker>,
        gamma: Vec<usize>,
    ) -> NextLevelIterator<'a> {
        NextLevelIterator {
            automaton,
            expected_markers: expected_markers.into_iter().collect(),
            gamma,
            stack: vec![(HashSet::new(), HashSet::new())],
        }
    }

    fn follow_sp_sm(
        &self,
        gamma: &Vec<usize>,
        s_p: &HashSet<&Marker>,
        s_m: &HashSet<&Marker>,
    ) -> Vec<usize> {
        let adj = self.automaton.get_rev_assignations();
        let mut path_set: HashMap<usize, Option<HashSet<_>>> = HashMap::new();

        for &state in gamma {
            path_set.insert(state, Some(HashSet::new()));
        }

        // Check if two sets are incomparable
        let are_incomparable =
            |set1: &HashSet<_>, set2: &HashSet<_>| !set1.is_subset(&set2) && !set2.is_subset(&set1);

        // TODO: Consider writing this as a recursive function?
        let mut queue: VecDeque<_> = gamma.iter().cloned().collect();

        while let Some(source) = queue.pop_front() {
            for (label, target) in &adj[source] {
                let label = label.get_marker().unwrap();

                if s_m.contains(label) {
                    continue;
                }

                if !path_set.contains_key(target) {
                    queue.push_back(*target);
                }

                let mut new_ps = path_set[&source].clone().unwrap();

                if s_p.contains(label) {
                    new_ps.insert(label);
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
    type Item = (HashSet<&'a Marker>, Vec<usize>);

    fn next(&mut self) -> Option<(HashSet<&'a Marker>, Vec<usize>)> {
        while let Some((mut s_p, mut s_m)) = self.stack.pop() {
            let mut gamma2 = Some(self.follow_sp_sm(&self.gamma, &s_p, &s_m));

            if gamma2.as_ref().unwrap().is_empty() {
                continue;
            }

            while s_p.len() + s_m.len() < self.expected_markers.len() {
                let depth = s_p.len() + s_m.len();
                s_p.insert(self.expected_markers[depth]);
                gamma2 = Some(self.follow_sp_sm(&self.gamma, &s_p, &s_m));

                if !gamma2.as_ref().unwrap().is_empty() {
                    // If current pair Sp/Sm is feasible, add the other branch
                    // to the stack.
                    let mut new_s_p = s_p.clone();
                    let mut new_s_m = s_m.clone();
                    new_s_m.insert(self.expected_markers[depth]);
                    new_s_p.remove(self.expected_markers[depth]);
                    self.stack.push((new_s_p, new_s_m));
                } else {
                    // Overwise, the other branch has to be feasible.
                    s_p.remove(self.expected_markers[depth]);
                    s_m.insert(self.expected_markers[depth]);
                    gamma2 = None;
                }
            }

            let gamma2 = match gamma2 {
                None => self.follow_sp_sm(&self.gamma, &s_p, &s_m),
                Some(val) => val,
            };

            return Some((s_p, gamma2));
        }

        None
    }
}
