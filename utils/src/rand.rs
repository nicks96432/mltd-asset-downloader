#![cfg(feature = "rand")]

use rand::distributions::uniform::{SampleRange, SampleUniform};
use rand::{thread_rng, Rng, RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus as MyRng;
use std::io::Cursor;

pub fn rand_ascii_string(len: usize) -> Cursor<Vec<u8>> {
    let mut rng = MyRng::from_rng(thread_rng()).unwrap();
    let mut buf = vec![0u8; len];
    for byte in buf.iter_mut().take(len) {
        *byte = u8::try_from(rng.gen_range(0x33..0x7f)).unwrap(); // printable ascii
    }
    buf.push(0u8);

    Cursor::new(buf)
}

pub fn rand_bytes(len: usize) -> Cursor<Vec<u8>> {
    let mut rng = MyRng::from_rng(thread_rng()).unwrap();
    let mut buf = vec![0u8; len];
    rng.fill_bytes(&mut buf);

    Cursor::new(buf)
}

pub fn rand_range<T, R>(range: R) -> T
where
    T: SampleUniform,
    R: SampleRange<T>,
{
    let mut rng = MyRng::from_rng(thread_rng()).unwrap();

    rng.gen_range(range)
}
