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

mod serialized;
mod typ;

pub(crate) use serialized::*;
pub use typ::*;

use crate::cstr_util::{c_str_len_ptr, c_str_null_terminated};
use core::cmp::Ordering;
use core::fmt::Debug;
use core::slice;

bitflags::bitflags! {
    /// Flags for the auxiliary vector. See <https://elixir.bootlin.com/linux/v5.15.5/source/include/uapi/linux/binfmts.h#L23>.
    pub struct AuxVarFlags: usize {
        /// Opposite of [`Self::PRESERVE_ARGV0`].
        const NOT_PRESERVE_ARGV0 = 0;
        /// Preserve argv0 for the interpreter.
        const PRESERVE_ARGV0 = 1;
    }
}

/// High-level version of the serialized form of an auxiliary vector entry. It is used to construct
/// the auxiliary vector in [`crate::InitialLinuxLibcStackLayoutBuilder`] and returned when
/// a data structure is parsed with [`crate::InitialLinuxLibcStackLayout`].
#[derive(Debug)]
pub enum AuxVar<'a> {
    /// Entry with payload for type [`AuxVarType::Null`].
    Null,
    /// Entry with payload for type [`AuxVarType::Ignore`].
    Ignore(usize),
    /// Entry with payload for type [`AuxVarType::ExecFd`].
    ExecFd(usize),
    /// Entry with payload for type [`AuxVarType::Phdr`].
    Phdr(*const u8),
    /// Entry with payload for type [`AuxVarType::Phent`].
    Phent(usize),
    /// Entry with payload for type [`AuxVarType::Phnum`].
    Phnum(usize),
    /// Entry with payload for type [`AuxVarType::Pagesz`].
    Pagesz(usize),
    /// Entry with payload for type [`AuxVarType::Base`].
    Base(*const u8),
    /// Entry with payload for type [`AuxVarType::Flags`].
    Flags(AuxVarFlags),
    /// Entry with payload for type [`AuxVarType::Entry`].
    Entry(*const u8),
    /// Entry with payload for type [`AuxVarType::NotElf`].
    NotElf(bool),
    /// Entry with payload for type [`AuxVarType::Uid`].
    Uid(usize),
    /// Entry with payload for type [`AuxVarType::EUid`].
    EUid(usize),
    /// Entry with payload for type [`AuxVarType::Gid`].
    Gid(usize),
    /// Entry with payload for type [`AuxVarType::EGid`].
    EGid(usize),
    /// Entry with payload for type [`AuxVarType::Platform`].
    Platform(&'a str),
    /// Entry with payload for type [`AuxVarType::HwCap`].
    HwCap(usize),
    /// Entry with payload for type [`AuxVarType::Clktck`].
    Clktck(usize),
    /// Entry with payload for type [`AuxVarType::Secure`].
    Secure(bool),
    /// Entry with payload for type [`AuxVarType::BasePlatform`].
    BasePlatform(&'a str),
    /// Entry with payload for type [`AuxVarType::Random`].
    /// This data owns the bytes rather than referencing to it,
    /// because the operation is cheap anyway but it also simplified
    /// parsing. Otherwise, I had to create a `&'a [u8; 16]` from a
    /// `&'a [u8]`, which is hard without hacky tricks.
    Random([u8; 16]),
    /// Entry with payload for type [`AuxVarType::HwCap2`].
    HwCap2(usize),
    /// Entry with payload for type [`AuxVarType::ExecFn`].
    ExecFn(&'a str),
    /// Entry with payload for type [`AuxVarType::Sysinfo`].
    Sysinfo(*const u8),
    /// Entry with payload for type [`AuxVarType::SysinfoEhdr`].
    SysinfoEhdr(*const u8),
    /// Entry with payload for type [`AuxVarType::L1iCacheSize`].
    L1iCacheSize(usize),
    /// Entry with payload for type [`AuxVarType::L1iCacheGeometry`].
    L1iCacheGeometry(usize),
    /// Entry with payload for type [`AuxVarType::L1dCacheSize`].
    L1dCacheSize(usize),
    /// Entry with payload for type [`AuxVarType::L1dCacheGeometry`].
    L1dCacheGeometry(usize),
    /// Entry with payload for type [`AuxVarType::L2CacheSize`].
    L2CacheSize(usize),
    /// Entry with payload for type [`AuxVarType::L2CacheGeometry`].
    L2CacheGeometry(usize),
    /// Entry with payload for type [`AuxVarType::L3CacheSize`].
    L3CacheSize(usize),
    /// Entry with payload for type [`AuxVarType::L3CacheGeometry`].
    L3CacheGeometry(usize),
    /// Entry with payload for type [`AuxVarType::MinSigStkSz`].
    MinSigStkSz(usize),
}

impl<'a> AuxVar<'a> {
    /// Creates the high-level type [`AuxVar`] from the serialized version of an
    /// AT variable/aux vec entry.
    ///
    /// # Safety
    /// This function creates undefined behaviour or might even crash, if the AT value
    /// reference data in the aux vec data area, where the pointer is either invalid or
    /// if the C-strings is not null-terminated.
    pub(crate) unsafe fn from_serialized(serialized: &AuxVarSerialized) -> Self {
        if serialized.key().value_in_data_area() {
            let data_ptr = serialized.val() as *const u8;
            let len = serialized
                .key()
                .data_area_val_size_hint()
                .unwrap_or_else(|| c_str_len_ptr(data_ptr));
            let slice = unsafe { slice::from_raw_parts(data_ptr, len) };
            if serialized.key().value_is_cstr() {
                let cstr = core::str::from_utf8(slice).expect(
                    "must be valid c string. Either invalid memory or not null-terminated!",
                );
                match serialized.key() {
                    AuxVarType::Platform => Self::Platform(cstr),
                    AuxVarType::BasePlatform => Self::BasePlatform(cstr),
                    AuxVarType::ExecFn => Self::ExecFn(cstr),
                    _ => panic!("invalid variant"),
                }
            } else {
                match serialized.key() {
                    AuxVarType::Random => {
                        let mut random_bytes = [0; 16];
                        // memcpy into new, fixed length slice
                        random_bytes.copy_from_slice(slice);
                        Self::Random(random_bytes)
                    }
                    _ => panic!("invalid variant"),
                }
            }
        } else {
            match serialized.key() {
                AuxVarType::Null => Self::Null,
                AuxVarType::Ignore => Self::Ignore(serialized.val()),
                AuxVarType::ExecFd => Self::ExecFd(serialized.val()),
                AuxVarType::Phdr => Self::Phdr(serialized.val() as _),
                AuxVarType::Phent => Self::Phent(serialized.val()),
                AuxVarType::Phnum => Self::Phnum(serialized.val()),
                AuxVarType::Pagesz => Self::Pagesz(serialized.val()),
                AuxVarType::Base => Self::Base(serialized.val() as _),
                AuxVarType::Flags => Self::Flags(AuxVarFlags::from_bits(serialized.val()).unwrap()),
                AuxVarType::Entry => Self::Entry(serialized.val() as _),
                AuxVarType::NotElf => Self::NotElf(serialized.val() != 0),
                AuxVarType::Uid => Self::Uid(serialized.val()),
                AuxVarType::EUid => Self::EUid(serialized.val()),
                AuxVarType::Gid => Self::Gid(serialized.val()),
                AuxVarType::EGid => Self::EGid(serialized.val()),
                AuxVarType::HwCap => Self::HwCap(serialized.val()),
                AuxVarType::Clktck => Self::Clktck(serialized.val()),
                AuxVarType::Secure => Self::Secure(serialized.val() != 0),
                AuxVarType::HwCap2 => Self::HwCap2(serialized.val()),
                AuxVarType::Sysinfo => Self::Sysinfo(serialized.val() as _),
                AuxVarType::SysinfoEhdr => Self::SysinfoEhdr(serialized.val() as _),
                AuxVarType::L1iCacheSize => Self::L1iCacheSize(serialized.val()),
                AuxVarType::L1iCacheGeometry => Self::L1iCacheGeometry(serialized.val()),
                AuxVarType::L1dCacheSize => Self::L1dCacheSize(serialized.val()),
                AuxVarType::L1dCacheGeometry => Self::L1dCacheGeometry(serialized.val()),
                AuxVarType::L2CacheSize => Self::L2CacheSize(serialized.val()),
                AuxVarType::L2CacheGeometry => Self::L2CacheGeometry(serialized.val()),
                AuxVarType::L3CacheSize => Self::L3CacheSize(serialized.val()),
                AuxVarType::L3CacheGeometry => Self::L3CacheGeometry(serialized.val()),
                AuxVarType::MinSigStkSz => Self::MinSigStkSz(serialized.val()),
                _ => panic!("invalid variant"),
            }
        }
    }

    /// Returns the [`AuxVarType`] this aux var corresponds to.
    pub const fn key(&self) -> AuxVarType {
        match self {
            AuxVar::Null => AuxVarType::Null,
            AuxVar::Ignore(_) => AuxVarType::Ignore,
            AuxVar::ExecFd(_) => AuxVarType::ExecFd,
            AuxVar::Phdr(_) => AuxVarType::Phdr,
            AuxVar::Phent(_) => AuxVarType::Phent,
            AuxVar::Phnum(_) => AuxVarType::Phnum,
            AuxVar::Pagesz(_) => AuxVarType::Pagesz,
            AuxVar::Base(_) => AuxVarType::Base,
            AuxVar::Flags(_) => AuxVarType::Flags,
            AuxVar::Entry(_) => AuxVarType::Entry,
            AuxVar::NotElf(_) => AuxVarType::NotElf,
            AuxVar::Uid(_) => AuxVarType::Uid,
            AuxVar::EUid(_) => AuxVarType::EUid,
            AuxVar::Gid(_) => AuxVarType::Gid,
            AuxVar::EGid(_) => AuxVarType::EGid,
            AuxVar::Platform(_) => AuxVarType::Platform,
            AuxVar::HwCap(_) => AuxVarType::HwCap,
            AuxVar::Clktck(_) => AuxVarType::Clktck,
            AuxVar::Secure(_) => AuxVarType::Secure,
            AuxVar::BasePlatform(_) => AuxVarType::BasePlatform,
            AuxVar::Random(_) => AuxVarType::Random,
            AuxVar::HwCap2(_) => AuxVarType::HwCap2,
            AuxVar::ExecFn(_) => AuxVarType::ExecFn,
            AuxVar::Sysinfo(_) => AuxVarType::Sysinfo,
            AuxVar::SysinfoEhdr(_) => AuxVarType::SysinfoEhdr,
            AuxVar::L1iCacheSize(_) => AuxVarType::L1iCacheSize,
            AuxVar::L1iCacheGeometry(_) => AuxVarType::L1iCacheGeometry,
            AuxVar::L1dCacheSize(_) => AuxVarType::L1dCacheSize,
            AuxVar::L1dCacheGeometry(_) => AuxVarType::L1dCacheGeometry,
            AuxVar::L2CacheSize(_) => AuxVarType::L2CacheSize,
            AuxVar::L2CacheGeometry(_) => AuxVarType::L2CacheGeometry,
            AuxVar::L3CacheSize(_) => AuxVarType::L3CacheSize,
            AuxVar::L3CacheGeometry(_) => AuxVarType::L3CacheGeometry,
            AuxVar::MinSigStkSz(_) => AuxVarType::MinSigStkSz,
        }
    }

    /// Transforms any inner value into it's corresponding usize value.
    /// This is similar to the data that is serialized in the data structure on the stack,
    /// i.e. the value of the auxiliary vector entry.
    pub fn value_raw(&self) -> usize {
        match self {
            AuxVar::Null => 0,
            AuxVar::Ignore(val) => *val,
            AuxVar::ExecFd(val) => *val,
            AuxVar::Phdr(val) => *val as _,
            AuxVar::Phent(val) => *val,
            AuxVar::Phnum(val) => *val,
            AuxVar::Pagesz(val) => *val,
            AuxVar::Base(val) => *val as _,
            AuxVar::Flags(val) => val.bits(),
            AuxVar::Entry(val) => *val as _,
            AuxVar::NotElf(val) => {
                if *val {
                    1
                } else {
                    0
                }
            }
            AuxVar::Uid(val) => *val,
            AuxVar::EUid(val) => *val,
            AuxVar::Gid(val) => *val,
            AuxVar::EGid(val) => *val,
            AuxVar::Platform(val) => val.as_ptr() as _,
            AuxVar::HwCap(val) => *val,
            AuxVar::Clktck(val) => *val,
            AuxVar::Secure(val) => {
                if *val {
                    1
                } else {
                    0
                }
            }
            AuxVar::BasePlatform(val) => val.as_ptr() as _,
            AuxVar::Random(val) => val.as_ptr() as _,
            AuxVar::HwCap2(val) => *val,
            AuxVar::ExecFn(val) => val.as_ptr() as _,
            AuxVar::Sysinfo(val) => *val as _,
            AuxVar::SysinfoEhdr(val) => *val as _,
            AuxVar::L1iCacheSize(val) => *val,
            AuxVar::L1iCacheGeometry(val) => *val,
            AuxVar::L1dCacheSize(val) => *val,
            AuxVar::L1dCacheGeometry(val) => *val,
            AuxVar::L2CacheSize(val) => *val,
            AuxVar::L2CacheGeometry(val) => *val,
            AuxVar::L3CacheSize(val) => *val,
            AuxVar::L3CacheGeometry(val) => *val,
            AuxVar::MinSigStkSz(val) => *val,
        }
    }

    /// Returns a value, if the corresponding auxiliary vector entry corresponds to a basic
    /// value/integer, and not a pointer, flags, or a boolean.
    pub const fn value_integer(&self) -> Option<usize> {
        match self {
            AuxVar::Ignore(val) => Some(*val),
            AuxVar::ExecFd(val) => Some(*val),
            AuxVar::Phent(val) => Some(*val),
            AuxVar::Phnum(val) => Some(*val),
            AuxVar::Pagesz(val) => Some(*val),
            AuxVar::Uid(val) => Some(*val),
            AuxVar::EUid(val) => Some(*val),
            AuxVar::Gid(val) => Some(*val),
            AuxVar::EGid(val) => Some(*val),
            AuxVar::HwCap(val) => Some(*val),
            AuxVar::Clktck(val) => Some(*val),
            AuxVar::HwCap2(val) => Some(*val),
            AuxVar::L1iCacheSize(val) => Some(*val),
            AuxVar::L1iCacheGeometry(val) => Some(*val),
            AuxVar::L1dCacheSize(val) => Some(*val),
            AuxVar::L1dCacheGeometry(val) => Some(*val),
            AuxVar::L2CacheSize(val) => Some(*val),
            AuxVar::L2CacheGeometry(val) => Some(*val),
            AuxVar::L3CacheSize(val) => Some(*val),
            AuxVar::L3CacheGeometry(val) => Some(*val),
            AuxVar::MinSigStkSz(val) => Some(*val),
            _ => None,
        }
    }

    /// Returns a value, if the corresponding auxiliary vector entry is of type [`AuxVarType::Flags`].
    pub const fn value_flags(&self) -> Option<AuxVarFlags> {
        match self {
            AuxVar::Flags(flags) => Some(*flags),
            _ => None,
        }
    }

    /// Returns a value, if the corresponding auxiliary vector entry corresponds to a
    /// boolean, and not a pointer, flags, or a basic value/integer.
    pub const fn value_boolean(&self) -> Option<bool> {
        match self {
            AuxVar::NotElf(val) => Some(*val),
            AuxVar::Secure(val) => Some(*val),
            _ => None,
        }
    }

    /// Returns a value, if the corresponding auxiliary vector entry corresponds to a
    /// pointer, and not a boolean, flags, or a basic value/integer. This only affects
    /// entries, that point to memory outside of the initial stack layout, i.e. the aux
    /// vector data area.
    pub const fn value_ptr(&self) -> Option<*const u8> {
        match self {
            AuxVar::Phdr(val) => Some(*val),
            AuxVar::Base(val) => Some(*val),
            AuxVar::Entry(val) => Some(*val),
            AuxVar::Sysinfo(val) => Some(*val),
            AuxVar::SysinfoEhdr(val) => Some(*val),
            _ => None,
        }
    }

    /// Returns a value, if the corresponding auxiliary vector entry references data in the
    /// auxiliary vector data area of the data structure.
    /// This returns only something for [`AuxVarType::Random`].
    ///
    /// This function is safe, because the creation during parsing already guarantee memory
    /// safety (the addresses are accessed).
    pub fn value_payload_bytes(&'a self) -> Option<&'a [u8]> {
        match self {
            AuxVar::Random(bytes) => Some(&bytes[..]),
            _ => None,
        }
    }

    /// Returns a value, if the corresponding auxiliary vector entry references data in the
    /// auxiliary vector data area of the data structure.
    /// This returns only something for [`AuxVarType::Random`].
    ///
    /// This function is safe, because the creation during parsing already guarantee memory
    /// safety (the addresses are accessed).
    pub const fn value_payload_cstr(&'a self) -> Option<&'a str> {
        match self {
            AuxVar::Platform(val) => Some(*val),
            AuxVar::BasePlatform(val) => Some(*val),
            AuxVar::ExecFn(val) => Some(*val),
            _ => None,
        }
    }

    // #########################
    // helper methods to validate the object in the builder

    /// Returns the total number of bytes that needs to be written into the aux vec
    /// data area. This includes the null byte for c strings.
    /// Function is used as helper in tests.
    pub(crate) fn data_area_serialize_byte_count(&self) -> usize {
        let mut bytes = 0;
        bytes += self.value_payload_bytes().map(|x| x.len()).unwrap_or(0);
        bytes += self.value_payload_cstr().map(|x| x.len()).unwrap_or(0);
        if self.value_payload_cstr().is_some()
            && !c_str_null_terminated(self.value_payload_cstr().unwrap().as_bytes())
        {
            bytes + 1
        } else {
            bytes
        }
    }
}

impl<'a> PartialEq for AuxVar<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl<'a> PartialOrd for AuxVar<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key().partial_cmp(&other.key())
    }
}

impl<'a> Eq for AuxVar<'a> {}

impl<'a> Ord for AuxVar<'a> {
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
    fn test_aux_var_order() {
        let mut set = BTreeSet::new();
        set.insert(AuxVar::ExecFn("./executable"));
        set.insert(AuxVar::Platform("x86_64"));
        set.insert(AuxVar::Null);
        set.insert(AuxVar::Clktck(0x1337));
        set.insert(AuxVar::ExecFn("./executable"));
        assert_eq!(set.iter().last().unwrap().key(), AuxVarType::Null);
    }

    #[test]
    fn test_data_area_serialize_byte_count() {
        assert_eq!(
            AuxVar::ExecFn("./executable").data_area_serialize_byte_count(),
            13
        );
        assert_eq!(
            AuxVar::ExecFn("./executable\0").data_area_serialize_byte_count(),
            13
        );
        assert_eq!(
            AuxVar::Platform("x86_64").data_area_serialize_byte_count(),
            7
        );
        assert_eq!(
            AuxVar::Platform("x86_64\0").data_area_serialize_byte_count(),
            7
        );
        assert_eq!(AuxVar::Random([0; 16]).data_area_serialize_byte_count(), 16);
    }
}
