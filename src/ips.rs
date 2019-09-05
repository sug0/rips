pub mod mem;

use std::ops::{RangeBounds, Bound};
use std::io::Read;

macro_rules! shl {
    ($x:expr, $s:expr) => {
        ($x as usize) << $s
    }
}

macro_rules! parse_offset {
    ($x:expr) => {
        (shl!($x[0], 16) | shl!($x[1], 8) | shl!($x[2], 0)) as u64
    }
}

macro_rules! parse_size {
    ($x:expr) => {
        shl!($x[0], 8) | shl!($x[1], 0)
    }
}

pub struct RecordIterator<R> {
    first_read: bool,
    patch: R,
    buf: Vec<u8>,
    chunks: mem::Owner,
}

pub enum Data {
    RLE { byte: u8, size: usize },
    Chunk(mem::BorrowedMut),
}

pub struct Record {
    _off: u64,
    _data: Data,
}

impl Record {
    #[inline]
    pub fn data(self) -> Data {
        self._data
    }

    #[inline]
    pub fn off(&self) -> u64 {
        self._off
    }
}

impl<R: Read> RecordIterator<R> {
    #[inline]
    pub fn new(patch: R) -> Self {
        RecordIterator::new_with_bufsize(patch, 512)
    }

    #[inline]
    pub fn new_with_bufsize(patch: R, bufsize: usize) -> Self {
        RecordIterator {
            first_read: true,
            buf: vec![0; bufsize],
            chunks: mem::Owner::new(0),
            patch,
        }
    }

    #[inline]
    fn read_exact<B>(&mut self, bounds: B) -> Option<()>
        where B: RangeBounds<usize>
    {
        let lo = match bounds.start_bound() {
            Bound::Included(x) => *x,
            _                  => 0,
        };
        let hi = match bounds.end_bound() {
            Bound::Excluded(x) => *x,
            _                  => self.buf.len(),
        };
        self.patch
            .read_exact(&mut self.buf[lo..hi])
            .ok()
    }

    #[inline]
    fn check_header(&mut self) -> Option<bool> {
        self.read_exact(..5)?;
        Some(&self.buf[..5] == b"PATCH")
    }
}

impl<R: Read> Iterator for RecordIterator<R> {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first_read {
            let valid = self.check_header()?;
            if !valid {
                return None;
            }
            self.first_read = false;
        }

        self.read_exact(..3)?;

        if &self.buf[..3] == b"EOF" {
            return None;
        }

        self.read_exact(3..5)?;

        let off = parse_offset!(&self.buf[..3]);
        let size = parse_size!(&self.buf[3..5]);

        // RLE encoded
        if size == 0 {
            self.read_exact(..3)?;
            let size = parse_size!(&self.buf[..2]);

            return Some(Record {
                _off: off,
                _data: Data::run_length_encoded(self.buf[2], size),
            });
        }

        let mut borrow = self.chunks.slice_mut(..size);
        let rec = borrow.get().unwrap();
        let mut left = size;

        while left != 0 {
            let n = if left >= self.buf.len() {
                self.read_exact(..)?;
                self.buf.len()
            } else {
                self.read_exact(..left)?;
                left
            };

            slice_copy(&mut rec[size-left..], &self.buf[..n]);
            left -= n;
        }

        Some(Record {
            _off: off,
            _data: Data::chunk(borrow),
        })
    }
}

impl Data {
    #[inline]
    fn run_length_encoded(byte: u8, size: usize) -> Self {
        Data::RLE { byte, size }
    }

    #[inline]
    fn chunk(chk: mem::BorrowedMut) -> Self {
        Data::Chunk(chk)
    }
}

#[inline]
fn slice_copy<T: Copy>(dst: &mut [T], src: &[T]) {
    (&mut dst[..src.len()]).copy_from_slice(src)
}
