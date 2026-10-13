//! Multi-CGU Rust crate that exercises `Lto::ThinLocal` COMDAT/WeakODR handling
//! of `@__hwasan_personality_thunk` across codegen units.
//!
//! Includes a cross-CGU inlineable call (`small_mod::rust_small_entry` calling
//! `big_mod::cross_cgu_inlineable`) so `ComputeCrossModuleImport` adds
//! `__hwasan_personality_thunk` to `ExportLists` (`CrossModuleExportedGUIDs`).

#![allow(internal_features)]
#![feature(no_core)]
#![no_core]

extern crate minicore;

extern "C-unwind" {
    fn external_unwind(buf: *mut u8);
}

#[inline(never)]
pub fn touch_stack(buf: &mut [u8; 16]) {
    unsafe { external_unwind(buf as *mut _ as *mut u8) };
}

macro_rules! repeat16 {
    ($e:expr) => {{
        $e; $e; $e; $e; $e; $e; $e; $e;
        $e; $e; $e; $e; $e; $e; $e; $e;
    }};
}

// Largest module -> numbered `cgu.0` by `partitioning.rs` (which sorts CGUs in
// descending size order), and takes longest in `PreLinkThinLTO` so it finishes
// *after* `small_mod` (`cgu.1`) and the root module (`cgu.2`).
pub mod big_mod {
    use super::touch_stack;

    // Tiny cross-CGU function that passes a stack buffer to `touch_stack` in
    // `cgu.2` (so HWASan instruments its stack and attaches
    // `@__hwasan_personality_thunk`). LLVM's `ComputeCrossModuleImport` imports
    // `cross_cgu_inlineable` from `cgu.0` into `cgu.1`, adding
    // `@__hwasan_personality_thunk` to `ExportLists[cgu.0]`.
    pub fn cross_cgu_inlineable(seed: u8) {
        let mut local_buf = [seed; 16];
        touch_stack(&mut local_buf);
    }

    #[no_mangle]
    pub extern "C-unwind" fn rust_big_work(seed: u8) {
        let mut buf = [seed; 16];
        repeat16!(repeat16!(touch_stack(&mut buf)));
    }
}

// Second-largest module -> numbered `cgu.1` by `partitioning.rs`.
pub mod small_mod {
    use super::touch_stack;

    #[no_mangle]
    #[inline(never)]
    pub extern "C-unwind" fn rust_small_entry(seed: u8) {
        super::big_mod::cross_cgu_inlineable(seed);
        let mut buf = [seed; 16];
        repeat16!(touch_stack(&mut buf));
    }
}

