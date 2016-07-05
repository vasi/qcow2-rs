extern crate byteorder;
use self::byteorder::{ByteOrder};

use std;
use std::marker::PhantomData;
use super::pread::Pread;

// Allow simple reading of integer types.
pub trait ReadInt {
    fn read_u32(&mut self) -> std::io::Result<u32>;
    fn read_u64(&mut self) -> std::io::Result<u64>;
}
pub trait PreadInt {
    fn pread_uint(&mut self, nbytes: usize, pos: u64) -> std::io::Result<u64>;
    fn pread_u64(&mut self, pos: u64) -> std::io::Result<u64> {
        self.pread_uint(8, pos)
    }
}

// Wrapper to force endianness.
pub struct Io<I, E> where I: Pread, E: ByteOrder {
    io: I,
    endian: PhantomData<E>,
}

impl<I, E> Io<I, E> where I: Pread, E: ByteOrder {
    pub fn new(io: I) -> Self {
        Io { io: io, endian: PhantomData }
    }
}

impl<I, E> PreadInt for Io<I, E> where I: Pread, E: ByteOrder {
    fn pread_uint(&mut self, nbytes: usize, pos: u64) -> std::io::Result<u64> {
        let mut buf = vec![0; nbytes];
        try!(self.io.pread_exact(&mut buf, pos));
        Ok(E::read_uint(&buf, nbytes))
    }
}
