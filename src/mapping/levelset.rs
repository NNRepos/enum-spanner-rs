/// Represent the partitioning into levels of a product graph.
///
/// A same vertex can be store in several levels, and this level hierarchy can
/// be accessed rather efficiently.
#[derive(Debug)]
pub struct LevelSet {
	num_vertices: usize,
	
    /// Index level contents: `level id` -> `vertex id's list`.
	levels: Vec<Vec<usize>>,

    /// Index the id of a vertex iner to a level:
    ///     `(level id, vertex id)` -> `vertex position`.
    /// It can also be used to check if a pair `(level, vertex)` is already
    /// represented in the structure.
	vertex_index: Vec<Vec<usize>>,
}

impl LevelSet {
    pub fn new(num_levels: usize, num_vertices: usize) -> LevelSet {
        LevelSet {
			num_vertices,
            levels:       Vec::with_capacity(num_levels),
            vertex_index: Vec::new(),
        }
    }

    pub fn has_level(&self, level: usize) -> bool {
        (self.levels.len() > level) && (self.levels[level].len()>0)
    }

    pub fn get_level(&self, level: usize) -> &Vec<usize> {
        &self.levels[level]
    }

    pub fn get_nb_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn get_vertex_index(&self, level: usize, vertex: usize) -> Option<usize> {
        match self.vertex_index[level][vertex] {
		std::usize::MAX => None,
		_ => Some(self.vertex_index[level][vertex])
		}		

    }

//    /// Iterate over pairs (vertex, vertex_index) of a level
//    pub fn iter_level<'a>(&'a self, level: usize) -> impl Iterator<Item = (usize, usize)> + 'a {
//        let vertices = self.levels[&level].iter();
//        let levels = iter::repeat(level);
//
//        levels
//            .zip(vertices)
//            .map(move |(level, &vertex)| (vertex, self.vertex_index[&(level, vertex)]))
//    }

    /// Save a vertex in a level, the vertex need to be unique inside this level
    /// but can be registered in other levels.
    pub fn register(&mut self, level: usize, vertex: usize) {
		if self.vertex_index[level][vertex]==std::usize::MAX {
			self.vertex_index[level][vertex]=self.levels[level].len();
			self.levels[level].push(vertex);
		}
    }

    /// Remove a set of vertices from a level, if the level is left empty, it is
    /// then removed.
    //pub fn remove_from_level(&mut self, level: usize, del_vertices: &HashSet<usize>) {
	// TODO
    //}

	pub fn add_level(&mut self) {
		self.levels.push(Vec::with_capacity(self.num_vertices));
		self.vertex_index.push(vec![std::usize::MAX; self.num_vertices]);
	}
}
