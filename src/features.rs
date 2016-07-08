use byteorder::BigEndian;
use positioned_io::ByteIo;

use super::header::HeaderExtension;
use super::Result;

use std::collections::HashSet;
use std::default::Default;
use std::io::{self, Read};

// A features bitmask.
#[derive(Clone, Copy, Debug)]
enum FeatureKind {
    Incompatible = 0,
    Compatible = 1,
    Autoclear = 2,
    NumKinds = 3,
}

// A known feature IDs.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum KnownFeature {
    Dirty,
    Corrupt,
    LazyRefcounts,
    Bitmaps,
}
// A list of known features.
struct KnownFeatureDesc(KnownFeature, FeatureKind, usize, &'static str);
const KNOWN_FEATURES: &'static [KnownFeatureDesc] = &[
    KnownFeatureDesc(KnownFeature::Dirty, FeatureKind::Incompatible, 0, "dirty bit"),
    KnownFeatureDesc(KnownFeature::Corrupt, FeatureKind::Incompatible, 1, "corrupt bit"),
    KnownFeatureDesc(KnownFeature::LazyRefcounts, FeatureKind::Compatible, 0, "lazy refcounts"),
    KnownFeatureDesc(KnownFeature::Bitmaps, FeatureKind::Autoclear, 0, "valid bitmaps"),
];

// A description of a feature.
#[derive(Debug)]
struct FeatureDesc {
    id: Option<KnownFeature>,
    kind: FeatureKind,
    bit: usize,
    desc: Option<String>,
    enabled: bool,
}
impl<'a> From<&'a KnownFeatureDesc> for FeatureDesc {
    fn from(f: &KnownFeatureDesc) -> Self {
        match f {
            &KnownFeatureDesc(ref id, ref kind, ref bit, ref desc) => FeatureDesc {
                id: Some(*id),
                kind: *kind,
                bit: *bit,
                desc: Some(String::from(*desc)),
                enabled: false,
            }
        }
    }
}

// A feature that we can't handle.
pub struct IncompatibleFeature {
    bit: usize,
    desc: Option<String>,
}

// Actually keep track of features.
#[derive(Debug)]
pub struct Features {
    descs: Vec<FeatureDesc>,
    // Only the known enabled features are stored here.
    enabled: HashSet<KnownFeature>,
}


impl Features {
    // Read in from the header.
    pub fn read<I: Read>(&mut self, io: &mut ByteIo<I, BigEndian>) -> io::Result<()> {
        // Read the feature bits.
        let mut bitmasks = vec![0u64; FeatureKind::NumKinds as usize];
        for b in bitmasks.iter_mut() {
            *b = try!(io.read_u64());
        }

        // Figure out what we have.
        for desc in self.descs.iter_mut() {
            let bits = bitmasks[desc.kind as usize];
            if bits & (1 << desc.bit) != 0 {
                // We have this feature!
                desc.enabled = true;
                if let Some(id) = desc.id {
                    self.enabled.insert(id);
                }
            }
        }

        Ok(())
    }

    // When we have writing figured out.
    // pub fn write() { }
    // pub fn write_names() {}

    pub fn is_enabled() {}
    pub fn unknown_enabled() {}
    pub fn invalid() {}

    pub fn enable() {}
    pub fn disable() { }
    pub fn autoclear() { }
}

impl Default for Features {
    fn default() -> Self {
        // Get the already known descs.
        let descs: Vec<FeatureDesc> = KNOWN_FEATURES.into_iter().map(From::from).collect();
        Features {
            descs: descs,
            enabled: HashSet::new(),
        }
    }
}

impl<'a> HeaderExtension for &'a mut Features {
    fn identifier(&self) -> u32 {
        0x6803f857
    }

    fn read_extension(&mut self, buf: &[u8]) -> Result<()> {
        // TODO
        Ok(())
    }
}
