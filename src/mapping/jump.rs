use std::cmp::max;
use std::fmt;
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

	/// vertices of the automaton that have an incomping assignment transition
	jump_vertices: BitSet,

    /// Closest level where an assignation is done accessible from any node.
    jl: Vec<Vec<usize>>,

    /// Set of levels accessible from any level using `jl`.
    rlevel: Vec<Vec<usize>>,
    /// For any pair of level `(i, j)` such that i at the k-th position in `rlevel[j]`,
    /// `reach[i][k]` is the accessibility matrix of vertices from level i
    /// to level j.
    reach: Vec<Vec<Matrix>>,

	num_vertices: usize,
	
	/// used during init_reach phase. Holds the reach matrix between levels i and j,
	/// where i is the last jumpable level init_reach was run on and j is the last level
	/// init_reach was called on. Is empty if i==j.
	reach_matrix: Matrix,
	reach_level: usize,
	
	/// distance between jump targets
	jump_distance: usize,
	
	/// The levels that are valid jump targets  
	jump_levels: BitSet,
	
	last_jump_level: usize,
}

impl Jump {
    pub fn new<T>(initial_level: T, nonjump_adj: &Vec<Vec<usize>>, jump_vertices: &BitSet, num_levels: usize, num_vertices: usize, jump_distance: usize) -> Jump
    where
        T: Iterator<Item = usize>,
    {
        let mut jump = Jump {
            levelset:            LevelSet::new(num_levels, num_vertices),
            last_level:          0,
			jump_vertices:	     jump_vertices.clone(),
//            count_ingoing_jumps: HashMap::new(),
            jl:                  Vec::with_capacity(num_levels),
            rlevel:              Vec::with_capacity(num_levels),
            reach:               Vec::with_capacity(num_levels),
			num_vertices:		 num_vertices,
			reach_matrix:		 Matrix::new(1,1),
			reach_level:         0,
			jump_distance:       jump_distance,
			last_jump_level:     0,
			jump_levels:         if jump_distance==1 {
				                     BitSet::default()
                                 } else {
                                     BitSet::with_capacity(num_levels)	
			                     },
        };

        jump.rlevel.push(Vec::new());
		jump.reach.push(Vec::new());
		jump.jl.push(vec![0; num_vertices]);
		jump.jump_levels.insert(0);

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
    pub fn init_next_level(&mut self, jump_adj: &Vec<Vec<usize>>) {
        let levelset = &mut self.levelset;

        let last_level = self.last_level;
        let next_level = self.last_level + 1;

        // NOTE: this clone is only necessary for the borrow checker.
        let last_level_vertices = levelset.get_level(last_level).clone();

        // Register jumpable transitions from this level to the next one
        for source in last_level_vertices.iter() {
            for &target in &jump_adj[source] {
		        levelset.register(next_level, target);
            }
        }

        // If at some point the next level is not reached, the output will be empty
        // anyway.
        if levelset.get_level(next_level).is_empty() {
            return;
        }

        self.last_level = next_level;
    }

	pub fn trim_last_level(&mut self, final_states: &BitSet, nonjump_adj: &Vec<Vec<usize>>) {
		let mut keep = final_states.clone();
		for source in 0..nonjump_adj.len() {
			for &target in &nonjump_adj[source] {
				if keep.contains(target) {
					keep.insert(source);
				}
			}
		}
		
		self.levelset.keep_only(self.last_level, &keep);
	}

    pub fn trim_level(&mut self, level: usize, rev_jump_adj: &Vec<Vec<usize>>) {
	    let levelset = &mut self.levelset;
		let next_level = levelset.get_level(level);
		let mut keep = BitSet::with_capacity(self.num_vertices);

		for target in next_level.iter() {
			for &source in &rev_jump_adj[target] {
				keep.insert(source);
			}
		}
		
//		println!("keep level: {} curr: {:?} next: {:?} keep {:?}",level, levelset.get_level(level-1), next_level, keep);
		
		levelset.keep_only(level-1, &keep);
	}

    pub fn is_disconnected(&self) -> bool {
        self.levelset.get_level(self.last_level).is_empty()
    }

    /// Jump to the next relevant level from vertices in gamma at a given level.
    /// A relevent level has a node from which there is a path to gamma and
    /// that has an ingoing assignation.
    ///
    /// NOTE: It may be possible to return an iterator to refs of usize, but the
    /// autoref seems to not do the work.
    pub fn jump(&self, level: usize, gamma: &mut BitSet) -> Option<usize>
    {
		let jll = &self.jl[level];
		self.levelset.vertices_to_indices(level,gamma);
        let jump_level = gamma
            .iter()
            .filter_map(|vertex| {if jll[vertex]<std::usize::MAX {Some(jll[vertex])} else {None}})
            .max().unwrap_or(level);

		if jump_level == level {
			return None;
		}

		let mut current_level = level;
		
//		println!("level: {}   jump_level: {}", level, jump_level);

		while current_level>jump_level {
			let mut next_level = current_level;
			let mut index = 0;

			for &x in self.rlevel[current_level].iter() {
				if x < jump_level {
					index += 1 ;
				} else {
					next_level = x; 
					break;
				}
			}

			let matrix = &self.reach[current_level][index];
			matrix.col_mul_inplace(gamma);
			
			current_level = next_level;
		}	
		
		self.levelset.indices_to_vertices(jump_level,gamma);
		
        Some(jump_level)
    }

    /// Get the vertices that are in the final layer
    pub fn finals(&self) -> BitSet {
        if self.is_disconnected() {
            return BitSet::new();
        }

        self.levelset
            .get_level(self.last_level).clone()
    }

    /// Extend current level by reading non-jumpable edges inside the given
    /// level.
    fn extend_level(&mut self, level: usize, nonjump_adj: &Vec<Vec<usize>>) {
		let levelset = &mut self.levelset;
        let old_level = levelset.get_level(level).clone();

        for source in old_level.iter() {
            for &target in &nonjump_adj[source] {
                levelset.register(level, target);
            }
        }
    }

    // Compute reach and rlevel, that is the effective jump points to all levels
    // reachable from the current level.
    pub fn init_reach(&mut self, level: usize, jump_adj: &Vec<Vec<usize>>, nonjump_adj: &Vec<Vec<usize>>) {
//		println!("init_reach level: {}", level);
		if level == 0 {
			return;
		}
        let reach = &mut self.reach;
        let rlevel = &mut self.rlevel;

        let curr_level = self.levelset.get_level(level);

		reach.push(Vec::new());

		let mut new_jl = vec![std::usize::MAX;curr_level.len()];

		let prev_level_len = self.levelset.get_level(level - 1).len();
        let prev_level = self.levelset.get_level(level - 1);

        let mut nonjump_vertices = BitSet::with_capacity(self.num_vertices);

        for source in prev_level.iter() {
            for &target in &nonjump_adj[source] {
                nonjump_vertices.insert(target);
            }
        }

		let mut t_to_i = vec![std::usize::MAX; self.num_vertices];
		
		for (i,q) in curr_level.iter().enumerate() {
			t_to_i[q]=i;
		}


        // Register jumpable transitions from this level to the next one
        for (source_index,source) in prev_level.iter().enumerate() {
            // Notice that `source_jl` can be 0, however, if it is not in
            // nonjump_vertices it is sure that it is not 0 since it was
            // necessary added by following an atomic transition.
            let source_jl = self.jl[level-1][source_index];

            for &target in &jump_adj[source] {
				let target_index=t_to_i[target];
				if target_index!=std::usize::MAX {
                	if nonjump_vertices.contains(source) {
	                	new_jl[target_index]=level - 1;
					} else {
						if new_jl[target_index]==std::usize::MAX {
							new_jl[target_index]=source_jl;
						} else {
                    		new_jl[target_index]=max(source_jl, new_jl[target_index]);
						}
					}
                }
            }
        }

//		println!("Jump levels for level {}: {:?}", level, new_jl);

		self.jl.push(new_jl);



        // Compute the adjacency between current level and the previous one.
		let mut prev_level_iter = prev_level.iter();
        let mut new_reach_t = Matrix::new(curr_level.len(), prev_level_len);
		let mut targets = BitSet::with_capacity(self.num_vertices);

		// init new_reach_t to point to last level
        for id_source in 0..prev_level_len {
			let source = prev_level_iter.next().unwrap();
            for &target in &jump_adj[source] {
				targets.insert(target);
            }

			self.levelset.vertices_to_indices(level,&mut targets);
			for id in targets.iter() {
            	new_reach_t.set(id, id_source, true);
			}
			targets.clear();
        }

		// compute new_reach to point to reach_level
		let new_reach = if self.reach_level == level - 1 {
			new_reach_t.transpose()
		} else {			
			&self.reach_matrix * &new_reach_t
		};
		
		// no rlevel will point to this level
		if curr_level.is_disjoint(&self.jump_vertices) && (level < self.last_level) {
			self.reach_matrix = new_reach;
			rlevel.insert(level, Vec::new());
			return;
		} 

		// if necessary, update new_reach_t
		if self.reach_level < level - 1 {
			new_reach_t = new_reach.transpose();
		} 

		let mut rlev = self.jl[level].clone();

		rlev.sort();
		rlev.dedup();
		
		if rlev[rlev.len()-1]==std::usize::MAX {
			rlev.pop();
		}
						
		if level < self.last_jump_level + self.jump_distance {
		} else {
			self.last_jump_level = level;
			self.jump_levels.insert(level);
		}

		let last = rlev[rlev.len()-1];
		let jump_levels = &self.jump_levels;

		rlev.retain(|&x| (x==last) || (jump_levels.contains(x)));
		
		rlevel.push(rlev);

//		println!("rlevel[{}].len(): {}", level, rlevel[level].len());

        // Compute by a dynamic algorithm the adjacency of current level with all its
        // sublevels.
        for &sublevel in &rlevel[level] {
            if sublevel == self.reach_level {
				continue
            }

			let mut index = std::usize::MAX;

			for (i,&x) in rlevel[self.reach_level].iter().enumerate() {
				if x == sublevel {
					index = i;
					break;
				}
			}
			
			if index == std::usize::MAX {
				panic!("Index not found for sublevel {} in rlevel[{}] level: {}", sublevel, self.reach_level, level);
			}

//			println!("sublevel: {}  index: {}", sublevel, index);

            let new_matrix = &reach[self.reach_level][index] * &new_reach_t;

//			println!("Compute matrix ({},{}) insert at index {}", sublevel, level, reach[level].len() );

            reach[level].push(new_matrix);
        }

		self.reach_level = level;


		reach[level].push(new_reach);
    }

	pub fn get_statistics(&self) -> (usize, usize, f64) {
		(self.get_num_matrices(), self.get_max_width(), self.get_avg_width())
	}

	fn get_num_matrices(&self) -> usize {
		self.reach.iter().fold(0, |acc, x| acc + x.len())
	}

	fn get_max_width(&self) -> usize {
		self.jl.iter().fold(0, |acc, x| core::cmp::max(acc, x.len()))
	}

	fn get_avg_width(&self) -> f64 {
		self.jl.iter().fold(0, |acc, x| acc + x.len()) as f64 / self.jl.len() as f64
	}

	/// returns a rough estimation of the memory usage
	pub fn get_memory_usage(&self) -> usize {
		self.levelset.get_memory_usage() + self.get_matrix_usage() + self.get_rlevel_usage() + self.get_jl_usage()
	}

	#[inline(never)]
	fn get_matrix_usage(&self) -> usize {
		self.reach.iter().fold(0, |acc, x| { 
			acc + x.iter().fold(std::mem::size_of::<Vec<usize>>(), |acc2, y| acc2 + y.get_memory_usage())
		})
	}

	#[inline(never)]
	fn get_rlevel_usage(&self) -> usize {
		self.rlevel.iter().fold(0, |acc, x| { acc + std::mem::size_of::<Vec<usize>>() + x.capacity() * std::mem::size_of::<usize>()})
	}

	#[inline(never)]
	fn get_jl_usage(&self) -> usize {
		self.jl.iter().fold(0, |acc, x| { acc + std::mem::size_of::<Vec<usize>>() + x.capacity() * std::mem::size_of::<usize>()})
	}
}

impl fmt::Debug for Jump {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//        writeln!(f, "Levelset: {:?}", self.levelset);
//		writeln!(f, "Rlevel: {:?}", self.rlevel);
//		writeln!(f, "Reach: {:?}", self.reach);
//		writeln!(f, "JumpLevel: {:?}", self.jl)
		
		let mut hist = vec![0;self.num_vertices];
		let mut hist2 = vec![0;self.num_vertices];
		let mut num_matrices = 0;
		
		for level in 0..self.last_level {
			hist[self.levelset.get_level(level).len()]+=1;
			hist2[self.rlevel[level].len()]+=1;
			num_matrices+=self.rlevel[level].len();
		}

		writeln!(f,"Level histogramm: {:?}", hist)?;
		writeln!(f,"RLevel histogramm: {:?}", hist2)?;
		writeln!(f,"num_matrices: {}", num_matrices)

//		writeln!(f,"{:?}", self.levelset)
    }
}
