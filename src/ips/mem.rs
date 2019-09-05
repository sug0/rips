use std::cell::UnsafeCell;
use std::rc::{Rc, Weak};
use std::ops::{RangeBounds, Bound};

pub struct Owner {
    inner: Rc<UnsafeCell<Vec<u8>>>,
}

pub struct Borrowed {
    lo: usize,
    hi: usize,
    inner: Weak<UnsafeCell<Vec<u8>>>,
}

pub struct BorrowedMut(Borrowed);

impl Owner {
    #[inline]
    pub fn new(len: usize) -> Self {
        Owner::of(vec![0; len])
    }

    #[inline]
    pub fn of(vec: Vec<u8>) -> Self {
        Owner {
            inner: Rc::new(UnsafeCell::new(vec))
        }
    }
    
    #[inline]
    pub fn slice<B>(&self, bounds: B) -> Borrowed
        where B: RangeBounds<usize>
    {
        let (lo, hi) = self.indices(bounds);
        Borrowed {
            lo, hi,
            inner: Rc::downgrade(&self.inner),
        }
    }
    
    #[inline]
    pub fn slice_mut<B>(&mut self, bounds: B) -> BorrowedMut
        where B: RangeBounds<usize>
    {
        BorrowedMut(self.slice(bounds))
    }

    #[inline]
    fn indices<B>(&self, bounds: B) -> (usize, usize)
        where B: RangeBounds<usize>
    {
        let lo = match bounds.start_bound() {
            Bound::Included(x) => *x,
            _ => 0,
        };
        let hi = match bounds.end_bound() {
            Bound::Excluded(x) => *x,
            _ => unsafe { (*self.inner.get()).len() },
        };
        (lo, hi)
    }
}

impl Borrowed {
    #[inline]
    pub fn get(&self) -> Option<&[u8]> {
        unsafe {
            self.get_unsafe()
                .map(|x| &*x)
        }
    }

    #[inline]
    unsafe fn get_unsafe(&self) -> Option<&mut [u8]> {
        self.inner
            .upgrade()
            .map(|cell| {
                let len = self.hi - self.lo;
                let vec = &mut *cell.get();

                if len > vec.len() {
                    vec.resize(len, 0)
                }

                &mut vec[self.lo..self.hi]
            })
    }
}

impl BorrowedMut {
    #[inline]
    pub fn get(&mut self) -> Option<&mut [u8]> {
        unsafe { self.0.get_unsafe() }
    }
}
