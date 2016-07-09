extern crate qcow2;

use std::error::Error;
use std::env::args;
use std::fs::File;
use std::io::{stdout, Write};

use qcow2::Qcow2;

fn run() -> Result<Qcow2<File>, Box<Error>> {
    let mut args = args();
    let filename = try!(args.nth(1).ok_or("Provide at least one argument"));
    let file = try!(File::open(filename));

    let q = try!(qcow2::Qcow2::open(file));
    // println!("{:#?}", q);

    {
        let reader = try!(q.reader());
        let mut buf = vec![0; 4096];
        try!(reader.read_at(0, &mut buf));
        try!(stdout().write_all(&buf));
    }

    Ok(q)
}

fn main() {
    if let Err(e) = run() {
        println!("Error: {}", e);
    }
}
