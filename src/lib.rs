#![warn(missing_docs)]

//! This crate can parse and read qcow2 virtual disks, as used by qemu and other emulators.
//!
//! [Qcow2](https://en.wikipedia.org/wiki/Qcow) is a flexible format for disk images, that only
//! allocates space as needed. It has many other interesting features.
//!
//! The following featuers are supported by this crate:
//!
//!  * Reading blocks of virtual disk data.
//!  * Reading data that is not aligned to block boundaries.
//!  * Parsing and validation of the header.
//!  * Reporting the names of any unsupported features, using the "feature name table" extension.
//!  * Basic caching of guest data locations, so nearby reads will be fast.
//!
//! These features are not yet supported, but should be easy to add:
//!
//! * Listing and reading snapshots.
//! * Reading version 2, currently only version 3 is supported.
//! * Reading compressed data.
//! * Backing file support, so you can chain qcow2 files together.
//! * Reporting information about images.
//!
//! These features are harder, or less interesting to me. Patches welcome!
//!
//! * Reading encrypted qcow2 files.
//! * Writing virtual disk data.
//! * Repairing the disk if refcounts are out of date.
//! * Compacting the virtual disk so it takes less space.
//! * Maintaining a "dirty bitmap" to make backups faster.
//! * Creating new qcow2 images.
//! * Creating new snapshots.
//! * Checking qcow2 images for inconsistencies.
//! * Merging images into their backing file.
//! * Resizing images.
//!
//! The repository for this crate is at https://github.com/vasi/qcow2-rs

extern crate byteorder;
extern crate lru_cache;
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

/// A qcow2 image.
///
/// # Examples
///
/// ```no_run
/// extern crate positioned_io;
/// extern crate qcow2;
///
/// # use std::fs::File;
/// use positioned_io::ReadAt;
/// use qcow2::Qcow2;
///
/// # fn foo() -> qcow2::Result<()> {
///
/// // Open a file.
/// let file = try!(File::open("image.qcow2"));
/// let qcow = try!(Qcow2::open(file));
///
/// // Read some data.
/// let reader = try!(qcow.reader());
/// let mut buf = vec![0, 4096];
/// try!(reader.read_exact_at(5 * 1024 * 1024, &mut buf));
///
/// # Ok(()) } fn main() { foo().unwrap(); }
/// ```
pub struct Qcow2<I>
    where I: ReadAt
{
    header: header::Header,
    io: ByteIo<I, BigEndian>,

    l2_cache: Mutex<LruCache<u64, u64>>,
}

/// The result type for operations on qcow2 images.
pub type Result<T> = std::result::Result<T, Error>;

impl<I> Qcow2<I>
    where I: ReadAt
{
    /// Open a source of data as a qcow2 image.
    ///
    /// Usually the data source `io` will be a file.
    pub fn open(io: I) -> Result<Self> {
        let io: ByteIo<_, BigEndian> = ByteIo::new(io);
        let mut q = Qcow2 {
            header: Default::default(),
            io,
            l2_cache: Mutex::new(LruCache::new(L2_CACHE_SIZE)),
        };
        try!(q.header.read(&mut q.io));
        Ok(q)
    }

    /// Get the size of each block of this qcow2 image.
    pub fn cluster_size(&self) -> u64 {
        self.header.cluster_size()
    }

    /// Get the size of the virtual image.
    ///
    /// This is likely to differ from the size of the qcow2 file itself, since the file can grow.
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
