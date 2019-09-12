use std::ops::{Index, Mul};
use bit_vec::BitVec;

/// Naive representation of a matrix as a single consecutive chunk of memory.
pub struct Matrix {
    height: usize,
    width:  usize,
    padded_height: usize,
    padded_width:  usize,
    data:   BitVec,
	transposed_data: BitVec
}

// Custom trait for matrices that can be right-multiplied by a column vector.
pub trait ColMul {
    fn col_mul(&self, column: &Vec<bool>) -> Vec<bool>;
}

impl<'a> Matrix
{
    /// Create a matrix filled with false.
    pub fn new(height: usize, width: usize) -> Matrix {
		let padded_width = (((width - 1) / 32 ) + 1) * 32; 
		let padded_height = (((height - 1) / 32 ) + 1) * 32; 
	
        Matrix {
            width,
            height,
			padded_height,
			padded_width,
            data: BitVec::<u32>::from_elem(padded_width*height, false),
			transposed_data: BitVec::<u32>::from_elem(padded_height*width, false)
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
		self.transposed_data.set(self.data_transposed_index(row,col), value);
	}

    /// Get the index of a cell in the data vector.
    fn data_index(&self, row: usize, col: usize) -> usize {
        debug_assert!(col < self.width);
        debug_assert!(row < self.height);
        col + (row * self.padded_width)
    }

    /// Get the index of a cell in the data vector.
    fn data_transposed_index(&self, row: usize, col: usize) -> usize {
        debug_assert!(col < self.width);
        debug_assert!(row < self.height);
        row + (col * self.padded_height)
    }

	pub fn col_mul(&self, column: &BitVec) -> BitVec {
	    let storage_self = self.data.storage();
		let storage_other = column.storage();
	    let effective_width = self.padded_width/32;

		let mut result = BitVec::<u32>::from_elem(self.height, false);

		for i in 0..self.height {
			for k in 0..self.padded_width/32 {
				if (storage_self[i*effective_width + k] & storage_other[k]) != 0 {
					result.set(i, true);
					break;
				}
			}
		}

		result
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

impl Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, other: &Matrix) -> Matrix {
	  let mut result = Matrix::new(self.height, other.width);
	  let storage_self = self.data.storage();
	  let storage_other = other.transposed_data.storage();
	  let effective_width = self.padded_width/32;
      let effective_height = other.padded_height/32;

  	  for i in 0..self.height {
	    for j in 0..other.width {
	      for k in 0..self.padded_width/32 {
			//println!("mul2: {}, {}, {}", i, j, k);
	        if (storage_self[i*effective_width + k] & storage_other[j*effective_height+k]) != 0 {
		      result.set(i,j,true);
			  break;
	      }
            }
          }
        }  	

		result
    }
}

//impl ColMul for Matrix {
//    fn col_mul(&self, column: &Vec<bool>) -> Vec<bool> {
//        (0..self.height)
//            .map(|row| {
//                let row_iter = self.iter_row(row);
//                let col_iter = column.iter();
//                row_iter.zip(col_iter).any(|(&x, &y)| x && y)
//            })
//            .collect()
//    }
//}

