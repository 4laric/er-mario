// Compiles libsm64 (SM64's decompiled Mario code) straight into the mod DLL.
use std::path::Path;

fn main() {
    let root = Path::new("../libsm64/src");
    let dirs = ["", "decomp", "decomp/engine", "decomp/include/PR", "decomp/game", "decomp/pc",
                "decomp/pc/audio", "decomp/mario", "decomp/tools", "decomp/audio"];
    let mut build = cc::Build::new();
    for d in dirs {
        let Ok(entries) = std::fs::read_dir(root.join(d)) else { continue };
        for e in entries {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "c") {
                build.file(&p);
            }
        }
    }
    build
        .include("shim")
        .include(root)
        .include(root.join("decomp/include"))
        .define("GBI_FLOATS", None)
        .define("VERSION_US", None)
        .define("NO_SEGMENTED_MEMORY", None)
        .define("SM64_LIB_EXPORT", None)
        .warnings(false)
        .compile("sm64");
    println!("cargo:rerun-if-changed=../libsm64/src");
}
