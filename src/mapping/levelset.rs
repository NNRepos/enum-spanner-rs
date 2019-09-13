use bit_set::BitSet;

/// Represent the partitioning into levels of a product graph.
///
/// A same vertex can be store in several levels, and this level hierarchy can
/// be accessed rather efficiently.
#[derive(Debug)]
pub struct LevelSet {
	num_vertices: usize,
	
    /// Index level contents: `level id` -> `vertex id's list`.
	levels: Vec<BitSet>,

}

impl LevelSet {
    pub fn new(num_levels: usize, num_vertices: usize) -> LevelSet {
        LevelSet {
			num_vertices,
            levels:       Vec::with_capacity(num_levels),
//            vertex_index: Vec::new(),
        }
    }

    pub fn has_level(&self, level: usize) -> bool {
        (self.levels.len() > level) && (self.levels[level].len()>0)
    }

    pub fn get_level(&self, level: usize) -> &BitSet {
        &self.levels[level]
    }

    pub fn get_nb_levels(&self) -> usize {
        self.levels.len()
    }

	pub fn indices_to_vertices(&self, level: usize, indices: &BitSet) -> BitSet {
		let mut vertices = BitSet::with_capacity(self.num_vertices);
		let level_clone = &self.levels[level].clone();
		let mut level_iter = level_clone.iter();

		let mut last = 0;
		
		for i in indices.iter() {
			let mut diff = i - last;
			while (diff>0) {
				level_iter.next();
				diff-=1;
			}
			
			vertices.insert(level_iter.next().unwrap());
			last = i + 1;
		}
		
		vertices
	} 
	
	/// Used to trim the graph. Will change indices for the level.
	pub fn keep_only(&mut self, level: usize, vertices: &BitSet) {
		self.levels[level].intersect_with(vertices);
	}

	
	pub fn vertices_to_indices(&self, level: usize, vertices: &BitSet) -> BitSet{
//		print!("v_to_i({}): level: ", level);
		let mut count = 0;
		let mut indices = BitSet::with_capacity(self.levels[level].len());
		
//		let lc = &self.levels[level].clone();
//		for l in lc.iter() {
//			print!("{} ", l);
//		}
		
//		print!("vertices: ");
		
		let level_clone = &self.levels[level].clone();
		let mut level_iter = level_clone.iter();
		let mut x = level_iter.next().unwrap_or(std::usize::MAX);
		
		for v in vertices.iter() {
//			print!("{} ",v);
			while (x<v) {
				x = level_iter.next().unwrap_or(std::usize::MAX);
				count+=1;
			}
			if (x==v) {
				indices.insert(count);
			}
		}

//		print!("indices: ");		
		
//		for i in indices.iter() {
//			print!("{} ", i);
//		}

//		println!("");
		
		indices
	}


    /// Save a vertex in a level, the vertex need to be unique inside this level
    /// but can be registered in other levels.
    pub fn register(&mut self, level: usize, vertex: usize) {
		self.levels[level].insert(vertex);
    }


	pub fn add_level(&mut self) {
		self.levels.push(BitSet::with_capacity(self.num_vertices));
	}
}

