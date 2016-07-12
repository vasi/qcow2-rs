## qcow2

This crate can parse and read qcow2 virtual disks, as used by qemu and other emulators.

[Qcow2](https://en.wikipedia.org/wiki/Qcow) is a flexible format for disk images, that
only allocates space as needed. It has many other interesting features.

[![Build Status](https://travis-ci.org/vasi/qcow2-rs.svg?branch=master)](https://travis-ci.org/vasi/qcow2-rs)
[![Crates.io](https://img.shields.io/crates/v/qcow2-rs.svg?maxAge=2592000)]()

### Example

```rust
extern crate positioned_io;
extern crate qcow2;

use positioned_io::ReadAt;
use qcow2::Qcow2;

// Open a qcow2 file.
let file = try!(File::open("image.qcow2"));
let qcow = try!(Qcow2::open(file));

// Read some data from the middle.
let reader = try!(qcow.reader());
let mut buf = vec![0, 4096];
try!(reader.read_exact_at(5 * 1024 * 1024, &mut buf));
```

### Documentation

http://vasi.github.io/qcow2-rs/qcow2/

### Usage

This crate works with Cargo and is on
[crates.io](https://crates.io/crates/byteorder). Add it to your `Cargo.toml` like so:

```toml
[dependencies]
qcow2 = "0.1.0"
```
