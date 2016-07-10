extern crate byteorder;
extern crate lru_cache;
extern crate num;
extern crate positioned_io;

mod error;
mod extension;
mod feature;
mod header;
mod int;
mod read;
pub use error::Error;
pub use read::Reader;

use std::fmt::{self, Debug, Formatter};
use std::result;
use std::sync::Mutex;

use byteorder::BigEndian;
use lru_cache::LruCache;
use positioned_io::{ReadAt, ByteIo};


const L2_CACHE_SIZE: usize = 32;

pub struct Qcow2<I>
    where I: ReadAt
{
    header: header::Header,
    io: ByteIo<I, BigEndian>,

    l2_cache: Mutex<LruCache<u64, u64>>,
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
            l2_cache: Mutex::new(LruCache::new(L2_CACHE_SIZE)),
        };
        try!(q.header.read(&mut q.io));
        Ok(q)
    }

    pub fn cluster_size(&self) -> u64 {
        self.header.cluster_size()
    }
    pub fn guest_size(&self) -> u64 {
        self.header.guest_size()
    }
}

impl<I> Debug for Qcow2<I>
    where I: ReadAt
{
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        f.debug_struct("Qcow2")
            .field("header", &self.header)
            .finish()
    }
}
