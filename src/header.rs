extern crate byteorder;
use byteorder::BigEndian;

extern crate positioned_io;
use positioned_io::{ByteIo, ReadAt, Cursor};

extern crate num;
use self::num::Integer as NumInteger;

use super::{Result, Error};
use super::int::Integer;

use std::io::{self, Read};
use std::mem::size_of;
use std::ops::DerefMut;

const MAGIC: u32 = 0x514649fb;
const SUPPORTED_VERSION: u32 = 3;

// Common header for all versions.
#[repr(C)]
#[derive(Default)]
pub struct HeaderBase {
    pub magic: u32,
    pub version: u32,
    pub backing_file_offset: u64,
    pub backing_file_size: u32,
    pub cluster_bits: u32,
    pub size: u64,
    pub crypt_method: u32,
    pub l1_size: u32,
    pub l1_table_offset: u64,
    pub refcount_table_offset: u64,
    pub refcount_table_clusters: u32,
    pub nb_snapshots: u32,
    pub snapshots_offset: u64, // TODO: version 3 fields
}

#[derive(Default)]
pub struct Header {
    base: HeaderBase,
}

impl Header {
    fn read_base<I: Read>(&mut self, io: &mut ByteIo<I, BigEndian>) -> io::Result<()> {
        self.base.magic = try!(io.read_u32());
        self.base.version = try!(io.read_u32());
        self.base.backing_file_offset = try!(io.read_u64());
        self.base.backing_file_size = try!(io.read_u32());
        self.base.cluster_bits = try!(io.read_u32());
        self.base.size = try!(io.read_u64());
        self.base.crypt_method = try!(io.read_u32());
        self.base.l1_size = try!(io.read_u32());
        self.base.l1_table_offset = try!(io.read_u64());
        self.base.refcount_table_offset = try!(io.read_u64());
        self.base.refcount_table_clusters = try!(io.read_u32());
        self.base.nb_snapshots = try!(io.read_u32());
        self.base.snapshots_offset = try!(io.read_u64());
        Ok(())
    }

    fn validate_base(&self) -> Result<()> {
        if self.base.magic != MAGIC {
            return Err(Error::FileType);
        }
        if self.base.version != SUPPORTED_VERSION {
            return Err(Error::Version(self.base.version, SUPPORTED_VERSION));
        }
        if self.base.backing_file_offset != 0 {
            return Err(Error::UnsupportedFeature("backing file".to_owned()));
        }
        if self.base.cluster_bits < 9 || self.base.cluster_bits > 22 {
            return Err(Error::FileFormat(format!("bad cluster_bits {}", self.base.cluster_bits)))
        }
        if self.base.crypt_method != 0 {
            return Err(Error::UnsupportedFeature("encryption".to_owned()));
        }
        if self.base.l1_size as u64 != self.l1_entries() {
            return Err(Error::FileFormat("bad L1 entry count".to_owned()));
        }
        if !self.base.l1_table_offset.is_multiple_of(&self.cluster_size()) {
            return Err(Error::FileFormat("bad L1 offset".to_owned()));
        }
        if !self.base.refcount_table_offset.is_multiple_of(&self.cluster_size()) {
            return Err(Error::FileFormat("bad refcount offset".to_owned()));
        }
        if !self.base.snapshots_offset.is_multiple_of(&self.cluster_size()) {
            return Err(Error::FileFormat("bad snapshots offset".to_owned()));
        }
        Ok(())
    }

    pub fn read<I: ReadAt>(&mut self, io: &mut ByteIo<I, BigEndian>) -> Result<()> {
        // Get a cursor to read from.
        let mut curs = Cursor::new(io.deref_mut());
        let mut io: ByteIo<_, BigEndian> = ByteIo::new(&mut curs);

        // Read the header.
        try!(self.read_base(&mut io));
        try!(self.validate_base());
        Ok(())
    }

    // How big is each cluster, in bytes?
    pub fn cluster_size(&self) -> u64 {
        1 << self.base.cluster_bits
    }

    // How many virtual blocks can there be?
    pub fn max_virtual_blocks(&self) -> u64 {
        self.base.size.div_ceil(&self.cluster_size())
    }

    // How many entries are in an L2?
    pub fn l2_entries(&self) -> u64 {
        self.cluster_size() / size_of::<u64>() as u64
    }

    // How many entries are in an L1?
    pub fn l1_entries(&self) -> u64 {
        self.max_virtual_blocks().div_ceil(&self.l2_entries())
    }
}
