
extern crate qcow2;
use qcow2::pread::Pread;

fn main() {
    let file = std::fs::File::open("Cargo.toml").unwrap();
    let mut buf = [0; 4];
    file.pread_exact(&mut buf, 10).unwrap();
    println!("{}", std::str::from_utf8(&buf).unwrap());
}
