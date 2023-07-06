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
    build.flag("/Wall").flag("/MT").flag("/std:c++14");

    #[cfg(not(target_env = "msvc"))]
    build.flag("-Wall").flag("-Wextra").flag("-std=c++14");

    build.compile("acb");

    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=cgss");
    println!(
        "cargo:return-if-changed={}",
        Path::new("src").join("acb.cc").display()
    );
    println!(
        "cargo:return-if-changed={}",
        Path::new("src").join("acb.h").display()
    );
}
