use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=cpp/");
    println!("cargo:rerun-if-changed=src/");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    // Use third_party/ by default, allow env override
    let metamod_path = env::var("METAMOD_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("third_party/metamod-source"));

    let hl2sdk_path = env::var("HL2SDK_CS2")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("third_party/hl2sdk-cs2"));

    // Verify SDKs exist
    if !metamod_path.exists() {
        panic!(
            "Metamod SDK not found at {:?}. Run: git submodule update --init",
            metamod_path
        );
    }
    if !hl2sdk_path.exists() {
        panic!(
            "HL2SDK not found at {:?}. Run: git submodule update --init",
            hl2sdk_path
        );
    }

    // Determine platform
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let is_windows = target_os == "windows";

    // Build C++ bridge
    let mut build = cc::Build::new();

    build
        .cpp(true)
        .file("cpp/metamod_bridge.cpp")
        // Metamod includes
        .include(&metamod_path)
        .include(metamod_path.join("core"))
        .include(metamod_path.join("core/sourcehook"))
        // HL2SDK includes
        .include(hl2sdk_path.join("public"))
        .include(hl2sdk_path.join("public/tier0"))
        .include(hl2sdk_path.join("public/tier1"))
        .include(hl2sdk_path.join("public/entity2"))
        .include(hl2sdk_path.join("game/server"));

    if is_windows {
        build
            .flag("/std:c++17")
            .flag("/EHsc")
            .define("WIN32", None)
            .define("_WINDOWS", None)
            .define("COMPILER_MSVC", None)
            .define("COMPILER_MSVC64", None);
    } else {
        build
            .flag("-std=c++17")
            .flag("-fno-exceptions")
            .flag("-fvisibility=hidden") // Hidden by default; SMM_API marks CreateInterface visible
            .flag("-Wno-non-virtual-dtor")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-cast-function-type")
            .define("LINUX", None)
            .define("_LINUX", None)
            .define("POSIX", None)
            .define("COMPILER_GCC", None)
            .define("PLATFORM_64BITS", None);
    }

    // Common defines
    build
        .define("META_NO_HL2SDK", None)
        .define("SOURCE_ENGINE", Some("28")) // CS2
        .define("SE_CS2", Some("28"));

    build.compile("metamod_bridge");

    // Generate C header from Rust exports
    let output_path = manifest_dir.join("cpp/rust_exports.h");

    let config = cbindgen::Config::from_file("cbindgen.toml").unwrap_or_default();

    if let Ok(bindings) = cbindgen::Builder::new()
        .with_crate(&manifest_dir)
        .with_config(config)
        .generate()
    {
        bindings.write_to_file(&output_path);
    }

    // Link directories (for future library needs)
    // Note: With META_NO_HL2SDK, CreateInterface is defined in metamod_bridge.cpp
    // so we don't need interfaces.a
    if is_windows {
        println!(
            "cargo:rustc-link-search={}/lib/public/win64",
            hl2sdk_path.display()
        );
    } else {
        println!(
            "cargo:rustc-link-search={}/lib/linux64",
            hl2sdk_path.display()
        );

        // Force linker to include CreateInterface even though Rust doesn't reference it.
        // Without this, the linker would strip it as dead code.
        println!("cargo:rustc-link-arg=-Wl,-u,CreateInterface");

        // Use a version script to export CreateInterface and rust_* symbols
        let exports_map = manifest_dir.join("exports.map");
        println!("cargo:rerun-if-changed={}", exports_map.display());
        println!(
            "cargo:rustc-link-arg=-Wl,--version-script={}",
            exports_map.display()
        );
    }
}
