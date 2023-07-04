# 밀리시타-에셋-다운로더

[![GitHub Check Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/check.yaml?label=Check)](https://github.com/nicks96432/mltd-asset-downloader/actions/workflows/check.yaml)
[![GitHub Build Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/build.yaml)](https://github.com/nicks96432/mltd-asset-downloader/actions/workflows/build.yaml)
![GitHub Repo stars](https://img.shields.io/github/stars/nicks96432/mltd-asset-downloader)
![GitHub top language](https://img.shields.io/github/languages/top/nicks96432/mltd-asset-downloader)
[![License](https://img.shields.io/github/license/nicks96432/mltd-asset-downloader)](LICENSE)

[English](README.md) | [繁體中文](README.zh-TW.md) | 한국어

러스트 기반 THE IDOLM@STER MILLION LIVE! Theater Days (MLTD) 에셋 다운로더.

## 사용법

```console
$ ./mltd --help
THE IDOLM@STER MILLION LIVE! Theater Days (MLTD) 에셋 다운로더

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

## 빌드

```shell
cargo build --release
```

## 라이센스

Licensed under [MIT](LICENSE).

The copyright of anything that downloaded from this program belongs to Bandai Namco Entertainment.
