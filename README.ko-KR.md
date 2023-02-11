# 밀리시타(한섭)-에셋-다운로더

[English](README.md) | [繁體中文](README.zh-TW.md) | 한국어

러스트 기반 THE IDOLM@STER MILLION LIVE! Theater Days (MLTD) 게임 에셋 다운로더.

## 사용법

```console
$ ./mltd-asset-downloader --help
THE IDOLM@STER MILLION LIVE! Theater Days (MLTD) 에셋 다운로더

Usage: mltd-asset-downloader [옵션] <VARIANT>

Arguments:
  <VARIANT>  The os variant to download [possible values: android, ios]

Options:
      --keep-manifest       Keep the manifest file in the output directory
  -o, --output <DIR>        다운로드 경로 [default: "./assets"]
  -P, --parallel <CPUS>     다운로드 파일의 배치 크기, [default: (CPU 코어 수)]
  -v, --verbose...          More output per occurrence
  -q, --quiet...            Less output per occurrence
  -h, --help                이 도움말 표시
  -V, --version             버전 출력
```

## 빌드

```shell
cargo build --release
```

## 라이센스

Licensed under [MIT](LICENSE).

The copyright of anything that downloaded from this program belongs to Bandai Namco Entertainment.
