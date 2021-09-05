# 밀리시타(한섭)-에셋-다운로더

English | [繁體中文](README.zh-TW.md) | [한국어](README.ko-KR.md)

자바스크립트 기반 THE IDOLM@STER MILLION LIVE! Theater Days (MLTD) 게임 에셋 다운로더.

## 사용법

```console
$ ./mltd-asset-downloader --help
Usage: mltd-asset-downloader [옵션]

THE IDOLM@STER MILLION LIVE! Theater Days (MLTD) 에셋 다운로더

Options:
  -V, --version             버전 출력
  --latest                  모든 대화형 프롬프트를 건너뛰고 바로 최신 에셋을 다운로드 합니다.
  --dry-run                 디스크에 다운로드 하지 않습니다. 인터넷 속도 테스트에 도움이 될지도 모르겠네요 ¯\_(ツ)_/¯
  --checksum                파일을 다운로드 하지 않고 다운로드한 모든 파일을 확인합니다.
  -b, --batch-size <size>   다운로드 파일의 배치 크기, CPU 코어 수 (default: 8)
  -o, --output-path <path>  다운로드 경로 (default: "./assets")
  -L, --locale <locale>     the language of the assets to download, supporting Chinese and Korean now (choices: "zh", "ko")
  -h, --help                이 도움말 표시
```

## 빌드

```shell
npm install
npm run build
```

또는

```shell
yarn
yarn build
```

## 라이센스

Licensed under [MIT](LICENSE).

The copyright of anything that downloaded from this program belongs to Bandai Namco Entertainment.
