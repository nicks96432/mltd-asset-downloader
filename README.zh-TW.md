# MLTD 遊戲資源下載器

[English](README.md) | 繁體中文 | [한국어](README.ko-KR.md)

用Rust寫的偶像大師 百萬人演唱會！ 劇場時光 (MLTD) 遊戲資源下載器

## 用法

```console
$ ./mltd-asset-downloadeer --help
偶像大師 百萬人演唱會！ 劇場時光 (MLTD) 遊戲資源下載器

用法: mltd-asset-downloader [OPTIONS] <VARIANT>

參數:
  <VARIANT>  The os variant to download [possible values: android, ios]

選項:
      --keep-manifest       保留manifest在輸出資料夾裡
  -o, --output <DIR>        存檔路徑 (default: "./assets")
  -P, --parallel <CPUS>     一次要下載幾個檔案 [default: (你的CPU核心數)]
  -v, --verbose...          顯示更多輸出訊息
  -q, --quiet...            顯示更少輸出訊息
  -h, --help                顯示說明
  -V, --version             印出版本號
```

## 編譯

```shell
cargo build --release
```

## 授權條款

本軟體遵守[MIT](LICENSE)授權條款。

所有從本軟體下載的資料版權由萬代南夢宮娛樂所有
