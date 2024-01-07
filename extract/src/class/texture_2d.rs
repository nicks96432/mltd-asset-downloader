use std::error::Error;
use std::io::Cursor;
use std::str::FromStr;

use byteorder::{ByteOrder, ReadBytesExt};
use image::{DynamicImage, GrayImage, RgbaImage};
use rabex::files::SerializedFile;
use rabex::objects::classes::{GLTextureSettings, StreamingInfo, Texture2D};
use rabex::read_ext::{ReadSeekUrexExt, ReadUrexExt};

use crate::utils::ReadAlignedExt;
use crate::version::Version;

pub fn construct_texture_2d<E>(
    data: &[u8],
    serialized_file: &SerializedFile,
) -> Result<Texture2D, Box<dyn Error>>
where
    E: ByteOrder,
{
    log::debug!("data size: {}", data.len());

    let mut reader = Cursor::new(data);
    let unity_version = Version::from_str(serialized_file.m_UnityVersion.as_ref().unwrap())?;

    Ok(Texture2D {
        m_Name: reader.read_aligned_string::<E>()?,
        m_ForcedFallbackFormat: match Version::from_str("2017.3.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        m_DownscaleFallback: match Version::from_str("2017.3.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_IsAlphaChannelOptional: match Version::from_str("2020.2.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_Width: {
            reader.align4()?;
            reader.read_i32::<E>()?
        },
        m_Height: reader.read_i32::<E>()?,
        m_CompleteImageSize: reader.read_i32::<E>()? as i64,
        m_MipsStripped: match Version::from_str("2020.1.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        m_TextureFormat: reader.read_i32::<E>()?,
        m_MipMap: match Version::from_str("3.4.0f0").unwrap() <= unity_version
            && unity_version <= Version::from_str("5.1.5f1").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_MipCount: match Version::from_str("5.2.0f2").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        m_IsReadable: match unity_version >= Version::from_str("2.6.0f0").unwrap() {
            true => reader.read_bool()?,
            false => false,
        },
        m_IsPreProcessed: match Version::from_str("2019.4.9f1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_IgnoreMasterTextureLimit: match Version::from_str("2019.3.0f6").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.2.0a18").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_IgnoreMipmapLimit: match Version::from_str("2022.2.0f1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_MipmapLimitGroupName: match Version::from_str("2022.2.0f1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_string::<E>()?),
            false => None,
        },
        m_ReadAllowed: match Version::from_str("3.4.0f0").unwrap() <= unity_version
            && unity_version <= Version::from_str("5.4.6f3").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_StreamingMipmaps: match Version::from_str("2018.2.0b1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_bool()?),
            false => None,
        },
        m_StreamingMipmapsPriority: {
            reader.align4()?;
            match Version::from_str("2018.2.0b1").unwrap() <= unity_version
                && unity_version <= Version::from_str("2022.3.2f1").unwrap()
            {
                true => Some(reader.read_i32::<E>()?),
                false => None,
            }
        },
        m_ImageCount: reader.read_i32::<E>()?,
        m_TextureDimension: reader.read_i32::<E>()?,
        m_TextureSettings: GLTextureSettings {
            m_FilterMode: reader.read_i32::<E>()?,
            m_Aniso: reader.read_i32::<E>()?,
            m_MipBias: reader.read_f32::<E>()?,
            m_WrapMode: match Version::from_str("3.4.0f0").unwrap() <= unity_version
                && unity_version <= Version::from_str("5.6.7f1").unwrap()
            {
                true => Some(reader.read_i32::<E>()?),
                false => None,
            },
            m_WrapU: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
                && unity_version <= Version::from_str("2022.3.2f1").unwrap()
            {
                true => Some(reader.read_i32::<E>()?),
                false => None,
            },
            m_WrapV: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
                && unity_version <= Version::from_str("2022.3.2f1").unwrap()
            {
                true => Some(reader.read_i32::<E>()?),
                false => None,
            },
            m_WrapW: match Version::from_str("2017.1.0b1").unwrap() <= unity_version
                && unity_version <= Version::from_str("2022.3.2f1").unwrap()
            {
                true => Some(reader.read_i32::<E>()?),
                false => None,
            },
        },
        m_LightmapFormat: match unity_version >= Version::from_str("3.0.0f0").unwrap() {
            true => reader.read_i32::<E>()?,
            false => i32::default(),
        },
        m_ColorSpace: match Version::from_str("3.5.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_i32::<E>()?),
            false => None,
        },
        m_PlatformBlob: {
            let blob = match Version::from_str("2020.2.0b1").unwrap() <= unity_version
                && unity_version <= Version::from_str("2022.3.2f1").unwrap()
            {
                true => Some(reader.read_bytes::<E>()?),
                false => None,
            };
            reader.align4()?;

            blob
        },
        image_data: match Version::from_str("3.4.0").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(reader.read_bytes::<E>()?),
            false => None,
        },
        m_StreamData: match Version::from_str("5.3.0f1").unwrap() <= unity_version
            && unity_version <= Version::from_str("2022.3.2f1").unwrap()
        {
            true => Some(StreamingInfo {
                offset: match unity_version >= Version::from_str("2020.1.0f1").unwrap() {
                    true => reader.read_u64::<E>()?,
                    false => reader.read_u32::<E>()? as u64,
                },
                size: reader.read_u32::<E>()?,
                path: reader.read_string::<E>()?,
            }),
            false => None,
        },
    })
}

/// Format used when creating textures from scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, num_derive::FromPrimitive)]
pub enum TextureFormat {
    /// Alpha-only texture format, 8 bit integer.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.Alpha8.html):
    ///
    /// This format only stores the alpha channel and doesn't hold any color data. It can be used by
    /// custom shaders for computing alpha independently of the other channels. Set the texture data
    /// in the same way as with other textures, for example using `Texture2D.SetPixels`, except only
    /// the alpha component from `Color` is used.
    Alpha8 = 1,

    /// A 16 bits/pixel texture format. Texture stores color with an alpha channel.
    Argb4444 = 2,

    /// Color texture format, 8 bit per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RGB24.html):
    ///
    /// Each of RGB color channels is stored as an 8-bit value in `[0..1]` range. In memory, the
    /// channel data is ordered this way: R, G, B bytes one after another.
    ///
    /// Note that there are almost no GPUs that support this format natively, so at texture load
    /// time it is converted into an [`RGBA32`](TextureFormat::Rgba32) format.
    /// [`RGB24`](TextureFormat::Rgb24) is thus only useful for some game build size savings.
    Rgb24 = 3,

    /// Color with alpha texture format, 8 bit per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RGBA32.html):
    ///
    /// Each of RGBA color channels is stored as an 8-bit value in `[0..1]` range. In memory, the
    /// channel data is ordered this way: R, G, B, A bytes one after another.
    Rgba32 = 4,

    /// Color with alpha texture format, 8 bit per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.ARGB32.html):
    ///
    /// Each of RGBA color channels is stored as an 8-bit value in `[0..1]` range. In memory, the
    /// channel data is ordered this way: A, R, G, B bytes one after another.
    ///
    /// Note that [`RGBA32`](TextureFormat::Rgba32) format might be slightly more efficient as
    /// the data layout in memory more closely matches what the graphics APIs expect.
    Argb32 = 5,

    /// A 16 bit color texture format.
    Rgb565 = 7,

    /// Single channel (R) texture format, 16 bit integer.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.R16.html):
    ///
    /// Currently, this texture format is only useful for runtime or native code plugins as there
    /// is no support for texture importing for this format.
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    R16 = 9,

    /// Compressed color texture format.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.DXT1.html):
    ///
    /// [`DXT1`](TextureFormat::Dxt1) (BC1) format compresses textures to 4 bits per pixel, and is
    /// widely supported on PC and console platforms.
    ///
    /// It is a good format to compress most of RGB textures. For RGBA (with alpha) textures,
    /// use [`DXT5`](TextureFormat::Dxt5).
    Dxt1 = 10,

    /// Compressed color with alpha channel texture format.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.DXT5.html):
    ///
    /// [`DXT5`](TextureFormat::Dxt5) (BC3) format compresses textures to 8 bits per pixel, and is
    /// widely supported on PC and console platforms.
    ///
    /// It is a good format to compress RGBA textures. For RGB (without alpha) textures,
    /// [`DXT1`](TextureFormat::Dxt1) is better. When targeting DX11-class hardware (modern PC,
    /// PS4, XboxOne), using [`BC7`](TextureFormat::Bc7) might be useful, since compression quality
    /// is often better.
    Dxt5 = 12,

    /// Color and alpha texture format, 4 bit per channel.
    RgbA4444 = 13,

    /// Color with alpha texture format, 8 bit per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.BGRA32.html):
    ///
    /// [`BGRA32`](TextureFormat::Bgra32) format is used by WebCamTexture on some platforms. Each
    /// of RGBA color channels is stored as an 8-bit value in `[0..1]` range. In memory, the channel
    /// data is ordered this way: B, G, R, A bytes one after another.
    Bgra32 = 14,

    /// Scalar (R) texture format, 16 bit floating point.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RHalf.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    RHalf = 15,

    /// Two color (RG) texture format, 16 bit floating point per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RGHalf.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    RgHalf = 16,

    /// RGB color and alpha texture format, 16 bit floating point per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RGBAHalf.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    RgbaHalf = 17,

    /// Scalar (R) texture format, 32 bit floating point.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RFloat.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    RFloat = 18,

    /// Two color (RG) texture format, 32 bit floating point per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RGFloat.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    RgFloat = 19,

    /// RGB color and alpha texture format, 32-bit floating point per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RGBAFloat.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    RgbaFloat = 20,

    /// A format that uses the YUV color space and is often used for video encoding or playback.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.YUY2.html):
    ///
    /// Currently, this texture format is only useful for native code plugins as there is no support
    /// for texture importing or pixel access for this format. [`YUY2`](TextureFormat::Yuy2) is
    /// implemented for Direct3D 9, Direct3D 11, and Xbox One.
    Yuy2 = 21,

    /// RGB HDR format, with 9 bit mantissa per channel and a 5 bit shared exponent.
    ///
    /// Three partial-precision floating-point numbers encoded into a single 32-bit value all
    /// sharing the same 5-bit exponent (variant of s10e5, which is sign bit, 10-bit mantissa,
    /// and 5-bit biased(15) exponent). There is no sign bit, and there is a shared 5-bit biased(15)
    /// exponent and a 9-bit mantissa for each channel. [`RGB9e5Float`](TextureFormat::Rgb9e5Float)
    /// is implemented for Direct3D 11, Direct3D 12, Xbox One, Playstation 4, OpenGL 3.0+, metal and
    /// Vulkan. The format is used for Precomputed Enlighten Realtime Global Illumination textures
    /// on supported platforms.
    Rgb9e5Float = 22,

    /// Compressed one channel (R) texture format.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.BC4.html):
    ///
    /// BC4 format compresses textures to 4 bits per pixel, keeping only the red color channel.
    /// It is widely supported on PC and console platforms.
    ///
    /// It is a good format to compress single-channel textures like heightfields or masks. For
    /// two channel textures, see [`BC5`](TextureFormat::Bc5).
    Bc4 = 26,

    /// Compressed two-channel (RG) texture format.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.BC5.html):
    ///
    /// BC5 format compresses textures to 8 bits per pixel, keeping only the red and green color
    /// channels. It is widely supported on PC and console platforms.
    ///
    /// It is a good format to compress two-channel textures, e.g. as a compression format for
    /// tangent space normal maps or velocity fields. For one channel textures, see [`BC4`](TextureFormat::Bc4).
    Bc5 = 27,

    /// HDR compressed color texture format.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.BC6H.html):
    ///
    /// [`BC6H`](TextureFormat::Bc6H) format compresses RGB HDR textures to 8 bits per pixel, and is
    /// supported on DX11-class PC hardware, as well as PS4 and XboxOne.
    ///
    /// It is a good format for compressing floating point texture data (skyboxes, reflection probes,
    /// lightmaps, emissive textures), e.g. textures that uncompressed would be in
    /// [`RGBAHalf`](TextureFormat::RgbaHalf) or [`RGBAFloat`](TextureFormat::RgbaFloat) formats.
    /// Note that [`BC6H`](TextureFormat::Bc6H) does not retain the alpha channel; it only stores
    /// RGB color channels.
    ///
    /// When loading [`BC6H`](TextureFormat::Bc6H) textures on a platform that does not support it,
    /// he texture will be decompressed into RGBAHalf format (64 bits per pixel) at load time.
    /// Note that [`BC7`](TextureFormat::Bc7) is not available on Mac when using OpenGL.
    Bc6H = 24,

    /// High quality compressed color texture format.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.BC7.html):
    ///
    /// [`BC7`](TextureFormat::Bc7) format compresses textures to 8 bits per pixel, and is supported
    /// on DX11-class PC hardware, as well as PS4 and XboxOne.
    ///
    /// Generally it produces better quality than the more widely available [`DXT5`](TextureFormat::Dxt5)
    /// format, however it requires a modern GPU, and texture compression during import time is often
    /// slower too. Note that [`BC7`](TextureFormat::Bc7) is not available on Mac when using OpenGL.
    ///
    /// When loading [`BC7`](TextureFormat::Bc7) textures on a platform that does not support it,
    /// the texture will be decompressed into RGBA32 format (32 bits per pixel) at load time.
    Bc7 = 25,

    /// Compressed color texture format with Crunch compression for smaller storage sizes.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.DXT1Crunched.html):
    ///
    /// The [`DXT1Crunched`](TextureFormat::Dxt1Crunched) format is similar to the
    /// [`DXT1`](TextureFormat::Dxt1) format but with additional JPEG-like lossy compression for
    /// storage size reduction. Textures are transcoded into the [`DXT1`](TextureFormat::Dxt1)
    /// format at load time.
    Dxt1Crunched = 28,

    /// Compressed color with alpha channel texture format with Crunch compression for smaller
    /// storage sizes.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.DXT5Crunched.html):
    ///
    /// The [`DXT5Crunched`](TextureFormat::Dxt5Crunched) format is similar to the
    /// [`DXT5`](TextureFormat::Dxt5) format but with additional JPEG-like lossy compression for
    /// storage size reduction. Textures are transcoded into the [`DXT5`](TextureFormat::Dxt5)
    /// format at load time.
    Dxt5Crunched = 29,

    /// PowerVR (iOS) 2 bits/pixel compressed color texture format.
    PvrtcRgb2 = 30,

    /// PowerVR (iOS) 2 bits/pixel compressed with alpha channel texture format.
    PvrtcRgba2 = 31,

    /// PowerVR (iOS) 4 bits/pixel compressed color texture format.
    PvrtcRgb4 = 32,

    /// PowerVR (iOS) 4 bits/pixel compressed with alpha channel texture format.
    PvrtcRgba4 = 33,

    /// ETC (GLES2.0) 4 bits/pixel compressed RGB texture format.
    EtcRgb4 = 34,

    AtcRgb4 = 35,
    AtcRgba8 = 36,

    /// ETC2 / EAC (GL ES 3.0) 4 bits/pixel compressed unsigned single-channel texture format.
    EacR = 41,

    /// ETC2 / EAC (GL ES 3.0) 4 bits/pixel compressed signed single-channel texture format.
    EacRSigned = 42,

    /// ETC2 / EAC (GL ES 3.0) 8 bits/pixel compressed unsigned dual-channel (RG) texture format.
    EacRg = 43,

    /// ETC2 / EAC (GL ES 3.0) 8 bits/pixel compressed signed dual-channel (RG) texture format.
    EacRgSigned = 44,

    /// ETC2 (GL ES 3.0) 4 bits/pixel compressed RGB texture format.
    Etc2Rgb = 45,

    /// ETC2 (GL ES 3.0) 4 bits/pixel RGB+1-bit alpha texture format.
    Etc2Rgba1 = 46,

    /// ETC2 (GL ES 3.0) 8 bits/pixel compressed RGBA texture format.
    Etc2Rgba8 = 47,

    /// ASTC (4x4 pixel block in 128 bits) compressed RGB texture format.
    AstcRgb4x4 = 48,

    /// ASTC (5x5 pixel block in 128 bits) compressed RGB texture format.
    AstcRgb5x5 = 49,

    /// ASTC (6x6 pixel block in 128 bits) compressed RGB texture format.
    AstcRgb6x6 = 50,

    /// ASTC (8x8 pixel block in 128 bits) compressed RGB texture format.
    AstcRgb8x8 = 51,

    /// ASTC (10x10 pixel block in 128 bits) compressed RGB texture format.
    AstcRgb10x10 = 52,

    /// ASTC (12x12 pixel block in 128 bits) compressed RGB texture format.
    AstcRgb12x12 = 53,

    /// ASTC (4x4 pixel block in 128 bits) compressed RGBA texture format.
    AstcRgba4x4 = 54,

    /// ASTC (5x5 pixel block in 128 bits) compressed RGBA texture format.
    AstcRgba5x5 = 55,

    /// ASTC (6x6 pixel block in 128 bits) compressed RGBA texture format.
    AstcRgba6x6 = 56,

    /// ASTC (8x8 pixel block in 128 bits) compressed RGBA texture format.
    AstcRgba8x8 = 57,

    /// ASTC (10x10 pixel block in 128 bits) compressed RGBA texture format.
    AstcRgba10x10 = 58,

    /// ASTC (12x12 pixel block in 128 bits) compressed RGBA texture format.
    AstcRgba12x12 = 59,

    EtcRgb43ds = 60,
    EtcRgba83ds = 61,

    /// Two color (RG) texture format, 8-bits per channel.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.RG16.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    Rg16 = 62,

    /// Single channel (R) texture format, 8 bit integer.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.R8.html):
    ///
    /// Note that not all graphics cards support all texture formats, use
    /// `SystemInfo.SupportsTextureFormat` to check.
    R8 = 63,

    /// Compressed color texture format with Crunch compression for smaller storage sizes.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.ETC_RGB4Crunched.html):
    ///
    /// The [`ETC_RGB4Crunched`](TextureFormat::EtcRgb4crunched) format is similar to the
    /// [`ETC_RGB4`](TextureFormat::EtcRgb4) format but with additional JPEG-like lossy compression
    /// for storage size reduction. Textures are transcoded into the [`ETC_RGB4`](TextureFormat::EtcRgb4)
    /// format at load time.
    EtcRgb4crunched = 64,

    /// Compressed color with alpha channel texture format using Crunch compression for smaller
    /// storage sizes.
    ///
    /// According to [unity documentation](https://docs.unity3d.com/ScriptReference/TextureFormat.ETC2_RGBA8Crunched.html):
    ///
    /// The [`ETC2_RGBA8Crunched`](TextureFormat::Etc2Rgba8crunched) format is similar to the
    /// [`ETC2_RGBA8`](TextureFormat::Etc2Rgba8) format but with additional JPEG-like lossy
    /// compression for storage size reduction. Textures are transcoded into the
    /// [`ETC2_RGBA8`](TextureFormat::Etc2Rgba8) format at load time.
    Etc2Rgba8crunched = 65,

    /// ASTC (4x4 pixel block in 128 bits) compressed RGB(A) HDR texture format.
    AstcHdr4x4 = 66,

    /// ASTC (5x5 pixel block in 128 bits) compressed RGB(A) HDR texture format.
    AstcHdr5x5 = 67,

    /// ASTC (6x6 pixel block in 128 bits) compressed RGB(A) HDR texture format.
    AstcHdr6x6 = 68,

    /// ASTC (8x8 pixel block in 128 bits) compressed RGB(A) HDR texture format.
    AstcHdr8x8 = 69,

    /// ASTC (10x10 pixel block in 128 bits) compressed RGB(A) HDR texture format.
    AstcHdr10x10 = 70,

    /// ASTC (12x12 pixel block in 128 bits) compressed RGB(A) HDR texture format.
    AstcHdr12x12 = 71,
}

pub fn decode_texture(
    data: &[u8],
    format: TextureFormat,
    width: usize,
    height: usize,
) -> Result<DynamicImage, Box<dyn Error>> {
    let mut input = Vec::from(data);
    let mut output = vec![0u32; width * height];

    if [
        TextureFormat::Rgb565,
        TextureFormat::Dxt1,
        TextureFormat::Dxt1Crunched,
        TextureFormat::Dxt5,
        TextureFormat::Dxt5Crunched,
    ]
    .contains(&format)
    {
        // swap bytes for XBOX 360 textures
        // FIXME: textures on other platforms should not be swapped
        for i in 0..(input.len() / 2) {
            input.swap(i * 2, i * 2 + 1);
        }
    }

    // TODO: unpack crunched textures

    match format {
        // ATC
        TextureFormat::AtcRgba8 => {
            texture2ddecoder::decode_atc_rgba8(&input, width, height, &mut output)?
        }
        TextureFormat::AtcRgb4 => {
            texture2ddecoder::decode_atc_rgb4(&input, width, height, &mut output)?
        }

        // ASTC
        TextureFormat::AstcRgb4x4 | TextureFormat::AstcRgba4x4 | TextureFormat::AstcHdr4x4 => {
            texture2ddecoder::decode_astc_4_4(&input, width, height, &mut output)?
        }
        TextureFormat::AstcRgb5x5 | TextureFormat::AstcRgba5x5 | TextureFormat::AstcHdr5x5 => {
            texture2ddecoder::decode_astc_5_5(&input, width, height, &mut output)?
        }
        TextureFormat::AstcRgb6x6 | TextureFormat::AstcRgba6x6 | TextureFormat::AstcHdr6x6 => {
            texture2ddecoder::decode_astc_6_6(&input, width, height, &mut output)?
        }
        TextureFormat::AstcRgb8x8 | TextureFormat::AstcRgba8x8 | TextureFormat::AstcHdr8x8 => {
            texture2ddecoder::decode_astc_8_8(&input, width, height, &mut output)?
        }
        TextureFormat::AstcRgb10x10
        | TextureFormat::AstcRgba10x10
        | TextureFormat::AstcHdr10x10 => {
            texture2ddecoder::decode_astc_10_10(&input, width, height, &mut output)?
        }
        TextureFormat::AstcRgb12x12
        | TextureFormat::AstcRgba12x12
        | TextureFormat::AstcHdr12x12 => {
            texture2ddecoder::decode_astc_12_12(&input, width, height, &mut output)?
        }

        // BCn
        TextureFormat::Dxt1 | TextureFormat::Dxt1Crunched => {
            texture2ddecoder::decode_bc1(&input, width, height, &mut output)?
        }
        TextureFormat::Dxt5 | TextureFormat::Dxt5Crunched => {
            texture2ddecoder::decode_bc3(&input, width, height, &mut output)?
        }
        TextureFormat::Bc4 => texture2ddecoder::decode_bc4(&input, width, height, &mut output)?,
        TextureFormat::Bc5 => texture2ddecoder::decode_bc5(&input, width, height, &mut output)?,
        // FIXME: BC6H is signed or unsigned?
        TextureFormat::Bc6H => {
            texture2ddecoder::decode_bc6_unsigned(&input, width, height, &mut output)?
        }
        TextureFormat::Bc7 => texture2ddecoder::decode_bc7(&input, width, height, &mut output)?,

        TextureFormat::EacR => texture2ddecoder::decode_eacr(&input, width, height, &mut output)?,
        TextureFormat::EacRg => texture2ddecoder::decode_eacrg(&input, width, height, &mut output)?,
        TextureFormat::EacRgSigned => {
            texture2ddecoder::decode_eacrg_signed(&input, width, height, &mut output)?
        }
        TextureFormat::EacRSigned => {
            texture2ddecoder::decode_eacr_signed(&input, width, height, &mut output)?
        }

        // ETC
        TextureFormat::EtcRgb4
        | TextureFormat::EtcRgb43ds
        | TextureFormat::EtcRgba83ds
        | TextureFormat::EtcRgb4crunched => {
            texture2ddecoder::decode_etc1(&input, width, height, &mut output)?
        }
        TextureFormat::Etc2Rgb => {
            texture2ddecoder::decode_etc2_rgb(&input, width, height, &mut output)?
        }
        TextureFormat::Etc2Rgba1 => {
            texture2ddecoder::decode_etc2_rgba1(&input, width, height, &mut output)?
        }
        TextureFormat::Etc2Rgba8 | TextureFormat::Etc2Rgba8crunched => {
            texture2ddecoder::decode_etc2_rgba8(&input, width, height, &mut output)?
        }

        // PVRTC
        TextureFormat::PvrtcRgb2 | TextureFormat::PvrtcRgba2 => {
            texture2ddecoder::decode_pvrtc_2bpp(&input, width, height, &mut output)?
        }
        TextureFormat::PvrtcRgb4 | TextureFormat::PvrtcRgba4 => {
            texture2ddecoder::decode_pvrtc_4bpp(&input, width, height, &mut output)?
        }

        // raw
        TextureFormat::Alpha8 => {
            let img = GrayImage::from_raw(width as u32, height as u32, input)
                .ok_or("failed to decode alpha8 texture")?;
            return Ok(DynamicImage::ImageLuma8(img));
        }
        // TODO: more raw formats
        _ => return Err(format!("unsupported texture format: {:?}", format).into()),
    };

    Ok(DynamicImage::ImageRgba8(RgbaImage::from_fn(
        width as u32,
        height as u32,
        |x, y| {
            let i = (y * width as u32 + x) as usize;

            // decoded image is in ARGB format, so we need to convert it to RGBA
            let rgba = output[i].rotate_left(8);

            image::Rgba(rgba.to_be_bytes())
        },
    )))
}
