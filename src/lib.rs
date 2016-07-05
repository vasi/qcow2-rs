extern crate byteorder;
use byteorder::BigEndian;

mod error;
mod header;
mod io;
mod pread;
pub use error::Error;
use io::{Io, ReadInt};
pub use pread::{Pread, Pwrite};

use std::io::Cursor;

const MAGIC: u32 = 0x514649fb;

pub struct Qcow2<I>
    where I: Pread
{
    header: header::Header,
    io: Io<I, BigEndian>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl<I> Qcow2<I>
    where I: Pread
{
    pub fn open(io: I) -> Result<Self> {
        let mut buf = vec![0; std::mem::size_of::<header::Header>()];
        try!(io.pread_exact(&mut buf, 0));

        let mut header: header::Header = Default::default();
        let mut curs: Io<_, BigEndian> = Io::new(Cursor::new(buf));

        header.magic = try!(curs.read_u32());
        if header.magic != MAGIC {
            return Err(Error::FileType);
        }

        // header.magic = try!(curs.read_)

        Ok(Qcow2 {
            header: header,
            io: Io::new(io),
        })
    }
}
