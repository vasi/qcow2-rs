use byteorder::BigEndian;
use positioned_io::{ByteIo, ReadAt, Cursor};
use num::Integer as NumInteger;

use super::{Result, Error};
use super::int::Integer;
use super::features::Features;

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{self, Read};
use std::mem::size_of;
use std::ops::DerefMut;
use std::rc::Rc;


const MAGIC: u32 = 0x514649fb;
const SUPPORTED_VERSION: u32 = 3;

// Common header for all versions.
#[repr(C)]
#[derive(Default, Debug)]
pub struct HeaderCommon {
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
    pub snapshots_offset: u64,
}

pub trait HeaderExtension : Debug {
    fn identifier(&self) -> u32;
    fn read_extension(&mut self, buf: &[u8]) -> Result<()>;
    // TODO: write
}

#[derive(Default, Debug)]
pub struct HeaderV3 {
    pub known_extensions: HashMap<u32, Box<HeaderExtension>>,
    pub refcount_order: u32,
    pub header_length: u32,
    pub unknown_extensions: HashMap<u32, Vec<u8>>,

    // TODO: Once we support backing files, read/write this.
    pub backing_file_name: String,
}

#[derive(Default, Debug)]
pub struct Header {
    pub c: HeaderCommon,
    pub v3: HeaderV3,
}

impl Header {
    // Read the common header.
    fn read_common<I: Read>(&mut self, io: &mut ByteIo<I, BigEndian>) -> io::Result<()> {
        self.c.magic = try!(io.read_u32());
        self.c.version = try!(io.read_u32());
        self.c.backing_file_offset = try!(io.read_u64());
        self.c.backing_file_size = try!(io.read_u32());
        self.c.cluster_bits = try!(io.read_u32());
        self.c.size = try!(io.read_u64());
        self.c.crypt_method = try!(io.read_u32());
        self.c.l1_size = try!(io.read_u32());
        self.c.l1_table_offset = try!(io.read_u64());
        self.c.refcount_table_offset = try!(io.read_u64());
        self.c.refcount_table_clusters = try!(io.read_u32());
        self.c.nb_snapshots = try!(io.read_u32());
        self.c.snapshots_offset = try!(io.read_u64());
        Ok(())
    }

    // Validate the common header.
    fn validate_common(&self) -> Result<()> {
        if self.c.magic != MAGIC {
            return Err(Error::FileType);
        }
        if self.c.version != SUPPORTED_VERSION {
            return Err(Error::Version(self.c.version, SUPPORTED_VERSION));
        }
        if self.c.backing_file_offset != 0 {
            return Err(Error::UnsupportedFeature("backing file".to_owned()));
            // TODO: Validate backing file name position.
        }
        if self.c.cluster_bits < 9 || self.c.cluster_bits > 22 {
            return Err(Error::FileFormat(format!("bad cluster_bits {}", self.c.cluster_bits)));
        }
        if self.c.crypt_method != 0 {
            return Err(Error::UnsupportedFeature("encryption".to_owned()));
        }
        if self.c.l1_size as u64 != self.l1_entries() {
            return Err(Error::FileFormat("bad L1 entry count".to_owned()));
        }
        if !self.c.l1_table_offset.is_multiple_of(&self.cluster_size()) {
            return Err(Error::FileFormat("bad L1 offset".to_owned()));
        }
        if !self.c.refcount_table_offset.is_multiple_of(&self.cluster_size()) {
            return Err(Error::FileFormat("bad refcount offset".to_owned()));
        }
        if !self.c.snapshots_offset.is_multiple_of(&self.cluster_size()) {
            return Err(Error::FileFormat("bad snapshots offset".to_owned()));
        }
        Ok(())
    }

    fn extensions(&mut self) -> Vec<Rc<HeaderExtension>> {
        vec![Rc::new(&mut self.v3.features)]
    }

    // Read the version 3 header.
    fn read_v3<I: Read>(&mut self, io: &mut ByteIo<I, BigEndian>) -> Result<()> {
        try!(self.v3.features.read(io));
        self.v3.refcount_order = try!(io.read_u32());
        self.v3.header_length = try!(io.read_u32());
        loop {
            let extid = try!(io.read_u32());
            if extid == 0 {
                break;
            }

            // Read the data.
            let len = try!(io.read_u32()) as usize;
            let mut buf = vec![0; len.to_multiple_of(8)];
            try!(io.read_exact(&mut buf));
            buf.truncate(len);

            // Look for a handler.
            for ext in self.extensions() {

            }
            // match extid {
            //     HeaderExtension::FeatureNameTable as u32 => try!(self.v3.features.read_names(buf)),
            //     _ => { self.v3.unknown_extensions.insert(extid, buf); () }
            // }

        }
        Ok(())
    }

    pub fn read<I: ReadAt>(&mut self, io: &mut ByteIo<I, BigEndian>) -> Result<()> {
        // The headers are best read sequentially, rather than positioned.
        // So get a sequential cursor to read from.
        let mut curs = Cursor::new(io.deref_mut());
        let mut io: ByteIo<_, BigEndian> = ByteIo::new(&mut curs);

        // Read the header.
        try!(self.read_common(&mut io));
        try!(self.validate_common());
        try!(self.read_v3(&mut io));
        Ok(())
    }

    // How big is each cluster, in bytes?
    pub fn cluster_size(&self) -> u64 {
        1 << self.c.cluster_bits
    }

    // How many virtual blocks can there be?
    pub fn max_virtual_blocks(&self) -> u64 {
        self.c.size.div_ceil(self.cluster_size())
    }

    // How many entries are in an L2?
    pub fn l2_entries(&self) -> u64 {
        self.cluster_size() / size_of::<u64>() as u64
    }

    // How many entries are in an L1?
    pub fn l1_entries(&self) -> u64 {
        self.max_virtual_blocks().div_ceil(self.l2_entries())
    }
}
