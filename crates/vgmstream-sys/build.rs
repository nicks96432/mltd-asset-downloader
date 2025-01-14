use std::env::var;
use std::path::{Path, PathBuf};

fn main() {
    let crt_static = var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default().contains("crt-static");

    let libvgmstream_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("vgmstream");
    let mut config = cmake::Config::new(libvgmstream_path);

    let bool_to_str = |enabled: bool| if enabled { "ON" } else { "OFF" };

    config
        .no_build_target(true)
        .define("BUILD_AUDACIOUS", bool_to_str(false))
        .define("BUILD_CLI", bool_to_str(false))
        .define("BUILD_FB2K", bool_to_str(false))
        .define("BUILD_STATIC", bool_to_str(cfg!(feature = "static")))
        .define("BUILD_V123", bool_to_str(false))
        .define("BUILD_WINAMP", bool_to_str(false))
        .define("BUILD_XMPLAY", bool_to_str(false))
        .define("USE_ATRAC9", bool_to_str(cfg!(feature = "atrac9")))
        .define("USE_CELT", bool_to_str(cfg!(feature = "celt")))
        .define("USE_FFMPEG", bool_to_str(cfg!(feature = "ffmpeg")))
        .define("USE_G719", bool_to_str(cfg!(feature = "g719")))
        .define("USE_G7221", bool_to_str(cfg!(feature = "g7221")))
        .define("USE_MPEG", bool_to_str(cfg!(feature = "mpeg")))
        .define("USE_SPEEX", bool_to_str(cfg!(feature = "speex")))
        .define("USE_VORBIS", bool_to_str(cfg!(feature = "vorbis")))
        .define(
            "CMAKE_MSVC_RUNTIME_LIBRARY",
            format!("MultiThreaded{}", if crt_static { "" } else { "DLL" }),
        );

    let dst = config.build();

    let mut search_path = dst.join("build").join("src");
    if cfg!(target_os = "windows") {
        search_path = search_path.join("Debug");
    }
    println!("cargo:rustc-link-search=native={}", search_path.display());

    let lib_path = match cfg!(target_os = "windows") {
        true => "libvgmstream",
        false => "vgmstream",
    };
    println!("cargo:rustc-link-lib={}", lib_path);

    let bindings = bindgen::Builder::default()
        .header(concat!(env!("CARGO_MANIFEST_DIR"), "/vgmstream/src/libvgmstream.h"))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(var("OUT_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs")).expect("Couldn't write bindings!");
}
