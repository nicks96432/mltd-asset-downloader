use std::fmt::{Debug, Display};
use std::io::ErrorKind;
use std::str::FromStr;

/// Unity version string
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub suffix: String,
}

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self { major: 2, minor: 5, patch: 0, suffix: String::from("f5") }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}{}", self.major, self.minor, self.patch, self.suffix)
    }
}

impl FromStr for Version {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let nums: Vec<&str> = s.split('.').collect();

        let error = Self::Err::new(ErrorKind::InvalidInput, format!("invalid version: {}", s));
        if nums.len() != 3 {
            return Err(error);
        }

        let major = match nums[0].parse::<u32>() {
            Ok(major) => major,
            Err(_) => return Err(error),
        };
        let minor = match nums[1].parse::<u32>() {
            Ok(minor) => minor,
            Err(_) => return Err(error),
        };

        let suffix_start = match nums[2].find(|c: char| c.is_ascii_lowercase()) {
            Some(s) => s,
            None => nums[2].len(),
        };

        let patch = match nums[2][..suffix_start].parse::<u32>() {
            Ok(minor) => minor,
            Err(_) => return Err(error),
        };
        let suffix = nums[2][suffix_start..].to_owned();

        Ok(Self { major, minor, patch, suffix })
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    mltd_utils::init_test_logger!();
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use mltd_utils::{rand_ascii_string, rand_range};

    use super::Version;

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
