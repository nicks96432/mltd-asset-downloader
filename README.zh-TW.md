# MLTD 遊戲資源下載器

[![GitHub Build Status](https://img.shields.io/github/actions/workflow/status/nicks96432/mltd-asset-downloader/build.yaml)][build status]
![GitHub Repo stars](https://img.shields.io/github/stars/nicks96432/mltd-asset-downloader)
![GitHub top language](https://img.shields.io/github/languages/top/nicks96432/mltd-asset-downloader)
[![License](https://img.shields.io/github/license/nicks96432/mltd-asset-downloader)](LICENSE)

[English](README.md) | 繁體中文

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

## 下載

1. 下載主程式
   * [Github最新版本](https://github.com/nicks96432/mltd-asset-downloader/releases/latest)
   * [main自動編譯版](https://nightly.link/nicks96432/mltd-asset-downloader/workflows/build.yaml/main)
2. 安裝FFmpeg >= 7.1 共享函式庫版
   * [FFmpeg官網連結](https://www.ffmpeg.org/download.html)
   * 確保FFmpeg共享函式庫在你的 `PATH` 裡面
   * Windows上可以用 `winget` 來安裝FFmpeg，這樣它就會被自動加進`PATH`裡，例如
     `winget install BtbN.FFmpeg.LGPL.Shared.7.1` 。用 `winget search ffmpeg` 可以查看更多選擇

## 編譯

你需要這些工具:

* git
* rust 編譯工具 ([安裝教學](https://www.rust-lang.org/tools/install))
* cmake >= 3.6 (vgmstream要用到)
* clang (bindgen要用到) ([安裝教學](https://rust-lang.github.io/rust-bindgen/requirements.html))
  * Windows上要記得設定 `LIBCLANG_PATH` 環境變數
* pkg-config (Linux/MacOS)
* 在你的 `PATH` (Windows) 或 `LD_LIBRARY_PATH` (Linux/MacOS) 中有FFmpeg >= 7.1 共享函式庫版
  * Windows上要額外設定 `FFMPEG_DIR` 環境變數為FFmpeg安裝資料夾，例如在Powershell中設定 `winget` 安裝的FFmpeg套件：

  ```powershell
  $env:FFMPEG_DIR='C:\Users\username\AppData\Local\Microsoft\WinGet\Packages\BtbN.FFmpeg.LGPL.Shared.7.1_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-n7.1-62-gb168ed9b14-win64-lgpl-shared-7.1'
  ```

環境變數也可以在 `.cargo/config-windows.toml` 中設定。

編譯 (Windows)：

```shell
cargo build-windows --release
```

Linux/MacOS：

```shell
cargo build --release
```

執行檔會出現在 `target/release` 資料夾裡。

## 授權條款

本軟體遵守[MIT](LICENSE)授權條款。

本軟體在[LGPLv2.1](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html)授權條款下使用
[FFmpeg](https://ffmpeg.org)的程式，它的原始碼可以在[這裡](https://github.com/FFmpeg/FFmpeg)
下載。

本程式為個人專案，與萬代南夢宮娛樂無關，亦未受其贊助或認可。本程式的任何輸出內容之著作權均屬其作者所有。
使用或散布本程式的輸出內容，須遵循使用者所在地的相關法律規範。

使用本程式即表示您同意自行承擔因使用本程式而可能產生的任何法律責任。

[build status]: https://github.com/nicks96432/mltd-asset-downloader/actions/workflows/build.yaml
