use std::path::Path;

fn main() {
    let libcgss_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("libcgss");
    let dst = cmake::Config::new(libcgss_path)
        .pic(true)
        .uses_cxx11()
        .define("LIBCGSS_BUILD_SHARED_LIBS", "OFF")
        .build();

    let mut build = cxx_build::bridge("src/lib.rs");
    build.file("src/acb.cc").include(dst.join("include"));

    build.flag("-Wall").flag("-std=c++14");

    #[cfg(all(target_os = "windows", target_env = "gnu"))]
    build.flag("-static").flag("-static-libstdc++");

    #[cfg(not(target_env = "msvc"))]
    build.flag("-Wextra");

    build.compile("acb");

    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=cgss");
    println!("cargo:return-if-changed={}", Path::new("src").join("acb.cc").display());
    println!("cargo:return-if-changed={}", Path::new("src").join("acb.h").display());
}
