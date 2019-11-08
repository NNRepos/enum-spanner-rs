use std::cmp::max;
use std::fmt;
use bit_set::BitSet;

use super::super::matrix::Matrix;
use super::levelset::LevelSet;

/// Holds for some level the id, 
/// the jump target levels for all nodes, and 
/// a set of matrices together with the target levels
struct Level {
	id: usize,
	jl: Vec<usize>,
	reach: Vec<(usize,Matrix)>
}


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
    /// Holds the bitmat, describing which states are reachable in a level
    dag_bitmap: LevelSet,

	/// Holds all levels
	levels: Vec<Level>,

    /// Last level that was built.
    last_level: usize,

	/// vertices of the automaton that have an incomping assignment transition
	jump_vertices: BitSet,

	num_vertices: usize,
	
	/// used during init_reach phase. Holds the reach matrix between levels i and j,
	/// where i is the last jumpable level init_reach was run on and j is the last level
	/// init_reach was called on. Is empty if i==j.
	reach_matrix: Matrix,
	last_jl: Vec<usize>,
	
	/// distance between jump targets
	jump_distance: usize,
}

impl Jump {
    pub fn new<T>(initial_level: T, nonjump_adj: &Vec<Vec<usize>>, jump_vertices: &BitSet, num_levels: usize, num_vertices: usize, jump_distance: usize) -> Jump
    where
        T: Iterator<Item = usize>,
    {
        let mut jump = Jump {
            dag_bitmap:            LevelSet::new(num_levels, num_vertices),
            last_level:          0,
			jump_vertices:	     jump_vertices.clone(),
            levels:                  Vec::new(),
			num_vertices:		 num_vertices,
			reach_matrix:		 Matrix::new(1,1),
			jump_distance:       jump_distance,
			last_jl: Vec::new(),
        };

        for state in initial_level {
            jump.dag_bitmap.register(state, 0);
        }

        // Init first level
        jump.extend_level(0, nonjump_adj);

        jump
    }

	pub fn num_levels(&self) -> usize {
		self.levels.len()
	}

	pub fn get_pos(&self, level: usize) -> usize {
		self.levels[level].id
	}

    /// Compute next level given the adjacency list of jumpable edges from
    /// current level to the next one and adjacency list of non-jumpable
    /// edges inside the next level.
    pub fn init_next_level(&mut self, jump_adj: &Vec<Vec<usize>>) {
        let dag_bitmap = &mut self.dag_bitmap;

        let last_level = self.last_level;
        let next_level = self.last_level + 1;

        // NOTE: this clone is only necessary for the borrow checker.
        let last_level_vertices = dag_bitmap.get_level(last_level).clone();

        // Register jumpable transitions from this level to the next one
        for source in last_level_vertices.iter() {
            for &target in &jump_adj[source] {
		        dag_bitmap.register(next_level, target);
            }
        }

        // If at some point the next level is not reached, the output will be empty
        // anyway.
        if dag_bitmap.get_level(next_level).is_empty() {
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
		
		self.dag_bitmap.keep_only(self.last_level, &keep);
	}

    pub fn trim_level(&mut self, level: usize, rev_jump_adj: &Vec<Vec<usize>>) {
	    let dag_bitmap = &mut self.dag_bitmap;
		let next_level = dag_bitmap.get_level(level);
		let mut keep = BitSet::with_capacity(self.num_vertices);

		for target in next_level.iter() {
			for &source in &rev_jump_adj[target] {
				keep.insert(source);
			}
		}
		
//		println!("keep level: {} curr: {:?} next: {:?} keep {:?}",level, dag_bitmap.get_level(level-1), next_level, keep);
		
		dag_bitmap.keep_only(level-1, &keep);
	}

    pub fn is_disconnected(&self) -> bool {
        self.dag_bitmap.get_level(self.last_level).is_empty()
    }

    /// Jump to the next relevant level from vertices in gamma at a given level.
    /// A relevent level has a node from which there is a path to gamma and
    /// that has an ingoing assignation.
    pub fn jump(&self, level_id: usize, gamma: &mut BitSet) -> Option<usize>
    {
		
		let mut level = &self.levels[level_id];
		self.dag_bitmap.vertices_to_indices(level_id,gamma);
        let jump_level = gamma
            .iter()
            .filter_map(|vertex| {if level.jl[vertex]<std::usize::MAX {Some(level.jl[vertex])} else {None}})
            .max();

		if jump_level == None {
			return None;
		}

		let mut current_level = level_id;
		
		while current_level>jump_level.unwrap() {
			if let Some((l, matrix)) = level.reach.iter().find(|&&(id,_)| id>=jump_level.unwrap()) {
				matrix.col_mul_inplace(gamma);
				current_level = *l;
				level = &self.levels[current_level];
			} else {
				panic!("No suitable matrix found for jump.");
			}
		}	
		
		self.dag_bitmap.indices_to_vertices(jump_level.unwrap(),gamma);
		
        jump_level
    }

    /// Get the vertices that are in the final layer
    pub fn finals(&self) -> BitSet {
        if self.is_disconnected() {
            return BitSet::new();
        }

        self.dag_bitmap
            .get_level(self.last_level).clone()
    }

    /// Extend current level by reading non-jumpable edges inside the given
    /// level.
    fn extend_level(&mut self, level: usize, nonjump_adj: &Vec<Vec<usize>>) {
		let dag_bitmap = &mut self.dag_bitmap;
        let old_level = dag_bitmap.get_level(level).clone();

        for source in old_level.iter() {
            for &target in &nonjump_adj[source] {
                dag_bitmap.register(level, target);
            }
        }
    }

	fn compute_jl(&self, curr_level: &BitSet, prev_level: &BitSet, jump_adj: &Vec<Vec<usize>>, nonjump_adj: &Vec<Vec<usize>>, jl: &Vec<usize>, t_to_i: &Vec<usize>) -> Vec<usize> {
        let mut nonjump_vertices = BitSet::with_capacity(self.num_vertices);
		let prev_level_no = self.levels.len() - 1;

        for source in prev_level.iter() {
            for &target in &nonjump_adj[source] {
                nonjump_vertices.insert(target);
            }
        }

		let mut new_jl = vec![std::usize::MAX;curr_level.len()];

        // Register jumpable transitions from this level to the next one
        for (source_index,source) in prev_level.iter().enumerate() {
            // Notice that `source_jl` can be 0, however, if it is not in
            // nonjump_vertices it is sure that it is not 0 since it was
            // necessary added by following an atomic transition.
            let source_jl = jl[source_index];

            for &target in &jump_adj[source] {
				let target_index=t_to_i[target];
				if target_index!=std::usize::MAX {
                	if nonjump_vertices.contains(source) {
	                	new_jl[target_index]=prev_level_no;
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

		new_jl
	}

	fn compute_reach(&self, level: usize, curr_level: &BitSet, prev_level: &BitSet, jump_adj: &Vec<Vec<usize>>, t_to_i: &Vec<usize>) -> (Matrix,Matrix) {
        // Compute the adjacency between current level and the previous one.
		let prev_level_len = prev_level.len();
		let mut prev_level_iter = prev_level.iter();
        let mut new_reach_t = Matrix::new(curr_level.len(), prev_level_len);
		let mut targets = BitSet::with_capacity(self.num_vertices);

		// init new_reach_t to point to last level
        for id_source in 0..prev_level_len {
			let source = prev_level_iter.next().unwrap();
            for &target in &jump_adj[source] {
				if t_to_i[target]!=std::usize::MAX {
					targets.insert(t_to_i[target]);
				}
            }

			for id in targets.iter() {
            	new_reach_t.insert(id, id_source);
			}
			targets.clear();
        }

		// compute new_reach to point to reach_level
		let new_reach = if self.levels.last().unwrap().id == level - 1 {
			new_reach_t.transpose()
		} else {			
			&self.reach_matrix * &new_reach_t
		};

		(new_reach,new_reach_t)

	}

	fn init_levels(&mut self) {
		self.levels = Vec::new();
		self.levels.push(Level{
			id: 0,
			jl: vec![0;self.dag_bitmap.get_level(0).len()],
			reach: Vec::new(),
		})
	}

    /// Compute reach and rlevel, that is the effective jump points to all levels
    /// reachable from the current level.
    pub fn init_reach(&mut self, level: usize, jump_adj: &Vec<Vec<usize>>, nonjump_adj: &Vec<Vec<usize>>) {
		if level == 1 {
			self.init_levels();
		}

		let prev_level_no = self.levels.len() - 1;

        let curr_level = self.dag_bitmap.get_level(level);
        let prev_level = self.dag_bitmap.get_level(level - 1);
		let last_level = self.levels.last().unwrap();

		let jl = if level == last_level.id + 1 {
			&last_level.jl
		} else {
			&self.last_jl
		};

		let mut t_to_i = vec![std::usize::MAX; self.num_vertices];
		
		for (i,q) in curr_level.iter().enumerate() {
			t_to_i[q]=i;
		}

		let new_jl = self.compute_jl(&curr_level, &prev_level, jump_adj, nonjump_adj, jl, &t_to_i);

		let (new_reach, mut new_reach_t) = self.compute_reach(level, &curr_level, &prev_level, jump_adj, &t_to_i);
		
		// no rlevel will point to this level
		if curr_level.is_disjoint(&self.jump_vertices) && (level < self.last_level) {
			self.reach_matrix = new_reach;
			self.last_jl = new_jl;
			return;
		} 

		// we remove all levels that cannot be jumped to 
		self.dag_bitmap.move_level(level, prev_level_no + 1);
        if (level == self.last_level) {
            self.last_level = prev_level_no + 1;
            self.dag_bitmap.truncate(prev_level_no + 2);
        }


		// if necessary, update new_reach_t
		if self.levels.last().unwrap().id < level - 1 {
			new_reach_t = new_reach.transpose();
		} 

		//all reachable levels
		let mut rlev = new_jl.clone();

		rlev.sort();
		rlev.dedup();
		
		if rlev[rlev.len()-1]==std::usize::MAX {
			rlev.pop();
		}
						
		let last = rlev[rlev.len()-1];

		rlev.retain(|&x| (x==last) || (x % self.jump_distance == 0));
		
        // Compute by a dynamic algorithm the adjacency of current level with all its
        // sublevels.
		let mut matrix_iterator = last_level.reach.iter();

		let mut matrices = Vec::with_capacity(rlev.len());

        for sublevel in rlev {
            if sublevel == prev_level_no {
				continue;
            } else {
				if let Some((_,matrix)) = matrix_iterator.find(|&&(l,_)| l == sublevel) {
	            	matrices.push((sublevel, matrix * &new_reach_t));
				} else {
					panic!("Matrix not found for sublevel {} level: {}", sublevel, level);
				}
			}
        }
		matrices.push((prev_level_no, new_reach));

		let new_level = Level {
			id: level,
			jl: new_jl,
			reach: matrices,
		};

		self.levels.push(new_level);
    }

	pub fn get_statistics(&self) -> (usize, usize, f64, usize, f64, usize, f64) {
		let (num_matrices, num_used_matrices, matrix_avg_size, matrix_max_size, matrix_avg_density) = self.get_matrix_stats();

		(num_matrices, num_used_matrices, matrix_avg_size, matrix_max_size, matrix_avg_density, self.get_max_width(), self.get_avg_width())
	}

	fn get_num_matrices(&self) -> usize {
		self.levels.iter().fold(0, |acc, x| acc + x.reach.len())
	}

	fn get_matrix_stats(&self) -> (usize, usize, f64, usize, f64) {
		let (count, used_count, total_size, max_size, count_ones) = MatrixIterator::init(self).fold((0,0,0,0,0), |(count, used_count, total_size, max_size, count_ones), x| {
			let size = x.get_width() * x.get_height();

			(count + 1, used_count + if x.get_usage_count()>0 {1} else {0}, total_size + size, std::cmp::max(max_size, size), count_ones + x.count_ones())
		} );

		(count, used_count, total_size as f64 / count as f64, max_size as usize, count_ones as f64 / total_size as f64)
	}

	fn get_max_width(&self) -> usize {
		self.levels.iter().fold(0, |acc, x| core::cmp::max(acc, x.reach.len()))
	}

	fn get_avg_width(&self) -> f64 {
		self.levels.iter().fold(0, |acc, x| acc + x.reach.len()) as f64 / self.levels.len() as f64
	}

	/// returns a rough estimation of the memory usage
	pub fn get_memory_usage(&self) -> (usize, usize, usize) {
		(self.dag_bitmap.get_memory_usage(), self.get_matrix_usage(), self.get_jl_usage())
	}

	#[inline(never)]
	fn get_matrix_usage(&self) -> usize {
		self.levels.iter().fold(0, |acc, x| { 
			acc + x.reach.iter().fold(std::mem::size_of::<Level>() - std::mem::size_of::<Vec<usize>>(), |acc2, (_,y)| acc2 + y.get_memory_usage())
		})
	}

	#[inline(never)]
	fn get_jl_usage(&self) -> usize {
		self.levels.iter().fold(0, |acc, x| acc + std::mem::size_of::<Vec<usize>>() + x.jl.capacity()*std::mem::size_of::<usize>())
	}
}


/// iterates over all matrices for statistical reasons
struct MatrixIterator<'a> {
	level_iterator: std::slice::Iter<'a,Level>,
	matrix_iterator: std::slice::Iter<'a,(usize,Matrix)>,
}

impl<'a> MatrixIterator<'a> {
	fn init(jump: &'a Jump) -> MatrixIterator {
		let mut level_iterator = jump.levels.iter();
		let mut matrix_iterator = level_iterator.next().unwrap().reach.iter();

		MatrixIterator {
			level_iterator,
			matrix_iterator,
		}
	}
}

impl<'a> Iterator for MatrixIterator<'a> {
	type Item = &'a Matrix;

	fn next(&mut self) -> Option<&'a Matrix> {
		match self.matrix_iterator.next() {
			Some((_,matrix)) => Some(matrix),
			None => {
				if let Some(level) = self.level_iterator.next() {
					self.matrix_iterator = level.reach.iter();
				}

				match self.matrix_iterator.next() {
					None => None,
					Some((_,matrix)) => Some(matrix),
				}
			}
		}
	}
}
