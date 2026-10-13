#![feature(no_core)]
#![no_core]

extern crate minicore;

extern "C-unwind" {
    fn might_throw(buf: *mut u8);
}

// Compiled as a single CGU (`-Ccodegen-units=1 -Clto=off`) under HWASan so it
// emits a weak COMDAT definition `W __hwasan_personality_thunk` and an
// unresolved reference `U might_throw`. If a later archive on the link line
// has an unresolved `U __hwasan_personality_thunk`, `lld` extracts this object
// to satisfy it, pulling in the backward reference to `might_throw`.
#[no_mangle]
pub extern "C-unwind" fn unused_exceptions_fn() {
    let mut local_buf = [0u8; 16];
    unsafe {
        might_throw(&raw mut local_buf as *mut u8);
    }
}

