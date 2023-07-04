# MLTD 遊戲資源下載器

![GitHub Check Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/check.yaml?label=Check)
![GitHub Build Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/build.yaml)
![GitHub Repo stars](https://img.shields.io/github/stars/nicks96432/mltd-asset-downloader)
![GitHub top language](https://img.shields.io/github/languages/top/nicks96432/mltd-asset-downloader)
[![License](https://img.shields.io/github/license/nicks96432/mltd-asset-downloader)](LICENSE)

[English](README.md) | 繁體中文 | [한국어](README.ko-KR.md)

用Rust寫的偶像大師 百萬人演唱會！ 劇場時光 (MLTD) 遊戲資源下載器

## 用法

```console
$ ./mltd --help
偶像大師 百萬人演唱會！ 劇場時光 (MLTD) 遊戲資源下載器

用法: mltd [OPTIONS] <COMMAND>

Commands:
  download  從MLTD資源伺服器下載資源包
  extract   從MLTD的資源包中提取資源
  manifest  從MLTD資源伺服器下載資源包列表
  help      顯示這個說明訊息或是以上指令的說明

選項:
  -v, --verbose...  顯示更多輸出訊息
  -q, --quiet...    顯示更少輸出訊息
  -h, --help        顯示說明
  -V, --version     顯示版本
```

## 編譯

```shell
cargo build --release
```

## 授權條款

本軟體遵守[MIT](LICENSE)授權條款。

所有從本軟體下載的資料版權由萬代南夢宮娛樂所有
