#![feature(alloc_layout_extra)]

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    fmt::{Debug, Display, Formatter, Result as Result_},
    ptr::{copy, read, write},
};

fn get_next_pow2(num: usize) -> usize {
    let mut n = num as u64;
    if n == 0 {
        return 1;
    }
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n |= n >> 32;
    return (n + 1) as usize;
}

struct Array<T>
where
    T: Copy + Display,
{
    ptr: *mut T,
    size: usize,
}
impl<T: Display + Copy + PartialEq> Array<T> {
    unsafe fn from(arr: *mut T, size: usize) -> Array<T> {
        let buf: Array<T> = Array::new(size);
        copy(arr, buf.ptr, size);
        buf
    }
} impl<T: Copy + Display> Array<T> {
    unsafe fn new(size: usize) -> Array<T> {
        let layout = Layout::array::<T>(size).unwrap();
        let ptr: *mut T = alloc_zeroed(layout) as *mut T;
        Array {
            ptr: ptr,
            size: size,
        }
    }

    unsafe fn take_vec(v: Vec<T>) -> Array<T> {
        let mut v = v;
        return Self::from_vec(&mut v);
    }

    unsafe fn from_vec(v: &Vec<T>) -> Array<T> {
        let arr = Array::new(v.len());
        copy(v.as_ptr(), arr.ptr, v.len());
        return arr;
    }

    unsafe fn from_raw_buf(buf: *mut T, size: usize) -> Array<T> {
        let arr = Array::new(size);
        copy(buf, arr.ptr, size);
        return arr;
    }

    unsafe fn resize(&mut self, size: usize) {
        let buf = self.copy();

        dealloc(self.ptr as *mut u8, Layout::array::<T>(self.size).unwrap());

        self.ptr = alloc_zeroed(Layout::array::<T>(size).unwrap()) as *mut T;
        self.size = size;

        copy(buf.ptr, self.ptr, size);

        // I could use realloc (below) but I want to clear the memory where I want to move the data to
        // realloc(self.ptr as *mut u8, Layout::array::<T>(self.size).unwrap(), size);
    }

    unsafe fn copy(&self) -> Array<T> {
        Array::from_raw_buf(self.ptr, self.size)
    }

    fn len(&self) -> usize {
        self.size
    }

    unsafe fn clear(&mut self) {
        let layout = Layout::array::<T>(self.size).unwrap();
        dealloc(self.ptr as *mut u8, layout);
        self.ptr = alloc_zeroed(layout) as *mut T;
    }

    unsafe fn get(&self, index: usize) -> Result<T, String> {
        if index >= self.size {
            Err(format!(
                "Unable to get value at index {}, index out of range",
                index
            ))
        } else {
            Ok(read(self.ptr.add(index)))
        }
    }

    unsafe fn set(&self, index: usize, value: T) -> Result<(), String> {
        if index >= self.size {
            Err(format!(
                "Unable to set {} to {}, index out of range",
                index, value
            ))
        } else {
            write(self.ptr.add(index), value);
            Ok(())
        }
    }

    unsafe fn shift_from(&mut self, index: usize, amt: isize) {
        if amt == 0 {
            return;
        }

        let buf = self.copy();
        self.clear();

        

        return if amt < 0 {
            // before the shift items
            copy(
                buf.ptr,
                self.ptr,
                index - amt.abs() as usize
            );

            // the rest of the items
            copy(
                buf.ptr.add(index),
                self.ptr.add(index - amt.abs() as usize),
                buf.len() -index,                
            );

        } else {
            // before the gap
            copy(
                buf.ptr,
                self.ptr, 
                index,
            );

            // the rest, maybe
            if index + (amt as usize) < buf.len() {
                copy(
                    buf.ptr.add(index),
                    self.ptr.add(index + amt as usize),
                    {
                        buf.len() - index - amt as usize
                    }
                );
            }
        }
    }

}
impl<T: Copy + Display> Drop for Array<T> {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr as *mut u8, Layout::array::<T>(self.size).unwrap());
        }
    }
}
impl<T: Copy + Display> Debug for Array<T> {
    fn fmt(&self, f: &mut Formatter) -> Result_ {
        if self.size == 0 {
            return write!(f, "[]");
        }
        write!(f, "[").unwrap();
        unsafe {
            for index in 0..self.size - 1 {
                write!(f, "{}, ", self.get(index).unwrap()).unwrap();
            }
            write!(f, "{}]", self.get(self.size - 1).unwrap())
        }
    }
}

struct ArrayListIter<T>
where
    T: Copy + PartialEq + Display,
{
    list: ArrayList<T>,
    len: usize,
    current: usize,
}
impl<T: Copy + PartialEq + Display> ArrayListIter<T> {
    fn new(list: ArrayList<T>) -> ArrayListIter<T> {
        let len = list.len;
        ArrayListIter {
            list: list,
            len: len,
            current: 0,
        }
    }
}
impl<T: Copy + PartialEq + Display> Iterator for ArrayListIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.len {
            self.current += 1;
            unsafe { Some(self.list.get(self.current - 1)) }
        } else {
            None
        }
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
            arr: Array::from_vec(v),
            len: v.len(),
            cap: v.len(),
        }
    }

    unsafe fn get(&self, index: usize) -> T {
        match self.arr.get(index) {
            Ok(n) => n,
            Err(e) => panic!(e),
        }
    }

    unsafe fn set(&self, index: usize, value: T) -> Result<(), String> {
        self.arr.set(index, value)
    }

    unsafe fn push(&mut self, index: usize, value: T) {
        if self.len >= self.cap {
            self.cap = get_next_pow2(self.cap);
            self.arr.resize(self.cap);
        }
        self.arr.shift_from(index, 1);
        match self.arr.set(index, value) {
            Err(e) => panic!(e),
            _      => (),
        };
        self.len += 1;
    }

    unsafe fn push_front(&mut self, value: T) {
        self.push(0, value);
    }

    unsafe fn push_back(&mut self, value: T) {
        self.push(self.len, value);
    }

    unsafe fn pop(&mut self, index: usize) -> Result<T, String> {
        if self.len < self.cap / 2 {
            self.cap = self.cap / 2;
        } if index > self.len {
            Err(format!("index {} out of range: 0..{}", index, self.len))
        } else {
            let out: T = match self.arr.get(index) {
                Ok(n)  => n,
                Err(e) => panic!(e),
            };

            self.arr.shift_from(index, -1);
            self.len -= 1;

            Ok(out)
        }
    }

    unsafe fn pop_front(&mut self) -> Result<T, String> {
        self.pop(0)
    }

    unsafe fn pop_back(&mut self) -> Result<T, String> {
        self.pop(self.len)
    }
}
impl<T: Copy + Debug + Display + PartialEq> Display for ArrayList<T> {
    fn fmt(&self, f: &mut Formatter) -> Result_ {
        unsafe {
            if self.len == 0 {
                return write!(f, "[]");
            }
            write!(f, "[").unwrap();
            for index in 0..self.len - 1 {
                write!(f, "{}, ", self.get(index)).unwrap();
            }
            write!(f, "{}]", self.get(self.len - 1))
        }
    }
}
impl<T: Copy + Debug + Display + PartialEq> Debug for ArrayList<T> {
    fn fmt(&self, f: &mut Formatter) -> Result_ {
        write!(
            f,
            "ArrayList at {:p}:\n\tdata: {};\n\tlen: {};\n\tcap: {};",
            self.arr.ptr, self, self.len, self.cap
        )
    }
}
impl<T: Copy + Display + PartialEq> IntoIterator for ArrayList<T> {
    type Item = T;

    type IntoIter = ArrayListIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        ArrayListIter::new(self)
    }
}

fn main() {
    unsafe {
    }
}
