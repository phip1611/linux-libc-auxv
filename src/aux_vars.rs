/*
MIT License

Copyright (c) 2021 Philipp Schuster

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
//! Module for [`AuxVarType`].

use enum_iterator::IntoEnumIterator;

/// Type is only used as helper for the [`crate::AuxVecIter`] to parse the binary data.
/// This field is packed, because on 32-bit [`crate::AuxVarType`] is 32-bit long. We don't
/// want to have padding between the value and the key there.
#[repr(C, packed)]
pub(crate) struct AuxVarSerialized {
    key: AuxVarType,
    val: usize,
}

impl AuxVarSerialized {
    /// Returns the key.
    pub const fn key(&self) -> AuxVarType {
        self.key
    }

    /// Returns the val.
    pub const fn val(&self) -> usize {
        self.val
    }
}

/// All types of auxiliary variables that Linux supports for the initial stack.
/// Also called "AT"-variables in Linux source code.
///
/// According to some Linux code comments, `0-17` are architecture independent
/// The ones above and including `32` are for `x86_64`. Values above 40 are for power PC.
///
/// More info:
/// * <https://elixir.bootlin.com/linux/latest/source/include/uapi/linux/auxvec.h>
/// * <https://elixir.bootlin.com/linux/latest/source/fs/binfmt_elf.c#L259>
/// * <https://man7.org/linux/man-pages/man3/getauxval.3.html>
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Ord, Eq, IntoEnumIterator)]
#[repr(usize)]
pub enum AuxVarType {
    // ### architecture neutral
    /// end of vector
    AtNull = 0,
    /// entry should be ignored
    AtIgnore = 1,
    /// file descriptor of program
    AtExecfd = 2,
    /// program headers for program
    AtPhdr = 3,
    /// size of program header entry
    AtPhent = 4,
    /// number of program headers
    AtPhnum = 5,
    /// system page size
    AtPagesz = 6,
    /// The base address of the program interpreter (usually, the
    /// dynamic linker).
    AtBase = 7,
    /// flags
    AtFlags = 8,
    /// entry point of program
    AtEntry = 9,
    /// program is not ELF
    AtNotelf = 10,
    /// real uid
    AtUid = 11,
    /// effective uid
    AtEuid = 12,
    /// real gid
    AtGid = 13,
    /// effective gid
    AtEgid = 14,
    /// string identifying CPU for optimizations
    AtPlatform = 15,
    /// Arch dependent hints at CPU capabilities.
    /// On x86_64 these are the CPUID features.
    AtHwcap = 16,
    /// frequency at which times() increments
    AtClktck = 17,
    /// secure mode boolean
    AtSecure = 23,
    /// string identifying real platform, may differ from AtPlatform.
    AtBasePlatform = 24,
    /// address of 16 random bytes
    AtRandom = 25,
    /// extension of AtHwcap
    AtHwcap2 = 26,
    /// filename of program, for example "./my_executable\0"
    AtExecFn = 31,

    // ### according to Linux code: from here: x86_64
    /// The entry point to the system call function in the vDSO.
    /// Not present/needed on all architectures (e.g., absent on
    /// x86-64).
    AtSysinfo = 32,
    /// The address of a page containing the virtual Dynamic
    /// Shared Object (vDSO) that the kernel creates in order to
    /// provide fast implementations of certain system calls.
    AtSysinfoEhdr = 33,

    // ### according to Linux code: from here: PowerPC
    AtL1iCachesize = 40,
    AtL1iCachegeometry = 41,
    AtL1dCachesize = 42,
    AtL1dCachegeometry = 43,
    AtL2Cachesize = 44,
    AtL2Cachegeometry = 45,
    AtL3Cachesize = 46,
    AtL3Cachegeometry = 47,
}

impl AuxVarType {
    pub const fn val(self) -> usize {
        self as _
    }

    /// If this is true, the value of the key should be interpreted as pointer into
    /// the aux vector data area. Otherwise, the value of the key is an immediate value/integer.
    pub const fn value_in_data_area(self) -> bool {
        // this info can be found here:
        // https://elixir.bootlin.com/linux/latest/source/fs/binfmt_elf.c#L259
        match self {
            AuxVarType::AtNull => false,
            AuxVarType::AtIgnore => false,
            AuxVarType::AtExecfd => false,
            AuxVarType::AtPhdr => false,
            AuxVarType::AtPhent => false,
            AuxVarType::AtPhnum => false,
            AuxVarType::AtPagesz => false,
            AuxVarType::AtBase => false,
            AuxVarType::AtFlags => false,
            AuxVarType::AtEntry => false,
            AuxVarType::AtNotelf => false,
            AuxVarType::AtUid => false,
            AuxVarType::AtEuid => false,
            AuxVarType::AtGid => false,
            AuxVarType::AtEgid => false,
            // references C-str
            AuxVarType::AtPlatform => true,
            AuxVarType::AtHwcap => false,
            AuxVarType::AtClktck => false,
            AuxVarType::AtSecure => false,
            // references C-str
            AuxVarType::AtBasePlatform => true,
            // references random bytes
            AuxVarType::AtRandom => true,
            AuxVarType::AtHwcap2 => false,
            // references C-str
            AuxVarType::AtExecFn => true,
            AuxVarType::AtSysinfoEhdr => false,
            AuxVarType::AtSysinfo => false,
            AuxVarType::AtL1iCachesize => false,
            AuxVarType::AtL1iCachegeometry => false,
            AuxVarType::AtL1dCachesize => false,
            AuxVarType::AtL1dCachegeometry => false,
            AuxVarType::AtL2Cachesize => false,
            AuxVarType::AtL2Cachegeometry => false,
            AuxVarType::AtL3Cachesize => false,
            AuxVarType::AtL3Cachegeometry => false,
        }
    }

    /// The payload of some [`AuxVarType`] is stored in the aux var data area. Some of these
    /// data is null-terminated. Some has a fixed size. This helps to find out if there
    /// is a fixed size.
    pub const fn data_area_val_size_hint(self) -> Option<usize> {
        match self {
            AuxVarType::AtRandom => Some(16),
            _ => None,
        }
    }
}

impl From<usize> for AuxVarType {
    fn from(val: usize) -> Self {
        for variant in Self::into_enum_iter() {
            if variant.val() == val {
                return variant;
            }
        }
        panic!("invalid variant {}", val);
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
