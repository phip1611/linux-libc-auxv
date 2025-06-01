use crate::aux_var::{AuxVarType, ParseAuxVarTypeError};
use core::fmt::{Debug, Formatter};

type AuxVarTypeRaw = usize;

/// Serialized form of an [`AuxVar`] as used in the Linux ABI.
///
/// In memory, each entry is a `(usize, usize)`-pair. Depending on the `key`,
/// the `value` might be a boolean, an integer, or a pointer into the
/// _auxv data area_.
///
/// [`AuxVar`]: crate::AuxVar
#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AuxVarRaw {
    /// Encoded variant of [`AuxVarType`].
    key: AuxVarTypeRaw,
    value: usize,
}

impl AuxVarRaw {
    /// Creates a new struct.
    pub fn new(key: impl Into<AuxVarTypeRaw>, val: usize) -> Self {
        Self {
            key: key.into(),
            value: val,
        }
    }

    /// Tries to parse the underlying raw value as [`AuxVarType`].
    pub fn key(&self) -> Result<AuxVarType, ParseAuxVarTypeError> {
        self.key.try_into()
    }

    /// Returns the raw value.
    pub const fn value(&self) -> usize {
        self.value
    }
}

impl Debug for AuxVarRaw {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.key().unwrap().value_in_data_area() {
            write!(f, "{:?}: @ 0x{:x?}", self.key(), self.value())
        } else {
            write!(f, "{:?}: {:x?}", self.key(), self.value())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_serialized_aux_entry_size() {
        #[cfg(target_arch = "x86")]
        assert_eq!(size_of::<AuxVarRaw>(), 8);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(size_of::<AuxVarRaw>(), 16);

        // Generic, on all platforms:
        assert_eq!(size_of::<AuxVarRaw>(), 2 * size_of::<usize>());
    }
}
