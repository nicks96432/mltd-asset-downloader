use std::fmt::{Display, Formatter};

use crate::compression::Method as CompressionMethod;
use crate::error::Error;
use crate::utils::bool_to_yes_no;

use num_traits::FromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flags {
    pub bits: u32,
}

impl Flags {
    /// Creates a new [`Flags`] of old Unity version.
    ///
    /// For version differences, see [`AssetBundleVersion`][v] for more details.
    ///
    /// [v]: crate::bundle::Version
    pub fn new(flag: u32) -> Self {
        Self { bits: flag }
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
        self.bits & 0x200 != 0
    }
}

impl Display for Flags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // XXX: maybe try a different way to indent output?
        let indent = f.width().unwrap_or(0);

        writeln!(f, "{:indent$}Flags:", "", indent = indent)?;

        writeln!(
            f,
            "{:indent$}Compression method:                     {}",
            "",
            match self.compression_method() {
                Ok(c) => c.to_string(),
                Err(_) => "unknown".to_owned(),
            },
            indent = indent + 4
        )?;

        writeln!(
            f,
            "{:indent$}Block info and path info are combined?  {}",
            "",
            bool_to_yes_no(self.info_block_combined()),
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Info block is at the end?               {}",
            "",
            bool_to_yes_no(self.info_block_end()),
            indent = indent + 4
        )?;
        writeln!(
            f,
            "{:indent$}Info block has padding at the begining? {}",
            "",
            bool_to_yes_no(self.info_block_padding()),
            indent = indent + 4
        )?;

        Ok(())
    }
}
