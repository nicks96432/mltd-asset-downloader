#[cxx::bridge]
mod ffi {
    struct Track {
        name: String,
        data: Vec<u8>,
    }

    unsafe extern "C++" {
        include!("acb/src/acb.h");

        pub fn to_wav(buf: &Vec<u8>) -> Result<Vec<Track>>;
    }
}

pub use ffi::Track;

pub fn to_wav(buf: &Vec<u8>) -> Result<Vec<Track>, cxx::Exception> {
    ffi::to_wav(buf)
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use crate::to_wav;

    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    #[test]
    fn test_to_wav() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("test.acb");
        let mut f = File::open(&path).unwrap();

        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();

        let tracks = to_wav(&buf).unwrap();
        let track = tracks.get(0).unwrap();

        let mut expected_file = File::open("tests/test.wav").unwrap();
        let mut expected = Vec::new();
        expected_file.read_to_end(&mut expected).unwrap();

        assert_eq!(expected.len(), track.data.len());
        assert_eq!(expected, track.data);
    }
}
