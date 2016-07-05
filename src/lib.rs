extern crate byteorder;

mod error;
mod header;
mod io;
use io::Io;
mod pread;
pub use pread::{Pread, Pwrite};

const MAGIC: u32 = 0x514649fb;

pub struct Qcow2<I>
    where I: Pread
{
    header: header::Header,
    io: Io<I, byteorder::BigEndian>,
}

pub type Result<T> = std::result::Result<T, error::Error>;

impl<I> Qcow2<I>
    where I: Pread
{
    pub fn open(io: I) -> Result<Self> {
        let mut buf = vec![0; std::mem::size_of::<header::Header>()];
        try!(io.pread_exact(&mut buf, 0));

        let header: header::Header = Default::default();
        let mut curs = std::io::Cursor::new(buf);
        // header.magic = try!(curs.read_)

        Ok(Qcow2 {
            header: header,
            io: Io::new(io),
        })
    }
}
