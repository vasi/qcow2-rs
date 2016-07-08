use std::cell::{RefCell, Ref};
use std::collections::HashSet;
use std::fmt::{self, Debug, Formatter};
use std::io::Read;
use std::mem::size_of;
use std::ops::DerefMut;
use std::rc::Rc;
use std::result;

use byteorder::BigEndian;
use num::Integer as NumInteger;
use positioned_io::{ByteIo, ReadAt, ReadInt, Cursor};

use super::{Result, Error};
use super::int::Integer;
use super::extension::{Extension, FeatureNameTable, UnknownExtension, DebugExtensions};
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

#[allow(dead_code)]
const INCOMPATIBLE_DIRTY: u64 = 0b1;
#[allow(dead_code)]
const INCOMPATIBLE_CORRUPT: u64 = 0b10;
#[allow(dead_code)]
const COMPATIBLE_LAZY_REFCOUNTS: u64 = 0b1;
#[allow(dead_code)]
const AUTOCLEAR_BITMAPS: u64 = 0b1;

static INCOMPATIBLE_NAMES: &'static [&'static str] = &["dirty", "corrupt"];
static COMPATIBLE_NAMES: &'static [&'static str] = &["lazy refcounts"];
static AUTOCLEAR_NAMES: &'static [&'static str] = &["bitmaps"];

const HEADER_LENGTH_V3: usize = 104;

pub struct HeaderV3 {
    pub incompatible: Feature,
    pub compatible: Feature,
    pub autoclear: Feature,

    pub refcount_order: u32,
    pub header_length: u32,

    pub feature_name_table: Rc<RefCell<FeatureNameTable>>,
    pub extensions: Vec<Rc<RefCell<Extension>>>,

    // TODO: Once we support backing files, read/write this.
    pub backing_file_name: String,
}
impl HeaderV3 {
    pub fn feature_name_table(&self) -> Ref<FeatureNameTable> {
        self.feature_name_table.borrow()
    }

    // Get an extension by extension code. If we can't find one, use UnknownExtension.
    pub fn extension(&mut self, code: u32) -> Rc<RefCell<Extension>> {
        for r in &self.extensions {
            let e = r.borrow();
            if e.extension_code() == code {
                return r.clone();
            }
        }

        let u = Rc::new(RefCell::new(UnknownExtension::new(code)));
        self.extensions.push(u.clone());
        u
    }
}
impl Debug for HeaderV3 {
    fn fmt(&self, fmt: &mut Formatter) -> result::Result<(), fmt::Error> {
        fmt.debug_struct("HeaderV3")
            .field("incompatible",
                   &self.incompatible.debug(&self.feature_name_table()))
            .field("compatible",
                   &self.compatible.debug(&self.feature_name_table()))
            .field("autoclear",
                   &self.autoclear.debug(&self.feature_name_table()))
            .field("refcount_order", &self.refcount_order)
            .field("header_length", &self.header_length)
            .field("feature_name_table", &self.feature_name_table.borrow())
            .field("backing_file_name", &self.backing_file_name)
            .field("extensions", &DebugExtensions(&self.extensions))
            .finish()
    }
}
impl Default for HeaderV3 {
    fn default() -> Self {
        let feature_name_table = Rc::new(RefCell::new(FeatureNameTable::default()));
        HeaderV3 {
            incompatible: Feature::new(FeatureKind::Incompatible, INCOMPATIBLE_NAMES),
            compatible: Feature::new(FeatureKind::Compatible, COMPATIBLE_NAMES),
            autoclear: Feature::new(FeatureKind::Autoclear, AUTOCLEAR_NAMES),
            refcount_order: 0,
            header_length: 0,
            backing_file_name: String::new(),
            feature_name_table: feature_name_table.clone(),
            extensions: vec![feature_name_table.clone()],
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
    fn read_common<I: Read>(&mut self, io: &mut ByteIo<I, BigEndian>) -> Result<()> {
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

        // Validation.
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
    fn read_v3<I: ReadAt>(&mut self, io: &mut ByteIo<Cursor<I>, BigEndian>) -> Result<()> {
        self.v3.incompatible.set(try!(io.read_u64()));
        self.v3.compatible.set(try!(io.read_u64()));
        self.v3.autoclear.set(try!(io.read_u64()));
        self.v3.refcount_order = try!(io.read_u32());
        self.v3.header_length = try!(io.read_u32());

        let actual_length = io.position();

        // Read extensions.
        let mut seen = HashSet::<u32>::new();
        loop {
            let ext_code = try!(io.read_u32());
            if ext_code == 0 {
                break;
            }

            // No duplicates allowed
            if seen.contains(&ext_code) {
                return Err(Error::FileFormat(format!("duplicate header extension {:#x}",
                                                     ext_code)));
            }
            seen.insert(ext_code);

            let len = try!(io.read_u32()) as u64;
            let ext;
            {
                let take = io.take(len);
                let mut sub = ByteIo::<_, BigEndian>::new(take);
                ext = self.v3.extension(ext_code);
                try!(ext.borrow_mut().read(&mut sub));

                // Verify all is read.
                let remain = sub.bytes().count();
                if remain > 0 {
                    return Err(Error::FileFormat(format!("{} bytes left after reading \
                                                          extension {:#x}",
                                                         remain,
                                                         ext_code)));
                }
            }

            // Read padding.
            let mut pad = vec![0; len.padding_to_multiple(8) as usize];
            try!(io.read_exact(&mut pad));
        }

        // Validation.
        if self.v3.incompatible.enabled(INCOMPATIBLE_CORRUPT) {
            return Err(Error::UnsupportedFeature("corrupt bit".to_owned()));
        }
        try!(self.v3.incompatible.ensure_known(&self.v3.feature_name_table()));
        if self.v3.refcount_order > 6 {
            return Err(Error::FileFormat(format!("bad refcount_order {}", self.v3.refcount_order)));
        }
        if self.v3.header_length as u64 != actual_length {
            return Err(Error::FileFormat(format!("header is {} bytes, file claims {}",
                                                 io.position(),
                                                 self.v3.header_length)));
        }
        if actual_length != HEADER_LENGTH_V3 as u64 {
            return Err(Error::Internal(format!("header must be {} bytes, but we read {}",
                                               HEADER_LENGTH_V3,
                                               io.position())));
        }

        Ok(())
    }

    pub fn read<I: ReadAt>(&mut self, io: &mut ByteIo<I, BigEndian>) -> Result<()> {
        // The headers are best read sequentially, rather than positioned.
        // So get a sequential cursor to read from.
        let curs = Cursor::new(io.deref_mut());
        let mut io: ByteIo<_, BigEndian> = ByteIo::new(curs);

        // Read the header.
        try!(self.read_common(&mut io));
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
