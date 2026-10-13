#![feature(no_core, thread_local)]
#![no_core]
#![no_main]

extern crate minicore;

#[cfg(lazy)]
extern "C" {
    fn rust_small_entry(seed: u8);
}

#[no_mangle]
pub extern "C" fn _start() {
    #[cfg(lazy)]
    unsafe {
        rust_small_entry(7);
    }
    loop {}
}

// Minimal HWASan and unwind symbol stubs so cross-linking succeeds without a
// target C/HWASan runtime sysroot.
#[thread_local]
#[no_mangle]
pub static mut __hwasan_tls: usize = 0;

#[no_mangle]
pub static __start_hwasan_globals: u8 = 0;

#[no_mangle]
pub static __stop_hwasan_globals: u8 = 0;

macro_rules! stubs {
    ($($name:ident),*) => {
        $(#[no_mangle] pub extern "C" fn $name() {})*
    };
}

stubs!(
    __hwasan_init,
    __hwasan_personality_wrapper,
    __hwasan_tag_memory,
    __hwasan_tag_mismatch4,
    __hwasan_tag_mismatch_v2,
    _Unwind_GetGR,
    _Unwind_GetCFA,
    external_unwind
);

