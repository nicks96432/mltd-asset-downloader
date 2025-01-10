use std::env::var;
use std::path::Path;

fn main() {
    let crt_static =
        var("CARGO_CFG_TARGET_FEATURE").unwrap_or(String::new()).contains("crt-static");

    let libacb_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("libacb");
    let mut config = cmake::Config::new(libacb_path);
    config.define("LIBACB_BUILD_SHARED_LIBS", "OFF").define(
        "CMAKE_MSVC_RUNTIME_LIBRARY",
        format!("MultiThreaded{}", if crt_static { "" } else { "DLL" }),
    );

    let dst = config.build();

    let mut build = cxx_build::bridge("src/lib.rs");
    build.file("src/acb.cc").include(dst.join("include"));

    #[cfg(target_env = "msvc")]
    build.flag("/EHsc").flag("/W4").flag("/WX");

    #[cfg(not(target_env = "msvc"))]
    build.flag("-Wall").flag("-Wextra").flag("-Werror");

    #[cfg(all(target_os = "windows", target_env = "gnu"))]
    build.flag("-static").flag("-static-libstdc++");

    build.compile("acb-cpp");

    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=acb");
    println!("cargo:return-if-changed={}", Path::new("src").join("acb.cc").display());
    println!("cargo:return-if-changed={}", Path::new("src").join("acb.h").display());
}
