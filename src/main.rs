extern crate positioned_io;
extern crate qcow2;

use std::error::Error;
use std::env::args;
use std::fs::File;

use positioned_io::ReadAt;
use qcow2::Qcow2;

fn run() -> Result<Qcow2<File>, Box<Error>> {
    let mut args = args();
    let filename = try!(args.nth(1).ok_or("Provide at least one argument"));
    let file = try!(File::open(filename));

    let q = try!(qcow2::Qcow2::open(file));
    println!("{:#?}", q);

    {
        let reader = try!(q.reader());
        let mut buf = vec![0; 32768];
        try!(reader.read_exact_at(8_589_905_920, &mut buf));
        println!("Read ok!");
    }

    Ok(q)
}

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e);
    }
}
