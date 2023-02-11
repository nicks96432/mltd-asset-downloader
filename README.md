# MLTD-asset-downloader

English | [繁體中文](README.zh-TW.md) | [한국어](README.ko-KR.md)

Game asset downloader for THE IDOLM@STER MILLION LIVE! Theater Days (MLTD), written in Rust.

## Usage

```console
$ ./mltd-asset-downloader --help
asset downloader for THE IDOLM@STER MILLION LIVE! Theater Days (MLTD)

Usage: mltd-asset-downloader [OPTIONS] <VARIANT>

Arguments:
  <VARIANT>  The os variant to download [possible values: android, ios]

Options:
      --keep-manifest    Keep the manifest file in the output directory
  -o, --output <DIR>     The output path [default: ./assets]
  -P, --parallel <CPUS>  The number of threads to use [default: (your CPU core count)]
  -v, --verbose...       More output per occurrence
  -q, --quiet...         Less output per occurrence
  -h, --help             Print help
  -V, --version          Print version
```

## Build

```shell
cargo build --release
```

## License

Licensed under [MIT](LICENSE).

The copyright of anything that downloaded from this program belongs to Bandai Namco Entertainment.
