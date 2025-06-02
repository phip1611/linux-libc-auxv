/*
MIT License

Copyright (c) 2025 Philipp Schuster

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
use core::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, thiserror::Error)]
#[error("invalid aux var type: {0}")]
pub struct ParseAuxVarTypeError(usize);

/// Rust-style representation of the auxiliary variable's type.
///
/// Also see [`AuxVar`].
///
/// - `0-17` are architecture independent
/// - `>=32` are for `x86_64`.
/// - `>=40` are for power PC.
///
/// ## More Info
/// * <https://elixir.bootlin.com/linux/latest/source/include/uapi/linux/auxvec.h>
///
/// [`AuxVar`]: crate::AuxVar
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum AuxVarType {
    // ### architecture neutral
    /// end of vector
    Null = 0,
    /// entry should be ignored
    Ignore = 1,
    /// file descriptor of program
    ExecFd = 2,
    /// program headers for program
    Phdr = 3,
    /// size of program header entry
    Phent = 4,
    /// number of program headers
    Phnum = 5,
    /// system page size
    Pagesz = 6,
    /// The base address of the program interpreter (usually, the
    /// dynamic linker).
    Base = 7,
    /// Flags that apply on the whole auxiliary vector. See [`crate::AuxVarFlags`].
    Flags = 8,
    /// entry point of program
    Entry = 9,
    /// program is not ELF
    NotElf = 10,
    /// real uid
    Uid = 11,
    /// effective uid
    EUid = 12,
    /// real gid
    Gid = 13,
    /// effective gid
    EGid = 14,
    /// string identifying CPU for optimizations
    Platform = 15,
    /// Arch dependent hints at CPU capabilities.
    /// On x86_64 these are the CPUID features.
    HwCap = 16,
    /// frequency at which times() increments
    Clktck = 17,
    /// secure mode boolean
    Secure = 23,
    /// string identifying real platform, may differ from AtPlatform.
    BasePlatform = 24,
    /// address of 16 random bytes
    Random = 25,
    /// extension of AtHwcap
    HwCap2 = 26,
    /// filename of program, for example "./my_executable\0"
    ExecFn = 31,

    // ### according to Linux code: from here: x86_64
    /// The entry point to the system call function in the vDSO.
    /// Not present/needed on all architectures (e.g., absent on
    /// x86-64).
    Sysinfo = 32,
    /// The address of a page containing the virtual Dynamic
    /// Shared Object (vDSO) that the kernel creates in order to
    /// provide fast implementations of certain system calls.
    SysinfoEhdr = 33,

    // ### according to Linux code: from here: PowerPC
    /// L1 instruction cache size
    L1iCacheSize = 40,
    /// L1 instruction cache geometry
    L1iCacheGeometry = 41,
    /// L1 cache geometry
    L1dCacheSize = 42,
    /// L1 cache size
    L1dCacheGeometry = 43,
    /// L2 cache size
    L2CacheSize = 44,
    /// L2 cache geometry
    L2CacheGeometry = 45,
    /// L3 cache size
    L3CacheSize = 46,
    /// L3 cache geometry
    L3CacheGeometry = 47,

    /// Minimal stack size for signal delivery.
    MinSigStkSz = 51,
}

impl AuxVarType {
    /// Returns an array with all variants.
    #[must_use]
    pub const fn variants() -> &'static [Self] {
        &[
            Self::Null,
            Self::Ignore,
            Self::ExecFd,
            Self::Phdr,
            Self::Phent,
            Self::Phnum,
            Self::Pagesz,
            Self::Base,
            Self::Flags,
            Self::Entry,
            Self::NotElf,
            Self::Uid,
            Self::EUid,
            Self::Gid,
            Self::EGid,
            Self::Platform,
            Self::HwCap,
            Self::Clktck,
            Self::Secure,
            Self::BasePlatform,
            Self::Random,
            Self::HwCap2,
            Self::ExecFn,
            Self::Sysinfo,
            Self::SysinfoEhdr,
            Self::L1iCacheSize,
            Self::L1iCacheGeometry,
            Self::L1dCacheSize,
            Self::L1dCacheGeometry,
            Self::L2CacheSize,
            Self::L2CacheGeometry,
            Self::L3CacheSize,
            Self::L3CacheGeometry,
            Self::MinSigStkSz,
        ]
    }

    /// Returns the underlying ABI-compatible integer value.
    #[must_use]
    pub const fn val(self) -> usize {
        self as _
    }

    /// If this is true, the value of the key should be interpreted as pointer
    /// into the aux vector data area. Otherwise, the value of the key is an
    /// immediate value/integer.
    #[must_use]
    pub const fn value_in_data_area(self) -> bool {
        // this info can be found here:
        // https://elixir.bootlin.com/linux/latest/source/fs/binfmt_elf.c#L259
        match self {
            Self::Null => false,
            Self::Ignore => false,
            Self::ExecFd => false,
            Self::Phdr => false,
            Self::Phent => false,
            Self::Phnum => false,
            Self::Pagesz => false,
            Self::Base => false,
            Self::Flags => false,
            Self::Entry => false,
            Self::NotElf => false,
            Self::Uid => false,
            Self::EUid => false,
            Self::Gid => false,
            Self::EGid => false,
            // references C-str
            Self::Platform => true,
            Self::HwCap => false,
            Self::Clktck => false,
            Self::Secure => false,
            // references C-str
            Self::BasePlatform => true,
            // references random bytes
            Self::Random => true,
            Self::HwCap2 => false,
            // references C-str
            Self::ExecFn => true,
            Self::SysinfoEhdr => false,
            Self::Sysinfo => false,
            Self::L1iCacheSize => false,
            Self::L1iCacheGeometry => false,
            Self::L1dCacheSize => false,
            Self::L1dCacheGeometry => false,
            Self::L2CacheSize => false,
            Self::L2CacheGeometry => false,
            Self::L3CacheSize => false,
            Self::L3CacheGeometry => false,
            Self::MinSigStkSz => false,
        }
    }

    /// The payload of entries where this returns true represents a
    /// null-terminated C-string.
    #[must_use]
    pub fn value_is_cstr(self) -> bool {
        self.value_in_data_area()
            && [Self::Platform, Self::BasePlatform, Self::ExecFn].contains(&self)
    }

    /// The payload of some [`AuxVarType`] is stored in the aux var data area.
    /// Most of these payloads are variable-length and null-terminated. If they
    /// have a fixed size, then this function returns it.
    #[must_use]
    pub const fn data_area_val_size_hint(self) -> Option<usize> {
        match self {
            Self::Random => Some(16),
            _ => None,
        }
    }
}

impl From<AuxVarType> for usize {
    fn from(value: AuxVarType) -> Self {
        value.val()
    }
}

impl TryFrom<usize> for AuxVarType {
    type Error = ParseAuxVarTypeError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        for variant in Self::variants() {
            if variant.val() == value {
                return Ok(*variant);
            }
        }
        Err(ParseAuxVarTypeError(value))
    }
}

impl PartialOrd for AuxVarType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AuxVarType {
    fn cmp(&self, other: &Self) -> Ordering {
        if matches!(self, Self::Null) && !matches!(other, Self::Null) {
            Ordering::Greater
        } else {
            self.val().cmp(&other.val())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_variants_are_sorted() {
        let mut variants = AuxVarType::variants().to_vec();
        variants.sort();
        assert_eq!(AuxVarType::variants(), variants.as_slice());
    }

    /// Tests that the ATNull entry always comes last in an ordered collection.
    /// This enables us to easily write all AT-VARs at once but keep the
    /// terminating null entry at the end.
    #[test]
    fn test_aux_var_key_order() {
        let mut set = BTreeSet::new();
        set.insert(AuxVarType::ExecFn);
        set.insert(AuxVarType::Platform);
        set.insert(AuxVarType::Null);
        set.insert(AuxVarType::Clktck);
        set.insert(AuxVarType::ExecFn);
        assert_eq!(set.last(), Some(&AuxVarType::Null));
    }
}
