use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::io::Read;
use std::mem::size_of;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::result;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use byteorder::BigEndian;
use positioned_io::{ByteIo, ReadAt, ReadInt, Cursor};

use super::{Result, Error};
use super::int::{is_multiple_of, padding_to_multiple, div_ceil, div_rem};
use super::extension::{self, Extension, FeatureNameTable, UnknownExtension};
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

    pub feature_name_table: FeatureNameTable,
    pub unknown_extensions: Vec<UnknownExtension>,

    pub backing_file_name: PathBuf,
}
impl HeaderV3 {
    // Get an extension by extension code. If we can't find one, use UnknownExtension.
    pub fn extension(&mut self, code: u32) -> &mut Extension {
        match code {
            extension::EXT_CODE_FEATURE_NAME_TABLE => &mut self.feature_name_table,
            _ => {
                let u = UnknownExtension::new(code);
                self.unknown_extensions.push(u);
                self.unknown_extensions.last_mut().unwrap()
            }
        }
    }
}
impl Debug for HeaderV3 {
    fn fmt(&self, fmt: &mut Formatter) -> result::Result<(), fmt::Error> {
        fmt.debug_struct("HeaderV3")
            .field("incompatible",
                   &self.incompatible.to_string(&self.feature_name_table))
            .field("compatible",
                   &self.compatible.to_string(&self.feature_name_table))
            .field("autoclear",
                   &self.autoclear.to_string(&self.feature_name_table))
            .field("refcount_order", &self.refcount_order)
            .field("header_length", &self.header_length)
            .field("feature_name_table", &self.feature_name_table)
            .field("backing_file_name", &self.backing_file_name)
            .field("unknown extensions", &self.unknown_extensions)
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
            backing_file_name: PathBuf::new(),
            feature_name_table: FeatureNameTable::default(),
            unknown_extensions: Vec::new(),
        }
    }
}


#[derive(Default, Debug)]
pub struct Header {
    pub c: HeaderCommon,
    pub v3: HeaderV3,
}

impl Header {
    fn validate_common(&self) -> Result<()> {
        if self.c.magic != MAGIC {
            return Err(Error::FileType);
        }
        if self.c.version != SUPPORTED_VERSION {
            return Err(Error::Version(self.c.version));
        }
        if self.c.backing_file_offset != 0 {
            return Err(Error::UnsupportedFeature("backing file".to_owned()));
            // if self.c.backing_file_offset > self.cluster_size() {
            //     return Err(Error::FileFormat("backing file name not in first cluster"
            //          .to_owned()));
            // }
            // if self.c.backing_file_size > 1023 {
            //     return Err(Error::FileFormat("backing file name size too big".to_owned()));
            // }
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
        if !is_multiple_of(self.c.l1_table_offset, self.cluster_size()) {
            return Err(Error::FileFormat("bad L1 offset".to_owned()));
        }
        if !is_multiple_of(self.c.refcount_table_offset, self.cluster_size()) {
            return Err(Error::FileFormat("bad refcount offset".to_owned()));
        }
        if !is_multiple_of(self.c.snapshots_offset, self.cluster_size()) {
            return Err(Error::FileFormat("bad snapshots offset".to_owned()));
        }
        Ok(())
    }

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
        try!(self.validate_common());
        Ok(())
    }

    fn read_extensions<I: ReadAt>(&mut self, io: &mut ByteIo<Cursor<I>, BigEndian>) -> Result<()> {
        let mut seen = HashSet::<u32>::new();
        loop {
            let ext_code = try!(io.read_u32());

            // No duplicates allowed.
            if seen.contains(&ext_code) {
                return Err(Error::FileFormat(format!("duplicate header extension {:#x}",
                                                     ext_code)));
            }
            seen.insert(ext_code);

            let len = try!(io.read_u32()) as u64;
            if ext_code == extension::EXT_CODE_NONE {
                break;
            }

            if len + io.position() > self.cluster_size() {
                // Don't try to read too much dynamic data!
                return Err(Error::FileFormat("complete header too big for first cluster"
                    .to_owned()));
            }
            {
                let take = io.take(len);
                let mut sub = ByteIo::<_, BigEndian>::new(take);
                let mut ext = self.v3.extension(ext_code);
                try!(ext.read(&mut sub));

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
            let mut pad = vec![0; padding_to_multiple(len, 8)];
            try!(io.read_exact(&mut pad));
        }
        Ok(())
    }

    // Read a filesystem path.
    fn read_path<I: Read>(&mut self, io: &mut ByteIo<I, BigEndian>, len: usize) -> Result<PathBuf> {
        let mut buf = vec![0; len];
        try!(io.read_exact(&mut buf));

        if cfg!(unix) {
            // Paths on unix are arbitrary byte sequences.
            Ok(From::from(OsStr::from_bytes(&buf)))
        } else {
            // On other platforms, who knows what to do with non-UTF8 data in there?
            let s: String = String::from_utf8_lossy(&buf).into_owned();
            Ok(From::from(s))
        }
    }

    // Read the version 3 header.
    fn read_v3<I: ReadAt>(&mut self, io: &mut ByteIo<Cursor<I>, BigEndian>) -> Result<()> {
        self.v3.incompatible.set(try!(io.read_u64()));
        self.v3.compatible.set(try!(io.read_u64()));
        self.v3.autoclear.set(try!(io.read_u64()));
        self.v3.refcount_order = try!(io.read_u32());
        self.v3.header_length = try!(io.read_u32());
        if self.v3.header_length as u64 > io.position() {
            // There are addition fields.
            // XXX compression header field ought to be extracted
            io.set_position(self.v3.header_length as u64);
        }
        let actual_length = io.position();
        try!(self.read_extensions(io));
        if self.c.backing_file_offset != 0 {
            println!("{}, {}", self.c.backing_file_offset, io.position());
            if self.c.backing_file_offset != io.position() {
                return Err(Error::FileFormat("backing file offset not consistent with extensions"
                    .to_owned()));
            }
            // Need an extra copy to defeat borrow checker.
            // See https://github.com/rust-lang/rust/issues/29975
            let backing_file_size = self.c.backing_file_size;
            self.v3.backing_file_name = try!(self.read_path(io, backing_file_size as usize));
        }

        // Validation.
        if self.v3.incompatible.enabled(INCOMPATIBLE_CORRUPT) {
            return Err(Error::UnsupportedFeature("corrupt bit".to_owned()));
        }
        try!(self.v3.incompatible.ensure_known(&self.v3.feature_name_table));
        if self.v3.refcount_order > 6 {
            return Err(Error::FileFormat(format!("bad refcount_order {}", self.v3.refcount_order)));
        }
        if self.v3.header_length as u64 != actual_length {
            return Err(Error::FileFormat(format!("header is {} bytes, file claims {}",
                                                 actual_length,
                                                 self.v3.header_length)));
        }
        if io.position() > self.cluster_size() {
            return Err(Error::FileFormat("complete header too big for first cluster".to_owned()));
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

    // How big is the guest?
    pub fn guest_size(&self) -> u64 {
        self.c.size
    }

    // How many virtual blocks can there be?
    pub fn max_virtual_blocks(&self) -> u64 {
        div_ceil(self.c.size, self.cluster_size())
    }

    // How many entries are in an L2?
    pub fn l2_entries(&self) -> u64 {
        self.cluster_size() / size_of::<u64>() as u64
    }

    // How many entries are in an L1?
    pub fn l1_entries(&self) -> u64 {
        div_ceil(self.max_virtual_blocks(), self.l2_entries())
    }

    // Find how an offset fits in the guest block hierarchy.
    // Returns (l1_l2_idx, l2_block_idx, block_offset).
    pub fn guest_offset_info(&self, pos: u64) -> (u64, u64, u64) {
        let (block_idx, block_offset) = div_rem(pos, self.cluster_size());
        let (l1_l2_idx, l2_block_idx) = div_rem(block_idx, self.l2_entries());
        (l1_l2_idx, l2_block_idx, block_offset)
    }
}
