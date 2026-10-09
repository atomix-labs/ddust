//! The `cfg` the vector reader is built under, named once: `lanes = "built"` where the build has
//! the lookup's instruction, NEON's `tbl` on a little-endian aarch64 target, whose order the lanes
//! take, or SSSE3's `pshufb` on an `x86_64` one; `lanes = "checked"` on an `x86_64` build with SSE2
//! but not SSSE3 whose `runtime-dispatch` feature checks the CPU for it; and `lanes` alone with
//! either.

use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(lanes, values(none(), \"built\", \"checked\"))");
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
}
