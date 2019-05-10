#![feature(alloc_layout_extra)]


use std::{ 
    alloc::{alloc_zeroed, dealloc, Layout},
    fmt::{Debug, Display, Formatter, Result as Result_},
    ptr,
    cmp::min,
};

fn get_next_pow2(num: usize) -> usize {
    let mut n = num;
    if n == 0 {
        return 1;
    }
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n |= n >> 32;
    return n + 1;
}

struct ArrayIter<T>
where
    T: Copy,
{
    ptr: *mut T,
    offset: usize,
    size: usize,
}
impl<T: Copy> ArrayIter<T> {
    fn new(a: Array<T>) -> ArrayIter<T> {
        ArrayIter {
            ptr: a.ptr,
            offset: 0,
            size: a.size,
        }
    }
}
impl<T: Copy> Iterator for ArrayIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let out: Option<Self::Item>;
        if self.offset >= self.size {
            out = None;
        } else {
            unsafe {
                out = Some(*(self.ptr.add(self.offset)));
            }
        }
        self.offset += 1;
        return out;
    }
}

struct Array<T>
where
    T: Copy,
{
    ptr: *mut T,
    size: usize,
}
impl<T: Copy + Display> Array<T> {
    // Initialisers

    unsafe fn new(size: usize) -> Array<T> {
        let layout = Layout::array::<T>(size).unwrap();
        let ptr: *mut T = alloc_zeroed(layout) as *mut T;
        Array {
            ptr: ptr,
            size: size,
        }
    }

    unsafe fn from_vec(v: &Vec<T>) -> Array<T> {
        let sz: usize = v.len();
        let mut a: Array<T> = Array::new(sz);
        for index in 0..sz {
            a.set(index, *v.get(index).unwrap());
        }
        a
    }

    // Movers

    unsafe fn resize(&self, size: usize) -> Array<T> {
        let ptr: *mut T = alloc_zeroed(Layout::array::<T>(size).unwrap()) as *mut T;
        let mut new = Array {
            ptr: ptr,
            size: size,
        };

        for index in 0..min(size, self.size) {
            new.set(index, self.get(index));
        }

        drop(self);

        new
    }

    unsafe fn grow_pow2(&self) -> Array<T> {
        let next_pow2 = get_next_pow2(self.size);
        self.resize(next_pow2)
    }

    unsafe fn shrink_pow2(&self) -> Array<T> {
        let lower_pow2 = if self.size > 1 { self.size / 2 } else { 1 };
        self.resize(lower_pow2)
    }


    // Methods

    unsafe fn copy(&mut self) -> Array<T> {
        let mut a: Array<T> = Array::new(self.size);
        for index in 0..self.size {
            a.set(index, self.get(index));
        }
        a
    }

    unsafe fn set(&mut self, index: usize, value: T) -> Result<(), String> {
        if index >= self.size {
            Err(format!(
                "Unable to set {} to {}, index out of range",
                index, value
            ))
        } else {
            ptr::write(self.ptr.add(index), value);
            Ok(())
        }
    }

    unsafe fn get(&self, index: usize) -> T {
        *(self.ptr.add(index))
    }

    unsafe fn to_vec(&self) -> Vec<T> {
        let mut dst: Vec<T> = Vec::new();
        for offset in 0..self.size {
            let addr = self.ptr.add(offset);
            dst.push(*(addr));
        }
        return dst;
    }

    unsafe fn shift(&mut self, amt: isize) -> Self {
        self.shift_from(0, amt)
    }

    unsafe fn shift_from(&mut self, index: usize, amt: isize) -> Self {
        let mut buf: Self = Array::new(self.size);
        for count in 0..index {
            buf.set(count, self.get(count));
        }
        for count in index..self.size {
            if count as isize + amt < 0 {
                                continue;
            } else {
                let index: usize = (count as isize + amt) as usize;
                buf.set(index, self.get(count));
            }
        }

        return buf;
    }
}
impl<T: Copy> Drop for Array<T> {

    // Destructor

    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr as *mut u8, Layout::array::<T>(self.size).unwrap());
        }
    }
}
impl<T: Copy> IntoIterator for Array<T> {
    type Item = T;
    type IntoIter = ArrayIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        ArrayIter::new(self)
    }
}
impl<T: Copy + Debug + Display> Display for Array<T> {
    fn fmt(&self, f: &mut Formatter) -> Result_ {
        unsafe { write!(f, "{:?}", self.to_vec()) }
    }
}

struct ArrayList<T>
where
    T: Copy + Display + PartialEq,
{
    arr: Array<T>,
    len: usize,
    cap: usize,
}
impl<T: Copy + Display + PartialEq> ArrayList<T> {
    unsafe fn new() -> ArrayList<T> {
        ArrayList {
            arr: Array::<T>::new(1),
            len: 0,
            cap: 1,
        }
    }

    unsafe fn from_vec(v: &Vec<T>) -> ArrayList<T> {
        ArrayList {
            arr: Array::from_vec(v).grow_pow2(),
            len: v.len(),
            cap: get_next_pow2(v.len()),
        }
    }

    unsafe fn push(&mut self, value: T, index: usize) {
        self.len += 1;
        if self.len >= self.cap {
            self.cap = get_next_pow2(self.cap);
            self.arr = self.arr.grow_pow2();
        }
        self.arr = self.arr.shift_from(index, 1);
        self.arr.set(index, value);
    }

    unsafe fn push_front(&mut self, value: T) {
        self.push(value, 0);
    }

    unsafe fn push_back(&mut self, value: T) {
        self.push(value, self.len);
    }

    unsafe fn pop(&mut self, index: usize) -> Result<T, String> {
        if self.len == 0 || index > self.len {
            Err(format!("Unable to pop at index {}, out of range (0..{})", index, self.len))
        } else {
            let out = self.arr.get(index);
            self.arr = self.arr.shift_from(index+1 , -1);
            self.len -= 1;

            eprintln!("\n\nself.len < self.cap / 2 = {} < {} = {}", self.len, self.cap / 2, self.len < self.cap / 2);

            if self.len < self.cap / 2 {
                self.arr = self.arr.shrink_pow2();
                self.cap = self.arr.size;
            }

            Ok(out)
        }
    }

    unsafe fn pop_front(&mut self) -> Result<T, String> {
        self.pop(0)
    }

    unsafe fn pop_back(&mut self) -> Result<T, String> {
        self.pop(self.len)
    }

    unsafe fn get(&mut self, index: usize) -> T {
        self.arr.get(index)
    }

    unsafe fn index(&mut self, value: T) -> Result<usize, String> {
        for index in 0..self.len {
            if self.arr.get(index) == value {
                return Ok(index);
            }
        }
        Err(format!("{} not in ArrayList", value))
    }

    unsafe fn count(&mut self, value: T) -> usize {
        let mut count = 0;
        for index in 0..self.len {
            if self.arr.get(index) == value {
                count += 1;
            }
        }
        count
    }
}
impl<T: Copy + Debug + Display + PartialEq> Display for ArrayList<T> {
    fn fmt(&self, f: &mut Formatter) -> Result_ {
        unsafe {
            if self.len == 0 {
                return write!(f, "[]");
            }
            write!(f, "[");
            for index in 0..self.len - 1 {
                write!(f, "{}, ", self.arr.get(index));
            }
            write!(f, "{}]", self.arr.get(self.len -1))
        }

    }
} impl<T: Copy + Debug + Display + PartialEq> Debug for ArrayList<T> {
    fn fmt(&self, f: &mut Formatter) -> Result_ {
        unsafe {
            write!(
                f,
                "ArrayList at {:p}:\n\tdata: {};\n\tlen: {};\n\tcap: {}",
                self.arr.ptr, self, self.len, self.cap
            )
        }
    }
}


fn main() {

    unsafe {
        let mut a = ArrayList::from_vec(&vec![1, 2, 3]);
        eprintln!("\nA: {:?}", a);
        eprintln!("a.pop(1)        -> {:?}", a.pop(1));
        eprintln!("\nA: {:?}", a);
        eprintln!("a.pop(1)        -> {:?}", a.pop(1));
        eprintln!("\nA: {:?}", a);
        eprintln!("a.pop(1)        -> {:?}", a.pop(1));
        eprintln!("\nA: {:?}", a);
        eprintln!("a.pop(1)        -> {:?}", a.pop(1));
        eprintln!("\nA: {:?}", a);
        eprintln!("a.push_front(2) -> {:?}", a.push_front(2));
        eprintln!("\nA: {:?}", a);
        eprintln!("a.push_back(3)  -> {:?}", a.push_back(3));
        eprintln!("\nA: {:?}", a);
        eprintln!("a.push_front(1) -> {:?}", a.push_front(1));
        eprintln!("\nA: {:?}", a);
        eprintln!("a.push_back(4)  -> {:?}", a.push_back(4));
        eprintln!("\nA: {:?}", a);
        eprintln!("\nA: {:?}", a);
    }

}