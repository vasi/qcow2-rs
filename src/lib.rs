mod pread;
pub use pread::{Pread, Pwrite};

mod error;
pub use error::Error;

mod header;
use header::Header;

extern crate byteorder;
use byteorder::BigEndian;

use std::mem;

const MAGIC: u32 = 0x514649fb;

pub struct Qcow2<I>
    where I: Pread
{
    header: Header,
    io: self::io::Pread,
}

pub type Result<T> = std::result::Result<T, Error>;

impl<I> Qcow2<I>
    where I: io::Pread
{
    pub fn open(io: I) -> Result<Qcow2<I>> {
        let mut buf = vec![0; mem::size_of::<Header>()];
        try!(io.pread_exact(&mut buf, 0));

        let header: Header = Default::default();
        let mut curs = std::io::Cursor::new(buf);
        // header.magic = try!(curs.read_)

        Ok(Qcow2 {
            header: header,
            io: io,
        })
    }
}
