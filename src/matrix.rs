use std::ops::{Index, Mul, BitOr, BitAnd};
use std::cmp::PartialEq;

use bit_vec::BitVec;
use bit_set::BitSet;
use std::fmt;
use std::cell::RefCell;
use std::cell::Cell;
use std::slice;
use std::mem::{forget, replace, size_of};

/// Naive representation of a matrix as a single consecutive chunk of memory.
pub struct Matrix {
    height: u32,
    width:  u32,
    data:   usize,
	usage_count: Cell<u32>,
}

impl<'a> Matrix
{
    /// Create a matrix filled with false.
    pub fn new(height: u32, width: u32) -> Matrix {
		let padded_width = Matrix::padded_width(width);

		let size = height * padded_width;
		let data;

		if padded_width > 8 || height > 8 || width > 8 || size > 64 {
			println!("Matrix size: {} {} {} {}", height, width, padded_width, size);
		}

		if size < size_of::<usize>() as u32 * 8 {
			data = 0;
		} else {
			let real_size = (size / size_of::<usize>() as u32) + 1;
			let v: Vec<usize> = vec![0; real_size as usize];
			let data_ptr = v.as_ptr() as *mut usize;
			data = data_ptr as usize;
			forget(v);
		}
	
        Matrix {
            width,
            height,
            data,
			usage_count: Cell::new(0),
        }
    }

	#[inline(always)]
	fn padded_width(width: u32) -> u32 {
		match width {
			0...8 => 8,
			9...16 => 16,
			17...32 => 32,
			33...64 => 64,
			_ => width / 64 + if (width & 63)==0 {0} else {1},
		}
	}

	fn get_storage<T>(&self) -> &[T] {
		let padded_width = Matrix::padded_width(self.width);
		let size = self.height * padded_width;
		let data_ptr: *const T;


		if size <= 64 {
			data_ptr = &self.data as *const usize as *const T;
		} else {
			data_ptr = self.data as *const usize as *const T;
		}
		let data;
		unsafe {
			data = slice::from_raw_parts(data_ptr,size as usize/ size_of::<T>());
		}

		data
	}

	fn get_storage_mut<T>(&mut self) -> &mut[T] {
		let padded_width = Matrix::padded_width(self.width);
		let size = self.height * padded_width;
		let data_ptr: *mut T;


		if size <= 64 {
			data_ptr = &mut self.data as *mut usize as *mut T;
		} else {
			data_ptr = self.data as *mut usize as *mut T;
		}
		let mut data;
		unsafe {
			data = slice::from_raw_parts_mut(data_ptr,size as usize/ size_of::<T>());
		}

		data
	}

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }


	pub fn insert(&mut self, row: u32, col: u32) {
		let padded_width = Matrix::padded_width(self.width);
		let size = self.height * padded_width;

		match padded_width {
			8 => { let storage = self.get_storage_mut::<u8>(); storage[row as usize] |= 1 << col; },
			16 => { let storage = self.get_storage_mut::<u16>(); storage[row as usize] |= 1 << col; },
			32 => { let storage = self.get_storage_mut::<u32>(); storage[row as usize] |= 1 << col; },
			64 => { let storage = self.get_storage_mut::<u64>(); storage[row as usize] |= 1 << col; },
			_ => (),
		}
	}

	pub fn col_mul_inplace(&self, column: &mut BitSet) {
		self.usage_count.set(self.usage_count.get()+1);
//		println!("col_mul: width: {} height: {}, column_height: {}", self.width, self.height, column.capacity());
		
	    let padded_width = Matrix::padded_width(self.width);
		if padded_width <= 64 {
			let col = column.get_ref().storage()[0] as u64 + if padded_width==64 {(column.get_ref().storage()[1] as u64) <<32} else {0};
			column.clear();
			let result=column;

			match padded_width {
				8 => self.col_mul(col as u8, result),
				16 => self.col_mul(col as u16, result),
				32 => self.col_mul(col as u32, result),
				64 => self.col_mul(col as u64, result),
				width => panic!("invalid matrix effective width {}", width)
			}
		} else {
//			let col: Vec<u64> = vec![0;padded_width as usize/8 + 1];
//			let col_storage = column.get_ref().storage();
//			for i in 0..col_storage.len() {
//				col[i] = col_storage[i];
//			}

//			column.clear();
//			let result=column;

//			self.col_mul_wide(col, result);
		}
	}

	fn col_mul<T>(&self, column: T, result: &mut BitSet) 
	where T: BitOr + BitAnd + Copy + fmt::Display,
	  <T as BitAnd>::Output: PartialEq + From<u8>
	{
		let storage = self.get_storage::<T>();
		for i in 0..self.height {
			if (storage[i as usize] & column) != <T as BitAnd>::Output::from(0 as u8) {
				result.insert(i as usize);
			}
		}
	}

	fn col_mul_wide(&self, column: &mut [u64], result: &mut BitSet) {
		let storage = self.get_storage::<u64>();
		let len = storage.len();
		for i in 0..self.height {
			for k in 0..len {
				if (storage[i as usize*len + k] & column[k as usize])!=0 {
					result.insert(i as usize);
					break;
				}
			}
		}
	}
	
	pub fn transpose(&self) -> Matrix {
		let mut result = Matrix::new(self.width, self.height);
		for i in 0..self.height {
			for j in 0..self.width {
				if self[(i,j)] {
					result.insert(j,i);
				}
			}
		}
		
		result
	}

	pub fn get_usage_count(&self) -> u32 {
		self.usage_count.get()
	}

	pub fn count_ones(&self) -> usize {
		0 //self.data.iter().filter(|&x| x).count()
	}

	pub fn get_memory_usage(&self) -> usize {
		//std::mem::size_of::<Matrix>() + 
		0 // self.data.capacity()/8
	}		

	fn mulx<T>(&self, other: &Matrix, result: &mut Matrix) 	
	where T: BitOr + BitAnd + Copy,
	  <T as BitAnd>::Output: PartialEq + From<u8>
	{	
        let self_storage = self.get_storage::<T>();
        let other_storage = other.get_storage::<T>();

		for i in 0..self.height {
			for j in 0..other.height {
				if (self_storage[i as usize] & other_storage[j as usize]) != <T as BitAnd>::Output::from(0 as u8) {
					result.insert(i,j);
				}
			}
		}
	}
}

impl Index<(u32, u32)> for Matrix
{
    type Output = bool;

	#[inline(always)]
    fn index(&self, (row, col): (u32, u32)) -> &bool {
		let padded_width = Matrix::padded_width(self.width);
		let size = self.height * padded_width;

		let result = match padded_width {
			8 => { let storage = self.get_storage::<u8>(); (storage[row as usize] & (1 << col)) !=0},
			16 => { let storage = self.get_storage::<u16>(); (storage[row as usize] & (1 << col)) !=0},
			32 => { let storage = self.get_storage::<u32>(); (storage[row as usize] & (1 << col)) !=0},
			64 => { let storage = self.get_storage::<u64>(); (storage[row as usize] & (1 << col)) !=0},
			_ => true,
		};

		if result {
			&true
		} else {
			&false
		}
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

    	let padded_width = Matrix::padded_width(self.width);
		if padded_width <= 64 {
			match padded_width {
				8 => self.mulx::<u8>(other, &mut result),
				16 => self.mulx::<u16>(other, &mut result),
				32 => self.mulx::<u32>(other, &mut result),
				64 => self.mulx::<u64>(other, &mut result),
				width => panic!("invalid matrix effective width {}", width)
			}
		} else {
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
