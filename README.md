# MLTD-asset-downloader

English | [繁體中文](README.zh-TW.md) | [한국어](README.ko-KR.md)

Game asset downloader for THE IDOLM@STER MILLION LIVE! Theater Days (MLTD), written in Rust.

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

```shell
cargo build --release
```

The executable will be in the `target/release` directory.

## License

Licensed under [MIT](LICENSE).

The copyright of anything that downloaded from this program belongs to Bandai Namco Entertainment.
