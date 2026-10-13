//! Regression test for HWASan + `Lto::ThinLocal` dropping COMDAT definitions of
//! `__hwasan_personality_thunk` in non-prevailing codegen units.
//!
//! When a multi-CGU Rust crate is compiled with `-Zsanitizer=hwaddress` and optimizations
//! (`Lto::ThinLocal`):
//! 1. `HWAddressSanitizer` runs during `PreLinkThinLTO` on each CGU, creating a
//!    `linkonce_odr hidden @__hwasan_personality_thunk` in a COMDAT group.
//! 2. `thin_lto` resolves all copies after the first CGU that finishes `PreLinkThinLTO`
//!    (`cgu.2`) as non-prevailing (`WeakAny`), causing `resolveWeakForLinkerInIndex` and
//!    `thinLTOInternalizeAndPromoteInIndex` to turn `@__hwasan_personality_thunk` into an
//!    external declaration (`U __hwasan_personality_thunk`) and drop its COMDAT group in
//!    `cgu.0` and `cgu.1`.
//! 3. If a downstream link lazily extracts `cgu.1` (without `cgu.2`) or scans `cgu.0`
//!    before `cgu.2` under `--whole-archive`, the unresolved `U __hwasan_personality_thunk`
//!    can pull in an unrelated archive member earlier on the link line that happens to
//!    define `W __hwasan_personality_thunk`, triggering `--warn-backrefs` errors.

//@ needs-llvm-components: aarch64
//@ ignore-backends: gcc

use std::path::Path;

use run_make_support::{Rustc, env_var, is_windows, path, rustc, rustc_minicore, target};

const TARGET: &str = "aarch64-unknown-linux-gnu";

fn configure_lld(cmd: &mut Rustc) {
    cmd.arg("-Zunstable-options").linker_flavor("gnu-lld");
    let sysroot = rustc().print("sysroot").run().stdout_utf8();
    let rust_lld = Path::new(sysroot.trim())
        .join("lib")
        .join("rustlib")
        .join(target())
        .join("bin")
        .join(if is_windows() { "rust-lld.exe" } else { "rust-lld" });
    if rust_lld.exists() {
        cmd.linker("rust-lld");
    } else {
        let llvm_lld =
            Path::new(&env_var("LLVM_BIN_DIR")).join(if is_windows() { "lld.exe" } else { "lld" });
        if llvm_lld.exists() {
            cmd.linker(llvm_lld.to_str().unwrap());
        }
    }
}

fn link_with_warn_backrefs(rlib: &str, lazy: bool, out: &str) {
    let mut cmd = rustc();
    configure_lld(&mut cmd);
    cmd.target(TARGET)
        .panic("abort")
        .arg("-Cdefault-linker-libraries=no")
        .link_arg("--warn-backrefs")
        .link_arg("--fatal-warnings")
        .arg("-lstatic:+verbatim=libbackrefs_early_dep.rlib")
        .arg("-lstatic:+verbatim=libbackrefs_unused_exceptions.rlib");
    if lazy {
        cmd.cfg("lazy").arg(format!("-lstatic:+verbatim={rlib}"));
    } else {
        cmd.arg(format!("-lstatic:+whole-archive,+verbatim={rlib}"));
    }
    cmd.extern_("minicore", path("libminicore.rlib")).input("main.rs").output(out).run();
}

fn main() {
    rustc_minicore().target(TARGET).output("libminicore.rlib").run();

    rustc()
        .target(TARGET)
        .crate_type("rlib")
        .extern_("minicore", path("libminicore.rlib"))
        .input("backrefs_early_dep.rs")
        .output("libbackrefs_early_dep.rlib")
        .run();

    rustc()
        .target(TARGET)
        .crate_type("rlib")
        .opt_level("1")
        .codegen_units(1)
        .lto("off")
        .arg("-Zsanitizer=hwaddress")
        .arg("-Cunsafe-allow-abi-mismatch=sanitizer")
        .extern_("minicore", path("libminicore.rlib"))
        .input("backrefs_unused_exceptions.rs")
        .output("libbackrefs_unused_exceptions.rlib")
        .run();

    // Control check: with a single CGU, `__hwasan_personality_thunk` remains defined in the
    // single object file, so both lazy and whole-archive linking with `--warn-backrefs` succeed.
    rustc()
        .target(TARGET)
        .crate_type("rlib")
        .opt_level("1")
        .codegen_units(1)
        .arg("-Zsanitizer=hwaddress")
        .arg("-Cunsafe-allow-abi-mismatch=sanitizer")
        .extern_("minicore", path("libminicore.rlib"))
        .input("multi_cgu_rust_lib.rs")
        .output("libsingle_cgu_rust_lib.rlib")
        .run();

    link_with_warn_backrefs("libsingle_cgu_rust_lib.rlib", true, "single_cgu_lazy_bin");
    link_with_warn_backrefs("libsingle_cgu_rust_lib.rlib", false, "single_cgu_whole_archive_bin");

    // Multi-CGU build with ThinLocal LTO (`-Ccodegen-units=16 -Copt-level=1`):
    // Every CGU must retain its own COMDAT definition `W __hwasan_personality_thunk` so
    // extracting any subset of CGUs (or scanning earlier archive members in `--whole-archive`
    // mode) does not leave an unresolved `U __hwasan_personality_thunk` that pulls in
    // `libbackrefs_unused_exceptions.rlib` and triggers a `--warn-backrefs` error.
    rustc()
        .target(TARGET)
        .crate_type("rlib")
        .opt_level("1")
        .codegen_units(16)
        .arg("-Zsanitizer=hwaddress")
        .arg("-Cunsafe-allow-abi-mismatch=sanitizer")
        .extern_("minicore", path("libminicore.rlib"))
        .input("multi_cgu_rust_lib.rs")
        .output("libmulti_cgu_rust_lib.rlib")
        .run();

    link_with_warn_backrefs("libmulti_cgu_rust_lib.rlib", true, "lazy_bin");
    link_with_warn_backrefs("libmulti_cgu_rust_lib.rlib", false, "whole_archive_bin");
}

