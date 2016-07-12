extern crate positioned_io;
extern crate qcow2;

use std::fs::File;
use positioned_io::ReadAt;
use qcow2::Qcow2;

#[test]
fn basic_read() {
    let file = File::open("tests/test.qcow2").unwrap();
    let qcow = Qcow2::open(file).unwrap();
    let reader = qcow.reader().unwrap();
    let mut buf = [0; 11];
    reader.read_exact_at(1024 * 1024 * 200, &mut buf).unwrap();
    let s = std::str::from_utf8(&buf).unwrap();
    assert_eq!(s, "Lorem ipsum");
}
