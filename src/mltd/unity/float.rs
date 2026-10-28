//! Numbers read from the serialized objects.

use std::fmt::Formatter;

use serde::de::{Deserialize, Deserializer, Visitor};

use super::Error;

/// Number that accepts any numeric type of the type tree.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Float(pub f64);

impl Float {
    /// Rounds the number to an integer.
    ///
    /// # Errors
    ///
    /// This method fails if the rounded number does not fit in an `u32`.
    pub(super) fn to_u32(self) -> crate::error::Result<u32> {
        let value = self.0.round();
        if !(0.0..=f64::from(u32::MAX)).contains(&value) {
            return Err(Error::corrupted("read number", "value is out of range").into());
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(value as u32)
    }
}

impl<'de> Deserialize<'de> for Float {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FloatVisitor;

        #[allow(clippy::cast_precision_loss)]
        impl Visitor<'_> for FloatVisitor {
            type Value = Float;

            fn expecting(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str("a number")
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E> {
                Ok(Float(f64::from(v)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Float(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Float(v as f64))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Float(v as f64))
            }
        }

        deserializer.deserialize_any(FloatVisitor)
    }
}
