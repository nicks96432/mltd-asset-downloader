# MLTD-asset-downloader

English | [繁體中文](README.zh-TW.md) | [한국어](README.ko-KR.md)

JavaScript based Game asset downloader for THE IDOLM@STER MILLION LIVE! Theater Days (MLTD).

## Usage

```console
$ ./mltd-asset-downloader --help
Usage: mltd-asset-downloader [options]

asset downloader for THE IDOLM@STER MILLION LIVE! Theater Days (MLTD)

Options:
  -V, --version             output the version number
  --latest                  skip all interactive prompts and download latest assets directly
  --dry-run                 don't download to disk. This may be helpful to test your network speed ¯\_(ツ)_/¯
  --checksum                don't download any file and check all downloaded files
  -b, --batch-size <size>   batch size of downloading file, default CPU cores count (default: 8)
  -o, --output-path <path>  downloaded path (default: "./assets")
  -L, --locale <locale>     the language of the assets to download, supporting Chinese and Korean now (choices: "zh", "ko")
  -h, --help                display this help
```

## Build

```shell
npm install
npm run build
```

or

```shell
yarn
yarn build
```

## License

Licensed under [MIT](LICENSE).

The copyright of anything that downloaded from this program belongs to Bandai Namco Entertainment.
