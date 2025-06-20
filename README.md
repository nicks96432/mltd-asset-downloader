# MLTD-asset-downloader

[![GitHub Build Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/build.yaml)][build status]
![GitHub Repo stars](https://img.shields.io/github/stars/nicks96432/mltd-asset-downloader)
![GitHub top language](https://img.shields.io/github/languages/top/nicks96432/mltd-asset-downloader)
[![License](https://img.shields.io/github/license/nicks96432/mltd-asset-downloader)](LICENSE)

English | [繁體中文](README.zh-TW.md)

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

## Get Started

1. Download executable
   * [Github latest release](https://github.com/nicks96432/mltd-asset-downloader/releases/latest)
   * [main branch CI build](https://nightly.link/nicks96432/mltd-asset-downloader/workflows/build.yaml/main)
2. Install FFmpeg shared library >= 7.1 version
   * [FFmpeg official link](https://www.ffmpeg.org/download.html)
   * Make sure FFmpeg shared library is in your `PATH`
   * On Windows, you can use `winget` to install FFmpeg so that it will be added to your `PATH` automatically.
     For example, `winget install BtbN.FFmpeg.LGPL.Shared.7.1`. See `winget search ffmpeg` for more options.
3. Download [AssetRipper](https://github.com/AssetRipper/AssetRipper) to extract assets
   * decompress the zip file and rename the folder to `./AssetRipper`.
   * you can also specify the path via the `--asset-ripper-path` option.

## Build From Source

The following tools are required:

* git
* Latest stable rust toolchain ([installation guide](https://www.rust-lang.org/tools/install))
* cmake >= 3.6 (for vgmstream)
* clang (for bindgen) ([installation guide](https://rust-lang.github.io/rust-bindgen/requirements.html))
  * remember to set `LIBCLANG_PATH` environment variable on Windows
* pkg-config (Linux/MacOS)
* FFmpeg >= 7.1 shared library in your `PATH` (Windows) or `LD_LIBRARY_PATH` (Linux/MacOS)
  * On Windows, `FFMPEG_DIR` should set to the folder where FFmpeg is installed. For example, setting it to `winget` installed
    package would be like:

    ```powershell
    FFMPEG_DIR = 'C:\Users\username\AppData\Local\Microsoft\WinGet\Packages\BtbN.FFmpeg.LGPL.Shared.7.1_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-n7.1-62-gb168ed9b14-win64-lgpl-shared-7.1'
    ```

Alternatively, you can set the environment variables in `.cargo/config-windows.toml`.

To compile (Windows):

```shell
cargo build-windows --release
```

MacOS/Linux:

```shell
cargo build --release
```

The executable will be in the `target/release` directory.

## License

Licensed under [MIT](LICENSE).

This software uses code of [FFmpeg](https://ffmpeg.org) licensed under the
[LGPLv2.1](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html) and its source can be downloaded
[here](https://github.com/FFmpeg/FFmpeg).

This program is an independent project and is not affiliated with, sponsored by, or endorsed by Bandai Namco Entertainment.
The copyright of any output generated by this program belongs to its author.
Use or distribution of the program's output must comply with the laws and regulations of the user's jurisdiction.

BY USING THIS PROGRAM, YOU AGREE TO ASSUME ANY LEGAL RESPONSIBILITY ARISING FROM ITS USE.

[build status]: https://github.com/nicks96432/mltd-asset-downloader/actions/workflows/build.yaml
