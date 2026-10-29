use std::ffi::OsStr;
use std::path::Path;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Returns the `fmt ` and `data` chunks of a WAV file.
///
/// The other chunks are skipped because ffmpeg writes its own version into the `LIST` chunk,
/// which changes whenever the library is upgraded.
fn wav_audio(path: &Path) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WAVE", "not a WAV file");

    let mut audio = Vec::new();
    let mut rest = &bytes[12..];
    while let Some((header, payload)) = rest.split_first_chunk::<8>() {
        let (id, size) = header.split_at(4);
        let size = u32::from_le_bytes(size.try_into().unwrap()) as usize;
        let (chunk, tail) = payload.split_at(size);
        if id == b"fmt " || id == b"data" {
            audio.extend_from_slice(id);
            audio.extend_from_slice(chunk);
        }

        // chunks are padded to an even size
        rest = tail.get(chunk.len() % 2..).unwrap_or_default();
    }

    audio
}

struct TestCase {
    pub bundle_path: &'static str,
    pub expected_path: &'static str,
    pub got_relpath: &'static str,
    pub predicate: fn(&TempDir, &Self),
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        bundle_path: "tests/test_acb.unity3d",
        expected_path: "tests/test_acb.acb",
        got_relpath: "Assets/imas/resources/adx2/song3/song3_00test.acb",
        predicate: |dir, case| {
            let got = dir.child(Path::new(case.got_relpath).with_file_name("song3_00test_bgm.wav"));
            let expected = Path::new(case.expected_path).with_extension("wav");
            assert!(
                wav_audio(got.path()) == wav_audio(&expected),
                "decoded audio differs from {}",
                expected.display()
            );

            // the temporary key file of vgmstream is removed after decoding
            dir.child(Path::new(case.got_relpath).with_file_name(".hcakey"))
                .assert(predicate::path::missing());
        },
    },
    TestCase {
        bundle_path: "tests/test_text.unity3d",
        expected_path: "tests/test_text.gtx",
        got_relpath: "Assets/imas/resources/textassets/zh/commu/card/card_blst_014mir0034_zh.gtx",
        predicate: |dir, case| {
            dir.child(case.got_relpath).assert(predicate::path::eq_file(
                Path::new(case.expected_path).with_file_name("test_text_decrypted.gtx"),
            ));
        },
    },
    TestCase {
        bundle_path: "tests/test_image.unity3d",
        expected_path: "tests/test_image.png",
        got_relpath: "Assets/imas/resources/card/014mir0011_0.png",
        predicate: |dir, case| {
            let expected_path =
                Path::new(case.expected_path).with_file_name("test_image_processed.png");
            let got_path =
                dir.child(Path::new(case.got_relpath).with_file_name("014mir0011_0_0.png"));
            got_path.assert(predicate::path::eq_file(expected_path));
        },
    },
];

#[test]
fn test_unpack_raw_then_decode_audio() {
    let dir = TempDir::new().unwrap();
    let relpath = Path::new("Assets/imas/resources/adx2/song3");

    cargo_bin_cmd!("mltd")
        .args(["unpack", "tests/test_acb.unity3d", "--raw"])
        .args([OsStr::new("-o"), dir.path().as_os_str()])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .assert()
        .success();

    // the audio is written but not decoded
    let raw = dir.child(relpath.join("song3_00test.acb"));
    raw.assert(predicate::path::eq_file("tests/test_acb.acb"));
    let wav = dir.child(relpath.join("song3_00test_bgm.wav"));
    wav.assert(predicate::path::missing());

    cargo_bin_cmd!("mltd")
        .arg("decode")
        .arg(raw.path())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .assert()
        .success();

    assert!(wav_audio(wav.path()) == wav_audio(Path::new("tests/test_acb.wav")));
    dir.child(relpath.join(".hcakey")).assert(predicate::path::missing());
}

#[test]
fn test_unpack_raw_then_decrypt_text() {
    let dir = TempDir::new().unwrap();
    let relpath = Path::new("Assets/imas/resources/textassets/zh/commu/card");
    let name = "card_blst_014mir0034_zh.gtx";

    cargo_bin_cmd!("mltd")
        .args(["unpack", "tests/test_text.unity3d", "--no-decrypt"])
        .args([OsStr::new("-o"), dir.path().as_os_str()])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .assert()
        .success();

    // the encrypted text keeps `.bytes` so it is not mistaken for a decrypted one
    let raw = dir.child(relpath.join(format!("{name}.bytes")));
    raw.assert(predicate::path::eq_file("tests/test_text.gtx"));
    dir.child(relpath.join(name)).assert(predicate::path::missing());

    cargo_bin_cmd!("mltd")
        .arg("decode")
        .arg(raw.path())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .assert()
        .success();

    dir.child(relpath.join(name)).assert(predicate::path::eq_file("tests/test_text_decrypted.gtx"));

    // a file without `.bytes` is never decrypted in place
    cargo_bin_cmd!("mltd")
        .arg("decode")
        .arg(dir.child(relpath.join(name)).path())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .assert()
        .failure();
}

#[test]
fn test_unpack() {
    for case in TEST_CASES {
        let dir = TempDir::new().unwrap();

        let mut cmd = cargo_bin_cmd!("mltd");
        cmd.arg("unpack")
            .arg(case.bundle_path)
            .args([OsStr::new("-o"), dir.path().as_os_str()])
            .current_dir(env!("CARGO_MANIFEST_DIR"));

        cmd.assert().success();

        (case.predicate)(&dir, case);
    }
}
