mod error;
mod header;
mod io;
mod pread;
pub use pread::{Pread, Pwrite};

const MAGIC: u32 = 0x514649fb;

pub struct Qcow2 {
    header: header::Header,
    io: io::Pread,
}

pub type Result<T> = std::result::Result<T, error::Error>;

impl Qcow2 {
    pub fn open<I>(io: I) -> Result<Qcow2> {
        let mut buf = vec![0; std::mem::size_of::<header::Header>()];
        try!(io.pread_exact(&mut buf, 0));

        let header: header::Header = Default::default();
        let mut curs = std::io::Cursor::new(buf);
        // header.magic = try!(curs.read_)

        Ok(Qcow2 {
            header: header,
            io: io,
        })
    }
}
