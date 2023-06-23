use crate::error::Error;

use std::backtrace::Backtrace;
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Unity version string
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: String,
    pub patch: String,
}

impl Version {
    /// Returns whether this [`Version`] is a newer Unity version.
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
    /// ```
    /// use std::str::FromStr;
    /// use unity::bundle::Version;
    ///
    /// assert!(Version::from_str("2020.3.34f1").unwrap().is_new());
    /// assert!(Version::from_str("2021.3.2f1").unwrap().is_new());
    /// assert!(Version::from_str("2022.1.1f1").unwrap().is_new());
    /// assert!(Version::from_str("2023.1.0a4").unwrap().is_new());
    /// ```
    pub fn is_new(&self) -> bool {
        self.major >= 2023
            || (self.major == 2022 && self >= &Version::from_str("2022.1.1f1").unwrap())
            || (self.major == 2021 && self >= &Version::from_str("2021.3.2f1").unwrap())
            || (self.major == 2020 && self >= &Version::from_str("2020.3.34f1").unwrap())
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self {
            major: 2,
            minor: String::from("0"),
            patch: String::from("f5"),
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        log::trace!("input version: {}", s);

        let nums: Vec<&str> = s.split('.').collect();
        if nums.len() != 3 {
            return Err(Error::InvalidVersion {
                version: s.to_string(),
                backtrace: Backtrace::capture(),
            });
        }

        Ok(Self {
            major: nums[0].parse()?,
            minor: nums[1].to_owned(),
            patch: nums[2].to_owned(),
        })
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use super::Version;
    use mltd_utils::{rand_ascii_string, rand_range};

    use std::str::FromStr;

    #[test]
    fn test_from_str() {
        let version = format!(
            "{}.{}.{}{}{}",
            rand_range(0..5000),
            rand_range(0..5000),
            rand_range(0..5000),
            rand_ascii_string(1).into_inner()[0],
            rand_range(0..5000),
        );
        Version::from_str(&version).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid() {
        let version = format!(
            "{}a.{}.{}",
            String::from_utf8(rand_ascii_string(5).into_inner()).unwrap(),
            String::from_utf8(rand_ascii_string(5).into_inner()).unwrap(),
            String::from_utf8(rand_ascii_string(5).into_inner()).unwrap(),
        );

        Version::from_str(&version).unwrap();
    }
}
