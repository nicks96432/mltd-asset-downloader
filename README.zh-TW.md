# MLTD遊戲資源下載器

English | [繁體中文](README.zh-TW.md) | [한국어](README.ko-KR.md)

使用Javascript製作的偶像大師 百萬人演唱會！ 劇場時光 (MLTD) 遊戲資源下載器

## 用法

```console
$ ./mltd-asset-downloadeer --help
Usage: mltd-asset-downloader [選項]

偶像大師 百萬人演唱會！ 劇場時光 (MLTD) 遊戲資源下載器

Options:
  -V, --version             印出版本號
  --latest                  跳過所有選項並直接下載最新版遊戲資源
  --dry-run                 不要把檔案存到硬碟裡。這個功能可能在測網速的時候有用 ¯\_(ツ)_/¯
  --checksum                不下載任何檔案，只檢查已下載的檔案是否正確
  -b, --batch-size <size>   一次要下載幾個檔案，預設為CPU核心數 (default: 8)
  -o, --output-path <path>  存檔路徑 (default: "./assets")
  -L, --locale <locale>     要下載的資源的語言，目前支援中文及韓文 (choices: "zh", "ko")
  -h, --help                顯示這個說明
```

## 編譯

```shell
npm install
npm run build
```

或

```shell
yarn
yarn build
```

## 授權條款

本軟體遵守[MIT](LICENSE)授權條款。

所有從本軟體下載的資料版權由萬代南夢宮娛樂所有
