use crate::aux_var::AuxVarType;
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;

/// Serialized form of an auxiliary vector entry / an AT variable. Each entry is a
/// `(usize, usize)`-pair in memory. The lifetime is bound to the one of the buffer,
/// where this structure is parsed from.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct AuxVarSerialized<'a> {
    key: AuxVarType,
    val: usize,
    // ZST. Required to get the right life time, when this is transformed to a [`crate::AuxVar`].
    _marker: PhantomData<&'a ()>,
}

impl<'a> AuxVarSerialized<'a> {
    /// Returns the key.
    pub const fn key(&self) -> AuxVarType {
        self.key
    }

    /// Returns the raw value.
    pub const fn val(&self) -> usize {
        self.val
    }
}

impl<'a> Debug for AuxVarSerialized<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.key.value_in_data_area() {
            write!(f, "{:?}: @ {:?}", self.key(), self.val() as *const u8)
        } else {
            write!(f, "{:?}: {:?}", self.key(), self.val() as *const u8)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use core::mem::size_of;

    #[test]
    fn test_serialized_aux_entry_size() {
        #[cfg(target_arch = "x86")]
        assert_eq!(size_of::<AuxVarSerialized>(), 8);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(size_of::<AuxVarSerialized>(), 16);
    }
}
