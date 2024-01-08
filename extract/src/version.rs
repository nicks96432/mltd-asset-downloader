use std::fmt::{Debug, Display};
use std::io::ErrorKind;
use std::str::FromStr;

/// Unity version string
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub suffix: &'static str,
}

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self { major: 2, minor: 5, patch: 0, suffix: "f5" }
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

        let suffix = Box::leak(String::from(&nums[2][suffix_start..]).into_boxed_str());

        Ok(Self { major, minor, patch, suffix })
    }
}

macro_rules! gen_version {
    ($name:ident,$major:expr, $minor:expr, $patch:expr) => {
        pub const $name: Version =
            Version { major: $major, minor: $minor, patch: $patch, suffix: "" };
    };
    ($name:ident,$major:expr, $minor:expr, $patch:expr, $suffix:ident) => {
        pub const $name: Version =
            Version { major: $major, minor: $minor, patch: $patch, suffix: stringify!($suffix) };
    };
}

gen_version!(UNITY_VERSION_2_6_0, 2, 6, 0);

gen_version!(UNITY_VERSION_3_0_0, 3, 0, 0);
gen_version!(UNITY_VERSION_3_4_0, 3, 4, 0);
gen_version!(UNITY_VERSION_3_5_0, 3, 5, 0);
gen_version!(UNITY_VERSION_3_5_7, 3, 5, 7);

gen_version!(UNITY_VERSION_4_0_0, 4, 0, 0);
gen_version!(UNITY_VERSION_4_2_0, 4, 2, 0);
gen_version!(UNITY_VERSION_4_3_0, 4, 3, 0);
gen_version!(UNITY_VERSION_4_5_0, 4, 5, 0);
gen_version!(UNITY_VERSION_4_7_2, 4, 7, 2);

gen_version!(UNITY_VERSION_5_0_0_F4, 5, 0, 0, f4);
gen_version!(UNITY_VERSION_5_1_5_F1, 5, 1, 5, f1);
gen_version!(UNITY_VERSION_5_2_0_F2, 5, 2, 0, f2);
gen_version!(UNITY_VERSION_5_3_0_F1, 5, 3, 0, f1);
gen_version!(UNITY_VERSION_5_4_0_F3, 5, 4, 0, f3);
gen_version!(UNITY_VERSION_5_4_2_F2, 5, 4, 2, f2);
gen_version!(UNITY_VERSION_5_4_6_F1, 5, 4, 6, f1);
gen_version!(UNITY_VERSION_5_4_6_F3, 5, 4, 6, f3);
gen_version!(UNITY_VERSION_5_5_6_F1, 5, 5, 6, f1);
gen_version!(UNITY_VERSION_5_6_0_B1, 5, 6, 0, b1);
gen_version!(UNITY_VERSION_5_6_7_F1, 5, 6, 7, f1);

gen_version!(UNITY_VERSION_2017_1_0_B1, 2017, 1, 0, b1);
gen_version!(UNITY_VERSION_2017_1_0_B2, 2017, 1, 0, b2);
gen_version!(UNITY_VERSION_2017_3_0_B1, 2017, 3, 0, b1);
gen_version!(UNITY_VERSION_2017_4_40_F1, 2017, 4, 40, f1);

gen_version!(UNITY_VERSION_2018_1_0_B2, 2018, 1, 0, b2);
gen_version!(UNITY_VERSION_2018_1_9_F2, 2018, 1, 9, f2);
gen_version!(UNITY_VERSION_2018_2_0_B1, 2018, 2, 0, b1);

gen_version!(UNITY_VERSION_2019_1_0_B1, 2019, 1, 0, b1);
gen_version!(UNITY_VERSION_2019_3_0_F6, 2019, 3, 0, f6);
gen_version!(UNITY_VERSION_2019_4_9_F1, 2019, 4, 9, f1);

gen_version!(UNITY_VERSION_2020_1_0_B1, 2020, 1, 0, b1);
gen_version!(UNITY_VERSION_2020_1_0_F1, 2020, 1, 0, f1);
gen_version!(UNITY_VERSION_2020_2_0_B1, 2020, 2, 0, b1);

gen_version!(UNITY_VERSION_2021_1_0_B1, 2021, 1, 0, b1);

gen_version!(UNITY_VERSION_2022_2_0_A18, 2022, 2, 0, a18);
gen_version!(UNITY_VERSION_2022_2_0_F1, 2022, 2, 0, f1);
gen_version!(UNITY_VERSION_2022_3_2_F1, 2022, 3, 2, f1);

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
            rand_range(0x61u8..0x7bu8) as char,
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
