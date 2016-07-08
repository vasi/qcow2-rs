use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Read};
use std::mem::size_of;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::result;

use byteorder::BigEndian;
use num::Integer as NumInteger;
use positioned_io::{ByteIo, ReadAt, ReadInt, Cursor};

use super::{Result, Error};
use super::int::Integer;
use super::extension::{Extension,FeatureNameTable};
use super::feature::{Feature, FeatureKind};

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

const INCOMPATIBLE_DIRTY: u64 = 0b1;
const INCOMPATIBLE_CORRUPT: u64 = 0b10;
const COMPATIBLE_LAZY_REFCOUNTS: u64 = 0b1;
const AUTOCLEAR_BITMAPS: u64 = 0b1;

static INCOMPATIBLE_NAMES: &'static [&'static str] = &["dirty", "corrupt"];
static COMPATIBLE_NAMES: &'static [&'static str] = &["lazy refcounts"];
static AUTOCLEAR_NAMES: &'static [&'static str] = &["bitmaps"];

pub struct HeaderV3 {
    pub incompatible: Feature,
    pub compatible: Feature,
    pub autoclear: Feature,

    pub refcount_order: u32,
    pub header_length: u32,

    pub feature_name_table: FeatureNameTable,
    // pub extensions: ...,

    // TODO: Once we support backing files, read/write this.
    pub backing_file_name: String,
}
impl Debug for HeaderV3 {
    fn fmt(&self, fmt: &mut Formatter) -> result::Result<(), fmt::Error> {
        fmt.debug_struct("HeaderV3")
        .field("incompatible", &self.incompatible.debug(&self.feature_name_table))
        .field("compatible", &self.incompatible.debug(&self.feature_name_table))
        .field("autoclear", &self.incompatible.debug(&self.feature_name_table))
        .field("refcount_order", &self.refcount_order)
        .field("header_length", &self.header_length)
        .field("feature_name_table", &self.feature_name_table)
        .field("backing_file_name", &self.backing_file_name)
            .finish()
    }
}
impl Default for HeaderV3 {
    fn default() -> Self {
        HeaderV3 {
            incompatible: Feature::new(FeatureKind::Incompatible, INCOMPATIBLE_NAMES),
            compatible: Feature::new(FeatureKind::Compatible, COMPATIBLE_NAMES),
            autoclear: Feature::new(FeatureKind::Autoclear, AUTOCLEAR_NAMES),
            refcount_order: 0,
            header_length: 0,
            backing_file_name: String::new(),
            feature_name_table: FeatureNameTable::default(),
        }
    }
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

    // Read the version 3 header.
    fn read_v3<I: Read>(&mut self, io: &mut ByteIo<I, BigEndian>) -> Result<()> {
        self.v3.incompatible.set(try!(io.read_u64()));
        self.v3.compatible.set(try!(io.read_u64()));
        self.v3.autoclear.set(try!(io.read_u64()));
        self.v3.refcount_order = try!(io.read_u32());
        self.v3.header_length = try!(io.read_u32());

        // TODO
        // self.v3.extensions = HashMap::new();
        // loop {
        //     let extid = try!(io.read_u32());
        //     if extid == 0 {
        //         break;
        //     }
        //
        //     let len = try!(io.read_u32()) as usize;
        //     let mut buf = vec![0; len.to_multiple_of(8)];
        //     try!(io.read_exact(&mut buf));
        //     buf.truncate(len);
        //     self.v3.extensions.insert(extid, buf);
        // }
        Ok(())
    }

    fn validate_v3(&mut self) -> Result<()> {
        // TODO
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
        try!(self.validate_v3());
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
