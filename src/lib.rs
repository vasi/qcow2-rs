extern crate byteorder;
use byteorder::BigEndian;

extern crate positioned_io;
use positioned_io::{ReadAt, ByteIo};

#[macro_use]
extern crate bitflags;

mod error;
mod header;
mod int;
pub use error::Error;

pub struct Qcow2<I>
    where I: ReadAt
{
    header: header::Header,
    io: ByteIo<I, BigEndian>,
    write: bool,
}

pub type Result<T> = std::result::Result<T, Error>;

impl<I> Qcow2<I>
    where I: ReadAt
{
    pub fn open(io: I, write: bool) -> Result<Self> {
        let mut io: ByteIo<_, BigEndian> = ByteIo::new(io);
        let mut q = Qcow2 {
            header: Default::default(),
            io: io,
            write: write,
        };
        try!(q.header.read(&mut q.io, write));
        Ok(q)
    }
}
