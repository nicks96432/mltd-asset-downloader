use std::error::Error;
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

use clap::builder::PossibleValue;
use clap::ValueEnum;
use image::codecs::avif::{AvifEncoder, ColorSpace};
use image::codecs::bmp::BmpEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::codecs::tiff::TiffEncoder;
use image::codecs::webp::{WebPEncoder, WebPQuality};
use image::{DynamicImage, ImageFormat};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct MyImageFormat(pub ImageFormat);

impl FromStr for MyImageFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            ImageFormat::from_extension(s).ok_or(format!("unsupported image format: {}", s))?,
        ))
    }
}

impl Deref for MyImageFormat {
    type Target = ImageFormat;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ValueEnum for MyImageFormat {
    fn from_str(input: &str, ignore_case: bool) -> Result<Self, <MyImageFormat as FromStr>::Err> {
        match ignore_case {
            true => FromStr::from_str(input.to_ascii_lowercase().as_str()),
            false => FromStr::from_str(input),
        }
    }

    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self(ImageFormat::Avif),
            Self(ImageFormat::Bmp),
            Self(ImageFormat::Jpeg),
            Self(ImageFormat::Png),
            Self(ImageFormat::Tiff),
            Self(ImageFormat::WebP),
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self.0 {
            ImageFormat::Avif => Some(PossibleValue::new("avif")),
            ImageFormat::Bmp => Some(PossibleValue::new("bmp")),
            ImageFormat::Jpeg => Some(PossibleValue::new("jpeg")),
            ImageFormat::Png => Some(PossibleValue::new("png")),
            ImageFormat::Tiff => Some(PossibleValue::new("tiff")),
            ImageFormat::WebP => Some(PossibleValue::new("webp")),
            _ => None,
        }
    }
}

pub fn write_buffer_with_format<P>(
    image: &DynamicImage,
    path: P,
    format: &ImageFormat,
    quality: u8,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let mut f = File::create(&path)?;
    log::info!("writing image to {}", path.as_ref().display());

    match format {
        ImageFormat::Avif => {
            image.write_with_encoder(
                AvifEncoder::new_with_speed_quality(f, 5, quality)
                    .with_colorspace(ColorSpace::Srgb)
                    .with_num_threads(Some(1)),
            )?;
        }
        ImageFormat::Bmp => image.write_with_encoder(BmpEncoder::new(&mut f))?,
        ImageFormat::Jpeg => image.write_with_encoder(JpegEncoder::new_with_quality(f, quality))?,
        ImageFormat::Png => image.write_with_encoder(PngEncoder::new_with_quality(
            f,
            CompressionType::Best,
            FilterType::Adaptive,
        ))?,
        ImageFormat::Tiff => image.write_with_encoder(TiffEncoder::new(f))?,
        ImageFormat::WebP => image.write_with_encoder(WebPEncoder::new_with_quality(
            f,
            match quality {
                100 => WebPQuality::lossless(),
                _ => WebPQuality::lossy(quality),
            },
        ))?,
        _ => return Err(format!("unsupported image format: {:?}", format).into()),
    };

    Ok(())
}
