use self::pread;

pub trait ReadInt {
    fn read_u32(&mut self) -> io::Result<u32>;
    fn read_u64(&mut self) -> io::Result<u64>;
}

pub trait PreadInt {
    fn pread_u64(&mut self, pos: u64) -> io::Result<u64>;
}

pub struct Pread(Pread);
