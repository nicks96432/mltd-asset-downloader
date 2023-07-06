# MLTD-asset-downloader

![GitHub Check Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/check.yaml?label=Check)
![GitHub Build Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/build.yaml)
![GitHub Repo stars](https://img.shields.io/github/stars/nicks96432/mltd-asset-downloader)
![GitHub top language](https://img.shields.io/github/languages/top/nicks96432/mltd-asset-downloader)
[![License](https://img.shields.io/github/license/nicks96432/mltd-asset-downloader)](LICENSE)

English | [繁體中文](README.zh-TW.md) | [한국어](README.ko-KR.md)

Game asset downloader for THE iDOLM@STER® MILLION LIVE! THEATER DAYS (MLTD), written in Rust.

## Usage

```console
$ ./mltd --help
A CLI made for assets in THE iDOLM@STER® MILLION LIVE! THEATER DAYS (MLTD)

Usage: mltd [OPTIONS] <COMMAND>

Commands:
  download  Download assets from MLTD asset server
  extract   Extract media from MLTD assets
  manifest  Download asset manifest from MLTD asset server
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  More output per occurrence
  -q, --quiet...    Less output per occurrence
  -h, --help        Print help
  -V, --version     Print version
```

## Build

The following is required:

* nightly version of rust toolchain.
* cmake >= 3.2 (for libcgss)
* MSVC v142 or newer version. (Windows)

```shell
cargo build --release
```

The executable will be in the `target/release` directory.

## License

Licensed under [MIT](LICENSE).

The copyright of anything that downloaded from this program belongs to Bandai Namco Entertainment.
