use std::path::Path;

fn main() {
    let libcgss_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("libcgss");
    let dst = cmake::Config::new(libcgss_path)
        .pic(true)
        .uses_cxx11()
        .build();

    let mut build = cxx_build::bridge("src/lib.rs");
    build.file("src/acb.cc").include(&dst.join("include"));

    #[cfg(feature = "debug")]
    build.define("DEBUG", None);

    #[cfg(target_env = "msvc")]
    build.flag("/Wall").flag("/WX").flag("/std:c++14");

    #[cfg(not(target_env = "msvc"))]
    build
        .flag("-Wall")
        .flag("-Wextra")
        .flag("-Werror")
        .flag("-std=c++14");

    build.compile("acb");

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=dylib=cgss");
    println!("cargo:return-if-changed=src/acb.cc");
    println!("cargo:return-if-changed=src/acb.h");
}
