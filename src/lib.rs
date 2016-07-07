extern crate byteorder;
use byteorder::BigEndian;

extern crate positioned_io;
use positioned_io::{ReadAt, ByteIo};

#[macro_use]
extern crate bitflags;

use std::fmt::{self, Debug, Formatter};
use std::result;

mod error;
mod header;
mod int;
pub use error::Error;

pub struct Qcow2<I>
    where I: ReadAt
{
    pub header: header::Header,
    io: ByteIo<I, BigEndian>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl<I> Qcow2<I>
    where I: ReadAt
{
    pub fn open(io: I) -> Result<Self> {
        let io: ByteIo<_, BigEndian> = ByteIo::new(io);
        let mut q = Qcow2 {
            header: Default::default(),
            io: io,
        };
        try!(q.header.read(&mut q.io));
        Ok(q)
    }
}

impl<I> Debug for Qcow2<I> where I: ReadAt {
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        f.debug_struct("Qcow2")
            .field("header", &self.header)
            .finish()
    }
}
