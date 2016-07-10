use std::cmp::min;
use std::io;
use std::mem::size_of;

use byteorder::BigEndian;
use positioned_io::{ByteIo, ReadAt, ReadIntAt, Size};

use super::{Error, Qcow2, Result};


const L1_COW: u64 = 1 << 63;
const L1_RESERVED: u64 = (0x7F << 56) | 0xFF;
const L1_POS: u64 = !(L1_COW | L1_RESERVED);
#[derive(Debug)]
pub enum L1Entry {
    Empty,
    Standard {
        pos: u64,
        cow: bool,
    },
}

const L2_COW: u64 = 1 << 63;
const L2_COMPRESSED: u64 = 1 << 62;
const L2_ZERO: u64 = 1;
const L2_RESERVED: u64 = (0x3F << 56) | 0xFE;
const L2_POS: u64 = !(L2_COW | L2_COMPRESSED | L2_ZERO | L2_RESERVED);
const L2_COMPRESSED_MASK: u64 = !(L2_COW | L2_COMPRESSED);
#[derive(Debug)]
pub enum L2Entry {
    Empty,
    Standard {
        pos: u64,
        cow: bool,
        zero: bool,
    },
    Compressed {
        pos: u64,
        cow: bool,
        size: u64,
    },
}

impl<I> Qcow2<I>
    where I: ReadAt
{
    pub fn reader<'a>(&'a self) -> Result<Reader<I>> {
        let offset = self.header.c.l1_table_offset;
        let reader = try!(Reader::new(self, offset));
        Ok(reader)
    }

    fn l1_entry_read<T: ReadIntAt>(&self, l1: &T, l1_l2_idx: u64) -> Result<L1Entry> {
        let offset = l1_l2_idx * size_of::<u64>() as u64;
        let entry = try!(l1.read_u64_at(offset));
        if entry & L1_RESERVED != 0 {
            return Err(Error::FileFormat("reserved bit used in L1 entry".to_owned()));
        }

        let pos = entry & L1_POS;
        if pos == 0 {
            return Ok(L1Entry::Empty);
        }
        Ok(L1Entry::Standard {
            pos: pos,
            cow: (entry & L1_COW != 0),
        })
    }
    fn l2_entry_read_raw(&self, l2_pos: u64, l2_block_idx: u64) -> Result<u64> {
        // TODO: Cache things.
        let offset = l2_pos + l2_block_idx * size_of::<u64>() as u64;
        self.io.read_u64_at(offset).map_err(From::from)
    }
    fn l2_entry_parse(&self, entry: u64) -> Result<L2Entry> {
        let cow = entry & L2_COW != 0;
        Ok(if entry & L2_COMPRESSED != 0 {
            let x = 70 - self.header.c.cluster_bits;
            let entry = entry & L2_COMPRESSED_MASK;
            let pos = entry & ((1 << x) - 1);
            let size = (entry >> x) * 512;
            L2Entry::Compressed {
                pos: pos,
                cow: cow,
                size: size,
            }
        } else {
            if entry & L2_RESERVED != 0 {
                return Err(Error::FileFormat("reserved bit used in L2 entry".to_owned()));
            }
            let pos = entry & L2_POS;
            if pos != 0 {
                L2Entry::Standard {
                    pos: pos,
                    cow: cow,
                    zero: (entry & L2_ZERO != 0),
                }
            } else {
                L2Entry::Empty
            }
        })
    }
    fn l2_entry_read<T: ReadIntAt>(&self, l1: &T, guest_offset: u64) -> Result<L2Entry> {
        let (l1_l2_idx, l2_block_idx, _) = self.header.guest_offset_info(guest_offset);
        let l1_entry = try!(self.l1_entry_read(l1, l1_l2_idx));
        Ok(match l1_entry {
            L1Entry::Empty => L2Entry::Empty,
            L1Entry::Standard { pos, .. } => {
                let raw = try!(self.l2_entry_read_raw(pos, l2_block_idx));
                try!(self.l2_entry_parse(raw))
            }
        })
    }
    fn zero_fill(buf: &mut [u8]) {
        for i in buf {
            *i = 0;
        }
    }
    fn guest_block_read(&self, entry: L2Entry, offset: u64, buf: &mut [u8]) -> Result<()> {
        match entry {
            L2Entry::Empty => Self::zero_fill(buf),
            L2Entry::Standard { pos, zero, .. } => {
                if zero {
                    Self::zero_fill(buf)
                } else {
                    try!(self.io.read_exact_at(pos + offset, buf))
                }
            }
            L2Entry::Compressed { .. } => {
                return Err(Error::UnsupportedFeature("compressed blocks".to_owned()))
            }
        }
        Ok(())
    }
    fn guest_read<T: ReadIntAt>(&self, l1: &T, pos: u64, mut buf: &mut [u8]) -> io::Result<usize> {
        // Check for reads past EOF.
        if pos >= self.header.guest_size() {
            return Ok(0);
        }
        let ret = min(buf.len() as u64, self.header.guest_size() - pos) as usize;
        let mut buf = &mut buf[..ret];

        let mut offset = pos % self.cluster_size();
        let mut guest_block_pos = pos - offset;
        while buf.len() > 0 {
            let entry = try!(self.l2_entry_read(l1, guest_block_pos));
            let size = min(buf.len() as u64, self.cluster_size() - offset) as usize;
            try!(self.guest_block_read(entry, offset, &mut buf[..size]));

            let tmp = buf;
            buf = &mut tmp[size..];
            guest_block_pos += self.cluster_size();
            offset = 0;
        }
        Ok(ret)
    }

    fn l1_read(&self, l1_offset: u64) -> Result<Vec<u8>> {
        let mut buf = vec![0; self.header.l1_entries() as usize * size_of::<u64>()];
        try!(self.io.read_exact_at(l1_offset, &mut buf));
        Ok(buf)
    }
}


pub struct Reader<'a, I: 'a + ReadAt> {
    pub q: &'a Qcow2<I>,
    pub l1: ByteIo<Vec<u8>, BigEndian>,
}

impl<'a, I: 'a + ReadAt> Reader<'a, I> {
    pub fn new(q: &'a Qcow2<I>, l1_offset: u64) -> Result<Self> {
        let buf = try!(q.l1_read(l1_offset));
        let l1 = ByteIo::<_, BigEndian>::new(buf);
        Ok(Reader { q: q, l1: l1 })
    }
}

impl<'a, I> ReadAt for Reader<'a, I>
    where I: 'a + ReadAt
{
    fn read_at(&self, pos: u64, mut buf: &mut [u8]) -> io::Result<usize> {
        self.q.guest_read(&self.l1, pos, buf)
    }
}

impl<'a, I> Size for Reader<'a, I>
    where I: 'a + ReadAt
{
    fn size(&self) -> io::Result<Option<u64>> {
        Ok(Some(self.q.guest_size()))
    }
}
