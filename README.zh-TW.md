# MLTD 遊戲資源下載器

[![GitHub Build Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/build.yaml)][build status]
![GitHub Repo stars](https://img.shields.io/github/stars/nicks96432/mltd-asset-downloader)
![GitHub top language](https://img.shields.io/github/languages/top/nicks96432/mltd-asset-downloader)
[![License](https://img.shields.io/github/license/nicks96432/mltd-asset-downloader)](LICENSE)

[English](README.md) | 繁體中文

用Rust寫的偶像大師 百萬人演唱會！ 劇場時光 (MLTD) 遊戲資源下載器

## 用法

> [!NOTE]
> 若要轉換資源格式，ffmpeg必須要在`$PATH`中。

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

## 下載

* [Github最新版本](https://github.com/nicks96432/mltd-asset-downloader/releases/latest)
* [main自動編譯版](https://nightly.link/nicks96432/mltd-asset-downloader/workflows/build.yaml/main)

## 編譯

你需要這些工具:

* git
* rust 編譯工具 ([安裝教學](https://www.rust-lang.org/tools/install))
* cmake >= 3.6 (vgmstream要用到)
* clang (bindgen要用到) ([安裝教學](https://rust-lang.github.io/rust-bindgen/requirements.html))

```shell
cargo build --release
```

執行檔會出現在`target/release`資料夾裡。

## 免責聲明

本軟體、工具、以及本軟體的作者與本軟體的repo與萬代南夢宮娛樂、Unity Technologies、以及他們的子公司
沒有任何關係，也沒有任何贊助或授權關係。

## 授權條款

本軟體遵守[MIT](LICENSE)授權條款。

本程式為個人專案，與萬代南夢宮娛樂無關，亦未受其贊助或認可。本程式的任何輸出內容之著作權均屬其作者所有。
使用或散布本程式的輸出內容，須遵循使用者所在地的相關法律規範。

使用本程式即表示您同意自行承擔因使用本程式而可能產生的任何法律責任。

[build status]: https://github.com/nicks96432/mltd-asset-downloader/actions/workflows/build.yaml
