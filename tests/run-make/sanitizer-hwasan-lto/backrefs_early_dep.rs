#![feature(no_core)]
#![no_core]

extern crate minicore;

#[no_mangle]
pub extern "C-unwind" fn might_throw(_: *mut u8) {}

