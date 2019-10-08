use std::ops::{Index, Mul};
use bit_vec::BitVec;
use bit_set::BitSet;
use std::fmt;
use std::cell::RefCell;
use std::cell::Cell;

/// Naive representation of a matrix as a single consecutive chunk of memory.
pub struct Matrix {
    height: usize,
    width:  usize,
    effective_width:  usize,
    data:   BitVec,
	temp_column: RefCell<BitVec>,
	usage_count: Cell<u32>,
}

impl<'a> Matrix
{
    /// Create a matrix filled with false.
    pub fn new(height: usize, width: usize) -> Matrix {
		let effective_width = width / 32 + if (width & 31)==0 {0} else {1}; 
	
        Matrix {
            width,
            height,
			effective_width,
            data: BitVec::<u32>::from_elem(effective_width*height*32, false),
			temp_column: RefCell::new(BitVec::<u32>::from_elem(effective_width*32, false)),
			usage_count: Cell::new(0),
        }
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn get_width(&self) -> usize {
        self.width
    }


	pub fn set(&mut self, row: usize, col: usize, value: bool) {
		self.data.set(self.data_index(row,col), value);
	}

    /// Get the index of a cell in the data vector.
    fn data_index(&self, row: usize, col: usize) -> usize {
        debug_assert!(col < self.width);
        debug_assert!(row < self.height);
        col + (row * self.effective_width * 32)
    }

	pub fn col_mul_inplace(&self, column: &mut BitSet) {
		self.usage_count.set(self.usage_count.get()+1);
//		println!("col_mul: width: {} height: {}, column_height: {}", self.width, self.height, column.capacity());
		let mut temp_column = self.temp_column.borrow_mut();
		temp_column.clone_from(column.get_ref());
	    let storage_self = self.data.storage();
		let storage_other = temp_column.storage();
	    let effective_width = self.effective_width;

		column.clear();
		let result = column;
		

		for i in 0..self.height {
			for k in 0..std::cmp::min(self.effective_width,storage_other.len()) {
				if (storage_self[i*effective_width + k] & storage_other[k]) != 0 {
					result.insert(i);
					break;
				}
			}
		}
	}
	
	pub fn transpose(&self) -> Matrix {
		let mut result = Matrix::new(self.width, self.height);
		for i in 0..self.height {
			for j in 0..self.width {
				result.set(j,i,self[(i,j)]);
			}
		}
		
		result
	}

	pub fn get_usage_count(&self) -> u32 {
		self.usage_count.get()
	}

	pub fn count_ones(&self) -> usize {
		self.data.iter().filter(|&x| x).count()
	}

	pub fn get_memory_usage(&self) -> usize {
		//std::mem::size_of::<Matrix>() + 
		self.data.capacity()/8
	}
}

impl Index<(usize, usize)> for Matrix
{
    type Output = bool;

    fn index(&self, (row, col): (usize, usize)) -> &bool {
        &self.data[self.data_index(row, col)]
    }
}

//  ____              _
// | __ )  ___   ___ | | ___  __ _ _ __
// |  _ \ / _ \ / _ \| |/ _ \/ _` | '_ \
// | |_) | (_) | (_) | |  __/ (_| | | | |
// |____/ \___/ \___/|_|\___|\__,_|_| |_|
//  __  __       _        _
// |  \/  | __ _| |_ _ __(_)_  __
// | |\/| |/ _` | __| '__| \ \/ /
// | |  | | (_| | |_| |  | |>  <
// |_|  |_|\__,_|\__|_|  |_/_/\_\
//

/// Implements multiplication for matrices. The other matric is assumed to be transposed.
impl Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, other: &Matrix) -> Matrix {
	  let mut result = Matrix::new(self.height, other.height);
	  let storage_self = self.data.storage();
	  let storage_other = other.data.storage();
	  let effective_width = self.effective_width;

  	  for i in 0..self.height {
	    for j in 0..other.height {
	      for k in 0..self.effective_width {
	        if (storage_self[i*effective_width + k] & storage_other[j*effective_width+k]) != 0 {
		      result.set(i,j,true);
			  break;
	      }
            }
          }
        }  	

		result
    }
}



impl fmt::Debug for Matrix {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		writeln!(f,"")?;
		for i in 0..self.height {
			for j in 0..self.width {
				let bit = match self[(i,j)] {
					false => ".",
					true => "x",
				};
				write!(f, "{}", bit)?; 
			}
			writeln!(f,"")?;
		}
		writeln!(f,"")
    }

}
