macro_rules! impl_default {
    ($type:ident) => {
        impl Default for $type {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

macro_rules! impl_from_file_error {
    ($type:ident) => {
        impl From<$type> for crate::UnityError {
            fn from(value: $type) -> Self {
                crate::UnityError::FileError(crate::FileError::$type(value))
            }
        }
    };
}

macro_rules! impl_from_for_error {
    ($type:ident) => {
        impl From<$type> for crate::UnityError {
            fn from(value: $type) -> Self {
                crate::UnityError::$type(value)
            }
        }
    };
}

macro_rules! impl_try_from_into_vec {
    ($type:ident) => {
        impl TryFrom<Vec<u8>> for $type {
            type Error = crate::UnityError;

            fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                Self::read(&mut std::io::Cursor::new(&value))
            }
        }

        impl TryInto<Vec<u8>> for $type {
            type Error = crate::UnityError;

            fn try_into(self) -> Result<Vec<u8>, Self::Error> {
                let mut buf = Vec::new();
                self.write(&mut buf)?;

                Ok(buf)
            }
        }
    };
}

pub(crate) use impl_default;
pub(crate) use impl_from_file_error;
pub(crate) use impl_from_for_error;
pub(crate) use impl_try_from_into_vec;
