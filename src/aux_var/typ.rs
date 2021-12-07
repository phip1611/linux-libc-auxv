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
use core::cmp::Ordering;
use enum_iterator::IntoEnumIterator;

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
#[derive(Copy, Clone, Debug, PartialEq, Eq, IntoEnumIterator)]
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
    L1iCacheSize = 40,
    L1iCacheGeometry = 41,
    L1dCacheSize = 42,
    L1dCacheGeometry = 43,
    L2CacheSize = 44,
    L2CacheGeometry = 45,
    L3CacheSize = 46,
    L3CacheGeometry = 47,

    /// Minimal stack size for signal delivery.
    MinSigStkSz = 51,
}

impl AuxVarType {
    pub const fn val(self) -> usize {
        self as _
    }

    /// If this is true, the value of the key should be interpreted as pointer into
    /// the aux vector data area. Otherwise, the value of the key is an immediate value/integer.
    // TODO move to AuxVar?!
    pub const fn value_in_data_area(self) -> bool {
        // this info can be found here:
        // https://elixir.bootlin.com/linux/latest/source/fs/binfmt_elf.c#L259
        match self {
            AuxVarType::Null => false,
            AuxVarType::Ignore => false,
            AuxVarType::ExecFd => false,
            AuxVarType::Phdr => false,
            AuxVarType::Phent => false,
            AuxVarType::Phnum => false,
            AuxVarType::Pagesz => false,
            AuxVarType::Base => false,
            AuxVarType::Flags => false,
            AuxVarType::Entry => false,
            AuxVarType::NotElf => false,
            AuxVarType::Uid => false,
            AuxVarType::EUid => false,
            AuxVarType::Gid => false,
            AuxVarType::EGid => false,
            // references C-str
            AuxVarType::Platform => true,
            AuxVarType::HwCap => false,
            AuxVarType::Clktck => false,
            AuxVarType::Secure => false,
            // references C-str
            AuxVarType::BasePlatform => true,
            // references random bytes
            AuxVarType::Random => true,
            AuxVarType::HwCap2 => false,
            // references C-str
            AuxVarType::ExecFn => true,
            AuxVarType::SysinfoEhdr => false,
            AuxVarType::Sysinfo => false,
            AuxVarType::L1iCacheSize => false,
            AuxVarType::L1iCacheGeometry => false,
            AuxVarType::L1dCacheSize => false,
            AuxVarType::L1dCacheGeometry => false,
            AuxVarType::L2CacheSize => false,
            AuxVarType::L2CacheGeometry => false,
            AuxVarType::L3CacheSize => false,
            AuxVarType::L3CacheGeometry => false,
            AuxVarType::MinSigStkSz => false,
        }
    }

    /// Most of the auxiliary vector entries where [`Self::value_is_cstr`] is true,
    /// represent a null-terminated C-string.
    pub fn value_is_cstr(self) -> bool {
        self.value_in_data_area()
            && [Self::Platform, Self::BasePlatform, Self::ExecFn].contains(&self)
    }

    /// The payload of some [`AuxVarType`] is stored in the aux var data area. Some of these
    /// data is null-terminated. Some has a fixed size. This helps to find out if there
    /// is a fixed size.
    pub const fn data_area_val_size_hint(self) -> Option<usize> {
        match self {
            AuxVarType::Random => Some(16),
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

impl PartialOrd for AuxVarType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if matches!(self, Self::Null) && !matches!(other, Self::Null) {
            Some(Ordering::Greater)
        } else {
            self.val().partial_cmp(&other.val())
        }
    }
}

impl Ord for AuxVarType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Tests that the ATNull entry always comes last in an ordered collection. This enables
    /// us to easily write all AT-VARs at once but keep the terminating null entry at the end.
    #[test]
    fn test_aux_var_key_order() {
        let mut set = BTreeSet::new();
        set.insert(AuxVarType::ExecFn);
        set.insert(AuxVarType::Platform);
        set.insert(AuxVarType::Null);
        set.insert(AuxVarType::Clktck);
        set.insert(AuxVarType::ExecFn);
        assert_eq!(set.into_iter().last().unwrap(), AuxVarType::Null);
    }
}
