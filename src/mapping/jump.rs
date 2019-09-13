use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::fmt;
use bit_vec::BitVec;
use bit_set::BitSet;

use super::super::matrix::Matrix;
use super::levelset::LevelSet;

//      _
//     | |_   _ _ __ ___  _ __
//  _  | | | | | '_ ` _ \| '_ \
// | |_| | |_| | | | | | | |_) |
//  \___/ \__,_|_| |_| |_| .__/
//                       |_|

/// Generic Jump function inside a product DAG.
///
/// The DAG will be built layer by layer by specifying the adjacency matrix from
/// one level to the next one, an adjancency matrix can specify the structure
/// inside of a level, made of 'assignation edges'. The goal of the structure is
/// to be able to be able to navigate quickly from the last to the first layer
/// by being able to skip any path that do not contain any assignation edges.
pub struct Jump {
    /// Represent levels in the levelset, which will be built on top of one
    /// another.
    levelset: LevelSet,
    /// Last level that was built.
    last_level: usize,

    /// Set of vertices in the last level that can't be jumped since it has an ingoing
    /// non-jumpable edge.
    nonjump_vertices: BitSet,

    /// Closest level where an assignation is done accessible from any node.
    jl: Vec<Vec<usize>>,

    /// Set of levels accessible from any level using `jl`.
    rlevel: Vec<Vec<usize>>,
    /// For any pair of level `(i, j)` such that i at the k-th position in `rlevel[j]`,
    /// `reach[i][k]` is the accessibility matrix of vertices from level i
    /// to level j.
    reach: Vec<Vec<Matrix>>,

	num_vertices: usize,
}

impl Jump {
    pub fn new<T>(initial_level: T, nonjump_adj: &Vec<Vec<usize>>, num_levels: usize, num_vertices: usize) -> Jump
    where
        T: Iterator<Item = usize>,
    {
        let mut jump = Jump {
            levelset:            LevelSet::new(num_levels, num_vertices),
            last_level:          0,
            nonjump_vertices:    BitSet::with_capacity(num_vertices),
//            count_ingoing_jumps: HashMap::new(),
            jl:                  Vec::with_capacity(num_levels),
            rlevel:              Vec::with_capacity(num_levels),
            reach:               Vec::with_capacity(num_levels),
			num_vertices:		 num_vertices,
        };

		jump.levelset.add_level();


        jump.rlevel.push(Vec::new());
		jump.reach.push(Vec::new());
		jump.jl.push(vec![0; num_vertices]);

        for state in initial_level {
            jump.levelset.register(state, 0);
//            jump.jl[0]].insert((0, state), 0);
        }

        // Init first level
        jump.extend_level(0, nonjump_adj);

        jump
    }

    /// Compute next level given the adjacency list of jumpable edges from
    /// current level to the next one and adjacency list of non-jumpable
    /// edges inside the next level.
    pub fn init_next_level(&mut self, jump_adj: &Vec<Vec<usize>>, nonjump_adj: &Vec<Vec<usize>>) {
		let nonjump_vertices = &self.nonjump_vertices;
        let levelset = &mut self.levelset;
        let jl = &mut self.jl;

        let last_level = self.last_level;
        let next_level = self.last_level + 1;

		levelset.add_level();
		jl.push(vec![0;self.num_vertices]);

        // NOTE: this clone is only necessary for the borrow checker.
        let last_level_vertices = levelset.get_level(last_level).clone();

        // Register jumpable transitions from this level to the next one
        for source in last_level_vertices.iter() {
            // Notice that `source_jl` can be 0, however, if it is not in
            // nonjump_vertices it is sure that it is not 0 since it was
            // necessary added by following an atomic transition.
            let source_jl = jl[last_level][source];

            for &target in &jump_adj[source] {
		        levelset.register(next_level, target);

                if nonjump_vertices.contains(source) {
	                jl[next_level][target]=last_level;
				} else {
                    jl[next_level][target]=max(source_jl, jl[next_level][target]);
                }
            }
        }

        // If at some point the next level is not reached, the output will be empty
        // anyway.
        if levelset.get_level(next_level).is_empty() {
            return;
        }

        // NOTE: isn't there a better way of organizing this?
        self.extend_level(next_level, nonjump_adj);

        self.init_reach(next_level, jump_adj);

        self.last_level = next_level;
    }

	pub fn trim_last_level(&mut self, final_states: &BitSet) {
		self.levelset.keep_only(self.last_level, final_states);
	}

    pub fn trim_level(&mut self, level: usize, jump_adj: &Vec<Vec<usize>>, nonjump_adj: &Vec<Vec<usize>>) {
	    
	}

    pub fn is_disconnected(&self) -> bool {
        !self.levelset.has_level(self.last_level)
    }

    /// Jump to the next relevant level from vertices in gamma at a given level.
    /// A relevent level has a node from which there is a path to gamma and
    /// that has an ingoing assignation.
    ///
    /// NOTE: It may be possible to return an iterator to refs of usize, but the
    /// autoref seems to not do the work.
    pub fn jump(&self, level: usize, gamma: BitSet) -> Option<(usize, BitSet)>
    {
		let jll = &self.jl[level];
        let jump_level = gamma
            .iter()
            .map(|vertex| jll[vertex])
            .max().unwrap();

		if jump_level == level {
			return Some((level, BitSet::new()));
		}

		let mut index = 424242;

		for (i,&x) in self.rlevel[level].iter().enumerate() {
			if x == jump_level {
				index = i;
				break;
			}
		}

		let matrix = &self.reach[level][index];

		let jump_level_vertices = self.levelset.get_level(jump_level);

		let mut source_vector = BitVec::from_elem(self.levelset.get_level(level).len(),false);

		let gamma_indices = self.levelset.vertices_to_indices(level,&gamma);
		
		let gamma2 = self.levelset.indices_to_vertices(jump_level,&BitSet::from_bit_vec(matrix.col_mul(gamma_indices.get_ref())));
		
        Some((jump_level, gamma2))
    }

    /// Get the vertices that are in the final layer
    pub fn finals(&self) -> BitSet {
        if self.is_disconnected() {
            return BitSet::new();
        }

        self.levelset
            .get_level(self.last_level).clone()
    }

    pub fn get_nb_levels(&self) -> usize {
        self.levelset.get_nb_levels()
    }

    /// Extend current level by reading non-jumpable edges inside the given
    /// level.
    fn extend_level(&mut self, level: usize, nonjump_adj: &Vec<Vec<usize>>) {
		let levelset = &mut self.levelset;
        let nonjump_vertices = &mut self.nonjump_vertices;
        let old_level = levelset.get_level(level).clone();

		nonjump_vertices.clear();

        for source in old_level.iter() {
            for &target in &nonjump_adj[source] {
                levelset.register(level, target);
                nonjump_vertices.insert(target);
            }
        }
    }

    // Compute reach and rlevel, that is the effective jump points to all levels
    // reachable from the current level.
    fn init_reach(&mut self, level: usize, jump_adj: &Vec<Vec<usize>>) {
        let reach = &mut self.reach;
        let rlevel = &mut self.rlevel;
        let jl = &self.jl[level];

        let curr_level = self.levelset.get_level(level);

		reach.push(Vec::new());

        // Build rlevel as the image of current level by jl
        rlevel.insert(
            level,
            curr_level
                .iter() //.filter_map(|&source| jl.get(&(level, source)).map(|&target| target))
                .map(|source| jl[source])
                .collect(),
        );

		rlevel[level].sort();
		rlevel[level].dedup();

        // Compute the adjacency between current level and the previous one.
		let prev_level_len = self.levelset.get_level(level - 1).len();
        let mut prev_level_iter = self.levelset.get_level(level - 1).iter();
        let mut new_reach = Matrix::new(prev_level_len, curr_level.len());
		let mut targets = BitSet::with_capacity(self.num_vertices);

        for id_source in 0..prev_level_len {
			let source = prev_level_iter.next().unwrap();
            for &target in &jump_adj[source] {
				targets.insert(target);
            }

			let ids_target = self.levelset.vertices_to_indices(level,&targets);
			for id in ids_target.iter() {
            	new_reach.set(id_source, id, true);
			}
			targets.clear();
        }


		let mut new_reach_index = None;

        // Compute by a dynamic algorithm the adjacency of current level with all its
        // sublevels.
        for &sublevel in &rlevel[level] {
            // This eliminates the stupid cast of level 0.
            // TODO: fix this hardcoded behaviour.
            if sublevel == level - 1 {
                new_reach_index = Some(reach[level].len());
				continue
            }

			let mut index = 424242;

			for (i,&x) in rlevel[level-1].iter().enumerate() {
				if x == sublevel {
					index = i;
					break;
				}
			}

            let new_matrix = &reach[level-1][index] * &new_reach;

            reach[level].push(new_matrix);
        }

		match new_reach_index {
			Some(nri) => reach[level].insert(nri, new_reach),
			None => (),
		};
    }
}

impl fmt::Debug for Jump {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Levelset: {:?}", self.levelset);
		writeln!(f, "Rlevel: {:?}", self.rlevel);
		writeln!(f, "Reach: {:?}", self.reach);
		writeln!(f, "JumpLevel: {:?}", self.jl)
    }
}
