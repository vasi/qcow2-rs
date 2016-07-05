use std::io::Result;
use std::fs::File;

pub trait Pread {
    fn pread(&self, buf: &mut [u8], pos: u64) -> Result<usize>;
    fn pread_exact(&self, buf: &mut [u8], pos: u64) -> Result<()>;
}

pub trait Pwrite {
    fn pwrite(&self, buf: &mut [u8], pos: u64) -> Result<usize>;
    fn pwrite_all(&self, buf: &mut [u8], pos: u64) -> Result<()>;
}


#[cfg(unix)]

use std::io::{Error, ErrorKind};
use std::os::unix::io::AsRawFd;

extern crate nix;
use self::nix::sys::uio;

impl Pread for File {
    fn pread(&self, buf: &mut [u8], pos: u64) -> Result<usize> {
        let fd = self.as_raw_fd();
        uio::pread(fd, buf, pos as i64).map_err(From::from)
    }

    fn pread_exact(&self, mut buf: &mut [u8], mut pos: u64) -> Result<()> {
        while !buf.is_empty() {
            match self.pread(buf, pos) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    pos += n as u64;
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"))
        } else {
            Ok(())
        }
    }
}
