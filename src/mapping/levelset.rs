use bit_set::BitSet;
use bit_vec::BitVec;
use std::fmt;

use std::cell::RefCell;
use std::cell::Cell;

/// Represent the partitioning into levels of a product graph.
///
/// A same vertex can be store in several levels, and this level hierarchy can
/// be accessed rather efficiently.
pub struct LevelSet {
	num_vertices: usize,
	effective_level_size: usize,
    /// Index level contents: `level id` -> `vertex id's list`.
	levels: BitVec,
	temp_level: RefCell<BitVec>,
	temp_level_no: Cell<usize>,
	temp_levelset: RefCell<BitSet>,

}

impl LevelSet {
    pub fn new(num_levels: usize, num_vertices: usize) -> LevelSet {
        let effective_level_size = ((num_vertices-1)/32) + 1;

		LevelSet {
			num_vertices,
			effective_level_size,
            levels:       BitVec::<u32>::from_elem(effective_level_size*32*num_levels, false),
			temp_level: RefCell::new(BitVec::from_elem(effective_level_size*32, false)),
			temp_level_no: Cell::new(0),
			temp_levelset: RefCell::new(BitSet::with_capacity(num_vertices)),
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

	fn set_temp(&self, level: usize) {
		unsafe {

			if self.temp_level_no.get() != level {
				let levels_storage = self.levels.storage();
                let mut temp = self.temp_level.borrow_mut();
				let temp_storage = temp.storage_mut();

				self.temp_level_no.set(level);
				for i in 0..self.effective_level_size {
					temp_storage[i] = levels_storage[level* self.effective_level_size + i];
				}
			}
		}
	}

	pub fn indices_to_vertices(&self, level: usize, indices: &mut BitSet) {
		let mut temp_indices = self.temp_levelset.borrow_mut();
		temp_indices.clone_from(indices);
		indices.clear();
		let vertices = indices; 
        self.set_temp(level);
		let level_vec = &self.temp_level.borrow();
		let mut level_iter = level_vec.iter().enumerate().filter(|&(_,x)| x==true);

		let mut last = 0;
		
		for i in temp_indices.iter() {
			let mut diff = i - last;
			while diff>0 {
				level_iter.next();
				diff-=1;
			}
			
			vertices.insert(level_iter.next().unwrap().0);
			last = i + 1;
		}
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

	
	pub fn vertices_to_indices(&self, level: usize, vertices: &mut BitSet){
		let mut temp_vertices = self.temp_levelset.borrow_mut();
		temp_vertices.clone_from(vertices);
		vertices.clear();
		let indices = vertices;
		let mut count = 0;
	
        self.set_temp(level);
		let level_vec = &self.temp_level.borrow();
        let mut vertex=0;
		let mut level_iter = level_vec.iter().map(|x| {if x {count+=1} count});
		
		let mut cnt = level_iter.next().unwrap();
		
		for v in temp_vertices.iter() {
			if level_vec.get(v).unwrap() {
    			while vertex<v {
                    vertex+=1;
		    		cnt = level_iter.next().unwrap();
			    }
				indices.insert(cnt-1);
			}
		}
	}


    /// Save a vertex in a level, the vertex need to be unique inside this level
    /// but can be registered in other levels.
    pub fn register(&mut self, level: usize, vertex: usize) {
		self.levels.set(level*self.effective_level_size*32 + vertex, true);
    }

	pub fn get_memory_usage(&self) -> usize {
		self.levels.capacity()/8
	}
}

impl fmt::Debug for LevelSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for level in 0..(self.levels.len()/self.effective_level_size)/32 {
            writeln!(f,"level {}: {:?}",level,self.get_level(level))?;
        }

        writeln!(f,"")
    }
}


