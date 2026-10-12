//! The `cfg`s the crate is built under, each named once.
//!
//! `lanes` is the vector reader's: `lanes = "built"` where the build has the lookup's instruction,
//! NEON's `tbl` on a little-endian aarch64 target, whose order the lanes take, or SSSE3's `pshufb`
//! on an `x86_64` one; `lanes = "checked"` on an `x86_64` build with SSE2 but not SSSE3 whose
//! `runtime-dispatch` feature checks the CPU for it; and `lanes` alone with either. `estimate`,
//! under which a quotient is taken from an `f64` estimate, is set on aarch64 with NEON, whose
//! `fdiv` is pipelined, the one target it is measured on.

use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(lanes, values(none(), \"built\", \"checked\"))");
    println!("cargo::rustc-check-cfg=cfg(estimate)");
    let variable = |name: &str| env::var(name).unwrap_or_default();
    let features = variable("CARGO_CFG_TARGET_FEATURE");
    let has = |feature: &str| features.split(',').any(|enabled| enabled == feature);
    let checks = env::var_os("CARGO_FEATURE_RUNTIME_DISPATCH").is_some();
    let arch = variable("CARGO_CFG_TARGET_ARCH");
    let lanes = match (arch.as_str(), variable("CARGO_CFG_TARGET_ENDIAN").as_str()) {
        ("aarch64", "little") if has("neon") => Some("built"),
        ("x86_64", _) if has("ssse3") => Some("built"),
        ("x86_64", _) if has("sse2") && checks => Some("checked"),
        _ => None,
    };
    if let Some(lanes) = lanes {
        println!("cargo::rustc-cfg=lanes");
        println!("cargo::rustc-cfg=lanes=\"{lanes}\"");
    }
    if arch == "aarch64" && has("neon") {
        println!("cargo::rustc-cfg=estimate");
    }
}
