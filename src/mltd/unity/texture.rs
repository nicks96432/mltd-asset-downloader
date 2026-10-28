//! Unity `Texture2D` pixel data decoding.

use std::fmt::Formatter;

use image::RgbaImage;
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use texture2ddecoder as t2d;

/// Integer that accepts any integer type of the type tree (rabex is strict about signedness/width).
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Int(pub i64);

impl<'de> Deserialize<'de> for Int {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IntVisitor;

        impl Visitor<'_> for IntVisitor {
            type Value = Int;

            fn expecting(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str("an integer")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Int(v))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                i64::try_from(v).map(Int).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(IntVisitor)
    }
}

/// Byte array that can be deserialized from either a byte sequence or a (non UTF-8) string.
#[derive(Debug, Default)]
pub(super) struct Bytes(pub Vec<u8>);

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Bytes;

            fn expecting(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str("bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(Bytes(v.to_vec()))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(Bytes(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Bytes(v.as_bytes().to_vec()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(1 << 26));
                while let Some(b) = seq.next_element::<u8>()? {
                    bytes.push(b);
                }

                Ok(Bytes(bytes))
            }
        }

        deserializer.deserialize_any(BytesVisitor)
    }
}

/// Block layout of a compressed format: (block width, block height, bytes per block).
fn block_layout(format: i32) -> Option<(usize, usize, usize)> {
    const ASTC_BLOCKS: [usize; 6] = [4, 5, 6, 8, 10, 12];

    Some(match format {
        10 | 34 | 41 | 42 | 45 | 46 => (4, 4, 8),
        12 | 25 | 43 | 47 => (4, 4, 16),
        // ASTC_RGB_{4x4..12x12} and ASTC_RGBA_{4x4..12x12}
        48..=59 => {
            let block = ASTC_BLOCKS[usize::try_from(format - 48).ok()? % 6];
            (block, block, 16)
        }
        _ => return None,
    })
}

/// Size of the first mip level in bytes for formats with a non-block layout.
fn mip0_size(format: i32, width: usize, height: usize) -> Option<usize> {
    Some(match format {
        1 => width * height,
        3 => width * height * 3,
        4 | 5 | 14 => width * height * 4,
        // PVRTC_RGB2 / PVRTC_RGBA2
        30 | 31 => (width.max(16) * height.max(8) * 2).div_ceil(8),
        // PVRTC_RGB4 / PVRTC_RGBA4
        32 | 33 => (width.max(8) * height.max(8) * 4).div_ceil(8),
        _ => {
            let (bw, bh, bytes) = block_layout(format)?;
            width.div_ceil(bw) * height.div_ceil(bh) * bytes
        }
    })
}

/// Decodes the first mip level of a texture to RGBA8. Rows are in Unity order (bottom-up).
///
/// # Errors
///
/// Fails when the format is unsupported or the data is malformed.
pub(super) fn decode_texture(
    format: i32,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<RgbaImage, String> {
    let (cols, rows) = (width as usize, height as usize);
    let size = mip0_size(format, cols, rows)
        .ok_or_else(|| format!("unsupported texture format {format}"))?;
    let data = data
        .get(..size)
        .ok_or_else(|| format!("texture data has {} bytes but {size} are required", data.len()))?;

    let raw = match format {
        1 => data.iter().flat_map(|&a| [255, 255, 255, a]).collect(),
        3 => data.as_chunks::<3>().0.iter().flat_map(|[r, g, b]| [*r, *g, *b, 255]).collect(),
        4 => data.to_vec(),
        5 => data.as_chunks::<4>().0.iter().flat_map(|[a, r, g, b]| [*r, *g, *b, *a]).collect(),
        14 => data.as_chunks::<4>().0.iter().flat_map(|[b, g, r, a]| [*r, *g, *b, *a]).collect(),
        _ => {
            let mut pixels = vec![0_u32; cols * rows];
            decode_compressed(format, data, cols, rows, &mut pixels)?;

            // decoders output packed BGRA
            pixels
                .iter()
                .flat_map(|p| {
                    let [blue, green, red, alpha] = p.to_le_bytes();
                    [red, green, blue, alpha]
                })
                .collect()
        }
    };

    RgbaImage::from_raw(width, height, raw).ok_or_else(|| "invalid image dimensions".to_owned())
}

fn decode_compressed(
    format: i32,
    data: &[u8],
    w: usize,
    h: usize,
    out: &mut [u32],
) -> Result<(), String> {
    let result = match format {
        10 => t2d::decode_bc1(data, w, h, out),
        12 => t2d::decode_bc3(data, w, h, out),
        25 => t2d::decode_bc7(data, w, h, out),
        30 | 31 => t2d::decode_pvrtc_2bpp(data, w, h, out),
        32 | 33 => t2d::decode_pvrtc_4bpp(data, w, h, out),
        34 => t2d::decode_etc1(data, w, h, out),
        41 => t2d::decode_eacr(data, w, h, out),
        42 => t2d::decode_eacr_signed(data, w, h, out),
        43 => t2d::decode_eacrg(data, w, h, out),
        45 => t2d::decode_etc2_rgb(data, w, h, out),
        46 => t2d::decode_etc2_rgba1(data, w, h, out),
        47 => t2d::decode_etc2_rgba8(data, w, h, out),
        48..=59 => {
            let (bw, bh, _) = block_layout(format).ok_or("unsupported ASTC block size")?;
            t2d::decode_astc(data, w, h, bw, bh, out)
        }
        _ => return Err(format!("unsupported texture format {format}")),
    };

    result.map_err(str::to_owned)
}
