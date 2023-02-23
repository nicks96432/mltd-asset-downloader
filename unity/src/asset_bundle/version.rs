use crate::UnityError;
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Unity version string
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetBundleVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: String,
}

impl Debug for AssetBundleVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for AssetBundleVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for AssetBundleVersion {
    type Err = UnityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        log::trace!("input version: {}", s);

        let nums: Vec<&str> = s.split('.').collect();
        if nums.len() != 3 {
            return Err(UnityError::InvalidVersion);
        }

        Ok(Self {
            major: nums[0].parse()?,
            minor: nums[1].parse()?,
            patch: nums[2].to_owned(),
        })
    }
}

impl AssetBundleVersion {
    /// Returns whether this [`AssetBundleVersion`] is a newer Unity version.
    ///
    /// from [UnityPy](https://github.com/K0lb3/UnityPy/blob/c8d41de4ee914bb63d765fcbeb063531e1eea460/UnityPy/files/BundleFile.py#L99):
    ///
    /// According to [this link](https://issuetracker.unity3d.com/issues/files-within-assetbundles-do-not-start-on-aligned-boundaries-breaking-patching-on-nintendo-switch),
    /// Unity CN introduced encryption before the alignment fix was introduced,
    /// and they used the same flag for the encryption as later on the
    /// alignment fix, so we have to check the version to determine the correct
    /// flag set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::str::FromStr;
    /// use unity::AssetBundleVersion;
    ///
    /// assert!(AssetBundleVersion::from_str("2020.3.34f1").unwrap().is_new())
    /// assert!(AssetBundleVersion::from_str("2021.3.2f1").unwrap().is_new())
    /// assert!(AssetBundleVersion::from_str("2022.1.1f1").unwrap().is_new())
    /// assert!(AssetBundleVersion::from_str("2023.1.0a4").unwrap().is_new())
    /// ```
    pub fn is_new(&self) -> bool {
        self.major >= 2023
            || (self.major == 2022 && self >= &AssetBundleVersion::from_str("2022.1.1f1").unwrap())
            || (self.major == 2021 && self >= &AssetBundleVersion::from_str("2021.3.2f1").unwrap())
            || (self.major == 2020 && self >= &AssetBundleVersion::from_str("2020.3.34f1").unwrap())
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_asset_bundle_version_read() {
        todo!()
    }
}
