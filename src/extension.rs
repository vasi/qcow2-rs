use std::ascii::AsciiExt;
use std::borrow::Cow;
use std::io::{ErrorKind};

use byteorder::ReadBytesExt;
use positioned_io::ReadInt;

use super::{Result, Error};
use super::feature::{FeatureKind, FEATURE_KIND_COUNT};


pub trait Extension {
    fn extension_code(&self) -> u32;
    fn read(&mut self, io: &mut ReadInt) -> Result<()>;
    fn validate(&mut self) -> Result<()> {
        Ok(())
    }
    // TODO: write
}

#[derive(Debug)]
pub struct FeatureName {
    kind: u8,
    bit: u8,
    name: String,
}
#[derive(Debug, Default)]
pub struct FeatureNameTable(Vec<FeatureName>);
impl FeatureNameTable {
    pub fn name(&self, kind: FeatureKind, bit: u8) -> Cow<String> {
        for n in &self.0 {
            if n.kind == kind as u8 && n.bit == bit {
                return Cow::Borrowed(&n.name);
            }
        }
        Cow::Owned(format!("bit {} of {:?}", bit, kind))
    }
}
impl Extension for FeatureNameTable {
    fn extension_code(&self) -> u32 {
        0x6803f857
    }
    fn read(&mut self, io: &mut ReadInt) -> Result<()> {
        loop {
            match io.read_u8() {
                Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(Error::Io(e)),
                Ok(kind) => {
                    let bit = try!(io.read_u8());
                    let mut buf = [0; 46];

                    try!(io.read_exact(&mut buf));
                    // Remove trailing zero bytes from name.
                    let mut chars = buf.into_iter().take_while(|&&c| c != 0);
                    // Error on non-ASCII characters, are those supported?
                    match chars.find(|c| !c.is_ascii()) {
                        None => {},
                        Some(_) => return Err(Error::FileFormat("unsafe characters in feature name table".to_owned()))
                    }
                    let name = chars.map(|&c| c as char).collect::<String>();
                    self.0.push(FeatureName {
                        kind: kind,
                        bit: bit,
                        name: name,
                    });
                },
            }
        }
        Ok(())
    }
    fn validate(&mut self) -> Result<()> {
        for n in &self.0 {
            if n.kind >= FEATURE_KIND_COUNT as u8 {
                return Err(Error::FileFormat("unknown feature type in feature name table".to_owned()));
            }
            if n.bit > 63 {
                return Err(Error::FileFormat("bit number too high in feature name table".to_owned()));
            }
        }
        Ok(())
    }
}
