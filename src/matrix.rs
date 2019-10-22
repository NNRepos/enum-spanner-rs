use std::ops::{Index, Mul, BitOr, BitAnd};
use std::cmp::PartialEq;

use bit_set::BitSet;
use std::fmt;
use std::cell::Cell;
use std::slice;
use std::mem::{forget, size_of};

/// Naive representation of a matrix as a single consecutive chunk of memory.
pub struct Matrix {
    height: u16,
    width:  u16,
	usage_count: Cell<u16>,
	/// if size<size_of<usize> this holds the matrix. Otherwise it holds a pointer to the matrix.
    data:   usize,
}

impl<'a> Matrix
{
    /// Create a matrix filled with false.
    pub fn new(height: usize, width: usize) -> Matrix {
		let padded_width = Matrix::padded_width(width);

		let size = height * padded_width;
		let data;

//		if padded_width > 8 || height > 8 || width > 8 || size > 64 {
//			println!("Matrix size: {} {} {} {}", height, width, padded_width, size);
//		}

		if size <= size_of::<usize>()* 8 {
			data = 0;
		} else {
//			panic!("Matrix size: {}", size);
			let real_size = (size / (size_of::<usize>()*8)) + 1;
			let v: Vec<usize> = vec![0; real_size as usize];
			let data_ptr = v.as_ptr() as *mut usize;
			data = data_ptr as usize;
			forget(v);
		}
	
        Matrix {
            width: width as u16,
            height: height as u16,
			usage_count: Cell::new(0),
            data,
        }
    }

	#[inline(always)]
	fn padded_width(width: usize) -> usize {
		match width {
			0...8 => 8,
			9...16 => 16,
			17...32 => 32,
			33...64 => 64,
			_ => (width / 64 + if (width & 63)==0 {0} else {1})*64,
		}
	}

	#[inline(always)]
	fn get_width_and_size(&self) -> (usize,usize) {
		let width = Matrix::padded_width(self.width as usize);
		let size = self.height as usize * width;

		(width,size)
	}

	fn get_storage<T>(&self) -> &[T] {
		let (_,size) = self.get_width_and_size();
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
		let (_,size) = self.get_width_and_size();
		let data_ptr: *mut T;


		if size <= 64 {
			data_ptr = &mut self.data as *mut usize as *mut T;
		} else {
			data_ptr = self.data as *mut usize as *mut T;
		}
		let data;
		unsafe {
			data = slice::from_raw_parts_mut(data_ptr,size as usize/ size_of::<T>());
		}

		data
	}

    pub fn get_height(&self) -> usize {
        self.height as usize
    }

    pub fn get_width(&self) -> usize {
        self.width as usize
    }


	pub fn insert(&mut self, row: usize, col: usize) {
		let (padded_width,_) = self.get_width_and_size();

		match padded_width {
			8 => { let storage = self.get_storage_mut::<u8>(); storage[row] |= 1 << col; },
			16 => { let storage = self.get_storage_mut::<u16>(); storage[row] |= 1 << col; },
			32 => { let storage = self.get_storage_mut::<u32>(); storage[row] |= 1 << col; },
			64 => { let storage = self.get_storage_mut::<u64>(); storage[row] |= 1 << col; },
			_ => { 
				let storage = self.get_storage_mut::<u64>();

				let i = col / 64;
				let j = col % 64;
				let effective_width = padded_width / 64;
				
				storage[row*effective_width + i] |= 1 << j; 
			},
		}
	}

	pub fn col_mul_inplace(&self, column: &mut BitSet) {
		self.usage_count.set(self.usage_count.get()+1);
//		println!("col_mul: width: {} height: {}, column_height: {}", self.width, self.height, column.capacity());
		
		let (padded_width,_) = self.get_width_and_size();
		if padded_width <= 64 {
			let col = column.get_ref().storage()[0] as u64 + if column.capacity()>32 {(column.get_ref().storage()[1] as u64) <<32} else {0};
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
//			panic!("col_mul_in_place not working for width > 64");
			let mut col: Vec<u64> = vec![0;padded_width/8 + 1];
			let col_storage = column.get_ref().storage();
			for i in 0..std::cmp::min(col_storage.len(),padded_width/4 + 1)  {
				if i%2 == 0 {
					col[i/2] = col_storage[i].into();
				} else {
					col[i/2] |= (col_storage[i] as u64) << 32;
				}
			}

			column.clear();
			let result=column;

			self.col_mul_wide(&col, result);
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

	fn col_mul_wide(&self, column: &[u64], result: &mut BitSet) {
		let storage = self.get_storage::<u64>();
		let (padded_width,_) = self.get_width_and_size();
		let effective_width = padded_width / 64;

		for i in 0..self.height {
			for k in 0..effective_width	{
				if (storage[i as usize*effective_width + k] & column[k as usize])!=0 {
					result.insert(i as usize);
					break;
				}
			}
		}
	}
	
	pub fn transpose(&self) -> Matrix {
		let mut result = Matrix::new(self.width as usize, self.height as usize);
		for i in 0..self.height as usize {
			for j in 0..self.width as usize {
				if self[(i,j)] {
					result.insert(j,i);
				}
			}
		}
		
		result
	}

	pub fn get_usage_count(&self) -> usize {
		self.usage_count.get() as usize
	}

	pub fn count_ones(&self) -> usize {
		0 //self.data.iter().filter(|&x| x).count()
	}

	pub fn get_memory_usage(&self) -> usize {
		let (_padded_width,size) = self.get_width_and_size();

		std::mem::size_of::<Matrix>() + if size <= 64 {0} else {size/8}
	}		

	fn mulx<T>(&self, other: &Matrix, result: &mut Matrix) 	
	where T: BitOr + BitAnd + Copy,
	  <T as BitAnd>::Output: PartialEq + From<u8>
	{	
        let self_storage = self.get_storage::<T>();
        let other_storage = other.get_storage::<T>();

		for i in 0..self.height as usize {
			for j in 0..other.height as usize {
				if (self_storage[i as usize] & other_storage[j as usize]) != <T as BitAnd>::Output::from(0 as u8) {
					result.insert(i,j);
				}
			}
		}
	}

	fn is_heap(&self) -> bool {
		let (_,size) = self.get_width_and_size();

		size > size_of::<usize>()*8
	}
}

impl Drop for Matrix {
	fn drop(&mut self) {
		if self.is_heap() {
			unsafe {
				let (_,size) = self.get_width_and_size();
				let ptr = self.data as *mut usize;
				let len = (size / (size_of::<usize>()*8)) + 1;
				Vec::from_raw_parts(ptr, len, len);
			}
		}
	}
}

impl Index<(usize, usize)> for Matrix
{
    type Output = bool;

	#[inline(always)]
    fn index(&self, (row, col): (usize, usize)) -> &bool {
		let (padded_width,_) = self.get_width_and_size();

		let result = match padded_width {
			8 => { let storage = self.get_storage::<u8>(); (storage[row] & (1 << col)) !=0},
			16 => { let storage = self.get_storage::<u16>(); (storage[row] & (1 << col)) !=0},
			32 => { let storage = self.get_storage::<u32>(); (storage[row] & (1 << col)) !=0},
			64 => { let storage = self.get_storage::<u64>(); (storage[row] & (1 << col)) !=0},
			_ => { 
				let storage = self.get_storage::<u64>(); 
				let i = col / 64;
				let j = col % 64;
				let effective_width = padded_width / 64;
				
				(storage[row*effective_width + i] & (1 << j))!=0 
			},
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
		let mut result = Matrix::new(self.height as usize, other.height as usize);

		let (padded_width,_) = self.get_width_and_size();
		if padded_width <= 64 {
			match padded_width {
				8 => self.mulx::<u8>(other, &mut result),
				16 => self.mulx::<u16>(other, &mut result),
				32 => self.mulx::<u32>(other, &mut result),
				64 => self.mulx::<u64>(other, &mut result),
				width => panic!("invalid matrix effective width {}", width)
			}
		} else {
			let self_storage = self.get_storage::<u64>();
        	let other_storage = other.get_storage::<u64>();
			let effective_width = padded_width / 64;

			for i in 0..self.height as usize {
				for j in 0..other.height as usize {
					for k in 0..effective_width {
						if (self_storage[i * effective_width + k] & other_storage[j * effective_width + k]) != 0 {
							result.insert(i,j);
							break;
						}
					}
				}
			}

		}

//		println!("Matrix multiplication:\n{:?}\n{:?}\n{:?}",self,other,result);

		result
    }
}



impl fmt::Debug for Matrix {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		writeln!(f,"")?;
		for i in 0..self.height as usize {
			for j in 0..self.width as usize {
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
