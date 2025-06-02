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

mod serialized;
mod typ;

pub use serialized::*;
pub use typ::*;

use crate::util::count_bytes_until_null;
use core::cmp::Ordering;
use core::ffi::CStr;
use core::fmt::{Debug, Display, Formatter};
#[cfg(feature = "alloc")]
use {alloc::borrow::ToOwned, alloc::ffi::CString, alloc::string::String, alloc::string::ToString};
bitflags::bitflags! {
    /// Flags for the auxiliary vector. See <https://elixir.bootlin.com/linux/v5.15.5/source/include/uapi/linux/binfmts.h#L23>.
    #[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
    pub struct AuxVarFlags: usize {
        /// Opposite of [`Self::PRESERVE_ARGV0`].
        const NOT_PRESERVE_ARGV0 = 0;
        /// Preserve argv0 for the interpreter.
        const PRESERVE_ARGV0 = 1;
    }
}

/// Possible string payload variants of an [`AuxVar`].
///
/// Due to the diverse variants, is not guaranteed that
/// - a terminating NUL byte is present, and
/// - that no interim NUL bytes are present.
///
/// When constructing these variants, adding a terminating NUL byte is not
/// necessary. Interim NUL bytes are prohibited.
///
/// This type can be easily construct using `::from()` respectively `.into()`.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AuxVarString<'a> {
    #[cfg(feature = "alloc")]
    String(String),
    #[cfg(feature = "alloc")]
    CString(CString),
    Str(&'a str),
    CStr(&'a CStr),
}

impl<'a> AuxVarString<'a> {
    /// Returns the bytes of the underlying type.
    ///
    /// Due to the diverse variants, is not guaranteed that
    /// - a terminating NUL byte is present, and
    /// - that no interim NUL bytes are present.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            #[cfg(feature = "alloc")]
            AuxVarString::String(str) => str.as_bytes(),
            #[cfg(feature = "alloc")]
            AuxVarString::CString(cstr) => cstr.to_bytes_with_nul(),
            AuxVarString::Str(str) => str.as_bytes(),
            AuxVarString::CStr(cstr) => cstr.to_bytes_with_nul(),
        }
    }

    /// Returns the number of bytes until the first NUL, excluding the NUL.
    pub fn count_bytes(&self) -> usize {
        count_bytes_until_null(self.as_bytes()).unwrap_or(self.as_bytes().len())
    }

    /// Upgrades the underlying reference to an owned variant.
    ///
    /// This is a no-op if the variant already owns the value.
    #[cfg(feature = "alloc")]
    pub fn upgrade_to_owned(self) -> Self {
        match self {
            AuxVarString::Str(str) => Self::String(str.to_owned()),
            AuxVarString::CStr(cstr) => Self::CString(cstr.to_owned()),
            o => o,
        }
    }

    /// Transforms the inner value into a owned Rust [`String`].
    #[cfg(feature = "alloc")]
    pub fn into_string(self) -> String {
        match self {
            AuxVarString::String(str) => str,
            AuxVarString::CString(str) => str.to_str().unwrap().to_string(),
            AuxVarString::Str(str) => str.to_string(),
            AuxVarString::CStr(str) => str.to_str().unwrap().to_string(),
        }
    }
}

impl Display for AuxVarString<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "alloc")]
            AuxVarString::String(v) => Display::fmt(v, f),
            #[cfg(feature = "alloc")]
            AuxVarString::CString(v) => Debug::fmt(&v.to_str(), f),
            AuxVarString::Str(v) => Display::fmt(v, f),
            AuxVarString::CStr(v) => Debug::fmt(&v.to_str(), f),
        }
    }
}

impl<'a> From<&'a str> for AuxVarString<'a> {
    fn from(str: &'a str) -> Self {
        Self::Str(str)
    }
}

impl<'a> From<&'a CStr> for AuxVarString<'a> {
    fn from(str: &'a CStr) -> Self {
        Self::CStr(str)
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<String> for AuxVarString<'a> {
    fn from(str: String) -> Self {
        Self::String(str)
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<CString> for AuxVarString<'a> {
    fn from(str: CString) -> Self {
        Self::CString(str)
    }
}

/// High-level version of an auxiliary vector (`auxv`) entry. Also called
/// _Auxiliary Variable_ or _AT Variable_.
///
/// The data/payload is either an immediate value embedded into the enum variant
/// or a pointer into the `auxv` data area. The enum variant's payload does not
/// necessarily correspond to the ABI.
///
/// ## More Info
/// * <https://elixir.bootlin.com/linux/latest/source/include/uapi/linux/auxvec.h>
/// * <https://elixir.bootlin.com/linux/latest/source/fs/binfmt_elf.c#L259>
/// * <https://man7.org/linux/man-pages/man3/getauxval.3.html>
/// * <https://lwn.net/Articles/631631/>
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AuxVar<'a> {
    /// Entry with payload for type [`AuxVarType::Null`].
    Null,
    /// Entry with payload for type [`AuxVarType::Ignore`].
    Ignore,
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
    Platform(AuxVarString<'a>),
    /// Entry with payload for type [`AuxVarType::HwCap`].
    HwCap(usize),
    /// Entry with payload for type [`AuxVarType::Clktck`].
    Clktck(usize),
    /// Entry with payload for type [`AuxVarType::Secure`].
    Secure(bool),
    /// Entry with payload for type [`AuxVarType::BasePlatform`].
    BasePlatform(AuxVarString<'a>),
    /// Entry with payload for type [`AuxVarType::Random`].
    Random(/* ABI: raw ptr to data area */ [u8; 16]),
    /// Entry with payload for type [`AuxVarType::HwCap2`].
    HwCap2(usize),
    /// Entry with payload for type [`AuxVarType::ExecFn`].
    ExecFn(AuxVarString<'a>),
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
    /// Creates a [`CStr`] reference from a underlying buffer.
    ///
    /// The string starts at the beginning and ends at the first NUL byte.
    ///
    /// # Arguments
    /// - `buffer`: Buffer containing the whole structure, also the data
    ///   that some auxiliary variables point to.
    ///
    fn _from_raw_to_cstr(ptr: usize, buffer: &[u8]) -> &CStr {
        let begin_index = ptr - buffer.as_ptr() as usize;

        let bytes = &buffer[begin_index..];
        CStr::from_bytes_until_nul(bytes).unwrap()
    }

    /// Creates the corresponding enum variant from a [`AuxVarRaw`].
    ///
    /// # Arguments
    /// - `serialized`: Raw value read from memory
    /// - `buffer`: Buffer containing the whole structure, also the data
    ///   that some auxiliary variables point to.
    ///
    /// # Safety
    /// This function creates undefined behavior or might even crash if the
    /// value is an invalid pointer or a pointer pointing to invalid memory.
    pub(crate) unsafe fn from_raw(serialized: &AuxVarRaw, buffer: &'a [u8]) -> Self {
        let key = serialized.key().unwrap();

        match key {
            AuxVarType::Platform => {
                Self::Platform(Self::_from_raw_to_cstr(serialized.value(), buffer).into())
            }
            AuxVarType::BasePlatform => {
                Self::BasePlatform(Self::_from_raw_to_cstr(serialized.value(), buffer).into())
            }
            AuxVarType::ExecFn => {
                Self::ExecFn(Self::_from_raw_to_cstr(serialized.value(), buffer).into())
            }
            AuxVarType::Random => {
                let begin_index = serialized.value() - buffer.as_ptr() as usize;
                let end_index = begin_index + 16 /* 16 bytes of randomness */;
                assert!(end_index < buffer.len());

                let mut bytes = [0; 16];
                bytes.copy_from_slice(&buffer[begin_index..end_index]);

                Self::Random(bytes)
            }
            AuxVarType::Null => Self::Null,
            AuxVarType::Ignore => Self::Ignore,
            AuxVarType::ExecFd => Self::ExecFd(serialized.value()),
            AuxVarType::Phdr => Self::Phdr(serialized.value() as *const u8),
            AuxVarType::Phent => Self::Phent(serialized.value()),
            AuxVarType::Phnum => Self::Phnum(serialized.value()),
            AuxVarType::Pagesz => Self::Pagesz(serialized.value()),
            AuxVarType::Base => Self::Base(serialized.value() as *const u8),
            AuxVarType::Flags => Self::Flags(AuxVarFlags::from_bits_truncate(serialized.value())),
            AuxVarType::Entry => Self::Entry(serialized.value() as *const u8),
            AuxVarType::NotElf => Self::NotElf(serialized.value() == 1),
            AuxVarType::Uid => Self::Uid(serialized.value()),
            AuxVarType::EUid => Self::EUid(serialized.value()),
            AuxVarType::Gid => Self::Gid(serialized.value()),
            AuxVarType::EGid => Self::EGid(serialized.value()),
            // AuxVarType::Platform =>
            AuxVarType::HwCap => Self::HwCap(serialized.value()),
            AuxVarType::Clktck => Self::Clktck(serialized.value()),
            AuxVarType::Secure => Self::Secure(serialized.value() == 1),
            // AuxVarType::BasePlatform =>
            // AuxVarType::Random =>
            AuxVarType::HwCap2 => Self::HwCap2(serialized.value()),
            //AuxVarType::ExecFn => Self::ExecFn(serialized.value()),
            AuxVarType::Sysinfo => Self::Sysinfo(serialized.value() as *const u8),
            AuxVarType::SysinfoEhdr => Self::SysinfoEhdr(serialized.value() as *const u8),
            AuxVarType::L1iCacheSize => Self::L1iCacheSize(serialized.value()),
            AuxVarType::L1iCacheGeometry => Self::L1iCacheGeometry(serialized.value()),
            AuxVarType::L1dCacheSize => Self::L1dCacheSize(serialized.value()),
            AuxVarType::L1dCacheGeometry => Self::L1dCacheGeometry(serialized.value()),
            AuxVarType::L2CacheSize => Self::L2CacheSize(serialized.value()),
            AuxVarType::L2CacheGeometry => Self::L2CacheGeometry(serialized.value()),
            AuxVarType::L3CacheSize => Self::L3CacheSize(serialized.value()),
            AuxVarType::L3CacheGeometry => Self::L3CacheGeometry(serialized.value()),
            AuxVarType::MinSigStkSz => Self::MinSigStkSz(serialized.value()),
        }
    }

    /// Returns the [`AuxVarType`] this aux var corresponds to.
    #[must_use]
    pub const fn key(&self) -> AuxVarType {
        match self {
            AuxVar::Null => AuxVarType::Null,
            AuxVar::Ignore => AuxVarType::Ignore,
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

    /// Transforms any inner value into it's corresponding serialized usize
    /// value.
    ///
    /// This only works for variants that do not reference data exceeding the
    /// size of an `usize`.
    #[must_use]
    pub fn value_raw(&self) -> usize {
        match self {
            AuxVar::Platform(_)
            | AuxVar::BasePlatform(_)
            | AuxVar::Random(_)
            | AuxVar::ExecFn(_) => todo!("return Result instead"),
            AuxVar::Null => 0,
            AuxVar::Ignore => 0,
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
            // AuxVar::Platform(val) => val.as_ptr() as _,
            AuxVar::HwCap(val) => *val,
            AuxVar::Clktck(val) => *val,
            AuxVar::Secure(val) => {
                if *val {
                    1
                } else {
                    0
                }
            }
            // AuxVar::BasePlatform(val) => val.as_ptr() as _,
            // AuxVar::Random(val) => val.as_ptr() as _,
            AuxVar::HwCap2(val) => *val,
            // AuxVar::ExecFn(val) => val.as_ptr() as _,
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

    /// Returns a value if the corresponding entry corresponds to a basic
    /// value/integer, and not a pointer, flags, or a boolean.
    #[must_use]
    pub const fn value_integer(&self) -> Option<usize> {
        match self {
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

    /// Returns the [`AuxVarFlags`] if the corresponding entry is of type
    /// [`AuxVarType::Flags`].
    #[must_use]
    pub const fn value_flags(&self) -> Option<AuxVarFlags> {
        match self {
            AuxVar::Flags(flags) => Some(*flags),
            _ => None,
        }
    }

    /// Returns a value if the corresponding entry corresponds to a boolean,
    /// and not a pointer, flags, or a basic value/integer.
    #[must_use]
    pub const fn value_boolean(&self) -> Option<bool> {
        match self {
            AuxVar::NotElf(val) => Some(*val),
            AuxVar::Secure(val) => Some(*val),
            _ => None,
        }
    }

    /// Returns a value if the corresponding entry corresponds to a pointer,
    /// and not a boolean, flags, or a basic value/integer.
    ///
    /// This only affects entries that point to memory outside the stack layout,
    /// i.e., the aux vector data area.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub const fn value_payload_str(&'a self) -> Option<&'a AuxVarString<'a>> {
        match self {
            AuxVar::Platform(val) => Some(val),
            AuxVar::BasePlatform(val) => Some(val),
            AuxVar::ExecFn(val) => Some(val),
            _ => None,
        }
    }
}

impl<'a> PartialOrd for AuxVar<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for AuxVar<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
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
        set.insert(AuxVar::ExecFn(c"./executable".into()));
        set.insert(AuxVar::Platform(c"x86_64".into()));
        set.insert(AuxVar::Null);
        set.insert(AuxVar::Clktck(0x1337));
        set.insert(AuxVar::ExecFn(c"./executable".into()));
        assert_eq!(set.iter().last().unwrap().key(), AuxVarType::Null);
    }
}
