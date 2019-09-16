use bit_set::BitSet;
use bit_vec::BitVec;
use std::fmt;

/// Represent the partitioning into levels of a product graph.
///
/// A same vertex can be store in several levels, and this level hierarchy can
/// be accessed rather efficiently.
pub struct LevelSet {
	num_vertices: usize,
	effective_level_size: usize,
    /// Index level contents: `level id` -> `vertex id's list`.
	levels: BitVec,

}

impl LevelSet {
    pub fn new(num_levels: usize, num_vertices: usize) -> LevelSet {
        let effective_level_size = ((num_vertices-1)/32) + 1;

		LevelSet {
			num_vertices,
			effective_level_size,
            levels:       BitVec::<u32>::from_elem(effective_level_size*32*num_levels, false),
//            vertex_index: Vec::new(),
        }
    }

    pub fn get_level(&self, level: usize) -> BitSet {
        let mut levelset = BitVec::from_elem(self.num_vertices,false);

		unsafe {
			let levels_storage = self.levels.storage();
			let level_storage = levelset.storage_mut();
			for i in 0..self.effective_level_size {
				level_storage[i] = levels_storage[level* self.effective_level_size + i];
			}
		}
		
		BitSet::from_bit_vec(levelset)
    }

    pub fn get_nb_levels(&self) -> usize {
        self.levels.len()
    }

	pub fn indices_to_vertices(&self, level: usize, indices: &BitSet) -> BitSet {
		let mut vertices = BitSet::with_capacity(self.num_vertices);
		let level_clone = &self.get_level(level);
		let mut level_iter = level_clone.iter();

		let mut last = 0;
		
		for i in indices.iter() {
			let mut diff = i - last;
			while diff>0 {
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
		let mut levelset = self.get_level(level);
		levelset.intersect_with(vertices);

		unsafe {
			let levels_storage = self.levels.storage_mut();
			let level_storage = levelset.get_ref().storage();
			for i in 0..self.effective_level_size {
				levels_storage[level* self.effective_level_size + i] = level_storage[i];
			}
		}
	}

	
	pub fn vertices_to_indices(&self, level: usize, vertices: &BitSet) -> BitSet{
//		print!("v_to_i({}): level: ", level);
		let mut count = 0;
		let mut indices = BitSet::with_capacity(self.num_vertices);
		
//		let lc = &self.levels[level].clone();
//		for l in lc.iter() {
//			print!("{} ", l);
//		}
		
//		print!("vertices: ");
		
		let level_clone = &self.get_level(level);
		let mut level_iter = level_clone.iter();
		let mut x = level_iter.next().unwrap_or(std::usize::MAX);
		
		for v in vertices.iter() {
//			print!("{} ",v);
			while x<v {
				x = level_iter.next().unwrap_or(std::usize::MAX);
				count+=1;
			}
			if x==v {
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
		self.levels.set(level*self.effective_level_size*32 + vertex, true);
    }


	pub fn add_level(&mut self) {
	}
}

impl fmt::Debug for LevelSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (level, vertices) in self.levels.iter().enumerate() {
            writeln!(f,"level {}: {:?}",level,vertices);
        }

        writeln!(f,"")
    }
}


