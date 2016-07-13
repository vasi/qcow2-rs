use std::ascii::AsciiExt;
use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::io::ErrorKind;
use std::result;

use byteorder::ReadBytesExt;
use positioned_io::ReadInt;

use super::{Result, Error};
use super::feature::{FeatureKind, FEATURE_KIND_COUNT};


pub const EXT_CODE_FEATURE_NAME_TABLE: u32 = 0x6803f857;
pub const EXT_CODE_NONE: u32 = 0;

pub trait Extension: Debug {
    fn extension_code(&self) -> u32;
    fn read(&mut self, io: &mut ReadInt) -> Result<()>;
}


pub struct UnknownExtension {
    code: u32,
    data: Vec<u8>,
}
impl UnknownExtension {
    pub fn new(code: u32) -> Self {
        UnknownExtension {
            code: code,
            data: vec![],
        }
    }
}
impl Extension for UnknownExtension {
    fn extension_code(&self) -> u32 {
        self.code
    }
    fn read(&mut self, io: &mut ReadInt) -> Result<()> {
        try!(io.read_to_end(&mut self.data));
        Ok(())
    }
}
impl Debug for UnknownExtension {
    fn fmt(&self, fmt: &mut Formatter) -> result::Result<(), fmt::Error> {
        fmt.debug_struct("UnknownExtension")
            .field("code", &format!("{:#x}", &self.code))
            .field("size", &self.data.len())
            .finish()
    }
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
        EXT_CODE_FEATURE_NAME_TABLE
    }
    fn read(&mut self, io: &mut ReadInt) -> Result<()> {
        loop {
            match io.read_u8() {
                Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(Error::Io(e)),
                Ok(kind) => {
                    if kind >= FEATURE_KIND_COUNT as u8 {
                        return Err(Error::FileFormat("unknown feature type in feature name table"
                            .to_owned()));
                    }

                    let bit = try!(io.read_u8());
                    if bit > 63 {
                        return Err(Error::FileFormat("bit number too high in feature name table"
                            .to_owned()));
                    }

                    let mut buf = [0; 46];
                    try!(io.read_exact(&mut buf));
                    // Remove trailing zero bytes from name.
                    let chars = buf.into_iter()
                        .take_while(|&&c| c != 0)
                        .map(|&c| c)
                        .collect::<Vec<_>>();
                    // Error on non-ASCII characters, are those supported?
                    match chars.iter().find(|c| !c.is_ascii()) {
                        None => {}
                        Some(_) => {
                            return Err(Error::FileFormat("unsafe characters in feature name table"
                                .to_owned()))
                        }
                    }
                    // This can't fail!
                    let name = String::from_utf8(chars).unwrap();
                    self.0.push(FeatureName {
                        kind: kind,
                        bit: bit,
                        name: name,
                    });
                }
            }
        }
        Ok(())
    }
}
