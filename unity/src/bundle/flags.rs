use crate::compression::Method as CompressionMethod;
use crate::error::Error;

use num_traits::FromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Flags {
    pub bits: u32,
    pub new: bool,
}

impl Flags {
    /// Creates a new [`Flags`] of old Unity version.
    ///
    /// For version differences, see [`AssetBundleVersion`][v] for more details.
    ///
    /// [v]: crate::bundle::Version
    pub fn new(flag: u32) -> Self {
        Self {
            bits: flag,
            new: false,
        }
    }

    /// Returns the compression method of this [`Flags`].
    ///
    /// # Errors
    ///
    /// This function will return [`Error::UnknownCompressionMethod`] if
    /// the compression method is unknown.
    pub fn compression_method(&self) -> Result<CompressionMethod, Error> {
        let value = self.bits & 0x3f;

        CompressionMethod::from_u32(value).ok_or_else(|| Error::UnknownCompressionMethod)
    }

    /// Returns whether the block info and asset info in
    /// [`InfoBlock`][InfoBlock] are combined.
    ///
    /// [InfoBlock]: crate::bundle::InfoBlock
    pub fn info_block_combined(&self) -> bool {
        self.bits & 0x40 != 0
    }

    /// Returns whether the [`InfoBlock`][InfoBlock] is at the end of this
    /// bundle file.
    ///
    /// [InfoBlock]: crate::bundle::InfoBlock
    pub fn info_block_end(&self) -> bool {
        self.bits & 0x80 != 0
    }

    /// Returns whether the [`InfoBlock`][InfoBlock] has padding at start.
    ///
    /// [InfoBlock]: crate::bundle::InfoBlock
    pub fn info_block_padding(&self) -> bool {
        self.new && self.bits & 0x200 != 0
    }
}
