macro_rules! impl_default {
    ($type:ident) => {
        impl Default for $type {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

macro_rules! impl_try_from_into_vec {
    ($type:ident) => {
        impl TryFrom<Vec<u8>> for $type {
            type Error = crate::error::Error;

            fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                Self::read(&mut std::io::Cursor::new(&value))
            }
        }

        impl TryInto<Vec<u8>> for $type {
            type Error = crate::error::Error;

            fn try_into(self) -> Result<Vec<u8>, Self::Error> {
                let mut buf = Vec::new();
                self.save(&mut buf)?;

                Ok(buf)
            }
        }
    };
}

pub(crate) use impl_default;
pub(crate) use impl_try_from_into_vec;
