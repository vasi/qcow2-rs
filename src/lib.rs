extern crate byteorder;
use byteorder::BigEndian;

extern crate positioned_io;
use positioned_io::{ReadAt, ByteIo};

mod error;
mod header;
mod int;
pub use error::Error;

pub struct Qcow2<I>
    where I: ReadAt
{
    header: header::Header,
    io: ByteIo<I, BigEndian>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl<I> Qcow2<I>
    where I: ReadAt
{
    pub fn open(io: I) -> Result<Self> {
        let mut io: ByteIo<_, BigEndian> = ByteIo::new(io);
        let mut header: header::Header = Default::default();
        try!(header.read(&mut io));

        Ok(Qcow2 {
            header: header,
            io: io,
        })
    }
}
