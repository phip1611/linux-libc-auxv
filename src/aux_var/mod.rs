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

use crate::c_str_len;
use core::cmp::Ordering;
use core::fmt::{Debug, Formatter};
pub(crate) use serialized::*;
pub use typ::*;

/// High-level version of the serialized form of an auxiliary vector entry. It is used to construct
/// the auxiliary vector in [`crate::InitialLinuxLibcStackLayoutBuilder`] and when parsing an
/// auxiliary vector with [`crate::InitialLinuxLibcStackLayout`].
#[derive(Debug)]
pub struct AuxVar<'a> {
    key: AuxVarType,
    data: AuxVarData<'a>,
}

#[allow(clippy::missing_const_for_fn)]
impl<'a> AuxVar<'a> {
    /// Generic constructor. It is recommended to use the specific constructors for
    /// better validation and type safety.
    pub fn new_generic(key: AuxVarType, data: AuxVarData<'a>) -> Self {
        Self { key, data }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtNull`].
    pub fn new_at_null() -> Self {
        Self {
            key: AuxVarType::AtNull,
            data: AuxVarData::Value(0),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtIgnore`].
    pub fn new_at_ignore(val: usize) -> Self {
        Self {
            key: AuxVarType::AtIgnore,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtExecFd`].
    pub fn new_at_exec_fd(val: usize) -> Self {
        Self {
            key: AuxVarType::AtExecFd,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtPhdr`].
    pub fn new_at_phdr(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPhdr,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtPhent`].
    pub fn new_at_phent(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPhent,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtPhnum`].
    pub fn new_at_phnum(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPhnum,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtPagesz`].
    pub fn new_at_pagesz(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPagesz,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtBase`].
    pub fn new_at_base(val: usize) -> Self {
        Self {
            key: AuxVarType::AtBase,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtFlags`].
    pub fn new_at_flags(val: usize) -> Self {
        Self {
            key: AuxVarType::AtFlags,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtEntry`].
    pub fn new_at_entry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtEntry,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtNotelf`].
    pub fn new_at_notelf(val: usize) -> Self {
        Self {
            key: AuxVarType::AtNotelf,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtUid`].
    pub fn new_at_uid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtUid,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtEuid`].
    pub fn new_at_euid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtEuid,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtGid`].
    pub fn new_at_gid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtGid,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtEgid`].
    pub fn new_at_egid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtEgid,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtPlatform`].
    pub fn new_at_platform(c_str_bytes: &'a [u8]) -> Self {
        Self::assert_valid_null_terminated_cstr(c_str_bytes);
        Self {
            key: AuxVarType::AtPlatform,
            data: AuxVarData::ReferencedData(c_str_bytes),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtHwcap`].
    pub fn new_at_hwcap(val: usize) -> Self {
        Self {
            key: AuxVarType::AtHwcap,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtClktck`].
    pub fn new_at_clktck(val: usize) -> Self {
        Self {
            key: AuxVarType::AtClktck,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtSecure`].
    pub fn new_at_secure(val: bool) -> Self {
        let val = if val { 1 } else { 0 };
        Self {
            key: AuxVarType::AtSecure,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtBasePlatform`].
    pub fn new_at_base_platform(c_str_bytes: &'a [u8]) -> Self {
        Self::assert_valid_null_terminated_cstr(c_str_bytes);
        Self {
            key: AuxVarType::AtBasePlatform,
            data: AuxVarData::ReferencedData(c_str_bytes),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtRandom`].
    ///
    /// # Parameters
    /// * `bytes` 16 random bytes
    pub fn new_at_random(bytes: &'a [u8]) -> Self {
        assert_eq!(bytes.len(), 16, "needs exactly 16 random bytes");
        Self {
            key: AuxVarType::AtRandom,
            data: AuxVarData::ReferencedData(bytes),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtHwcap2`].
    pub fn new_at_hwcap2(val: usize) -> Self {
        Self {
            key: AuxVarType::AtHwcap2,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtExecFn`].
    pub fn new_at_exec_fn(c_str_bytes: &'a [u8]) -> Self {
        Self::assert_valid_null_terminated_cstr(c_str_bytes);
        Self {
            key: AuxVarType::AtExecFn,
            data: AuxVarData::ReferencedData(c_str_bytes),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtSysinfo`].
    pub fn new_at_sysinfo(val: usize) -> Self {
        Self {
            key: AuxVarType::AtSysinfo,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtSysinfoEhdr`].
    pub fn new_at_sysinfo_ehdr(val: usize) -> Self {
        Self {
            key: AuxVarType::AtSysinfoEhdr,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL1iCachesize`].
    pub fn new_at_l1i_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1iCachesize,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL1iCachegeometry`].
    pub fn new_at_l1i_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1iCachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL1dCachesize`].
    pub fn new_at_l1d_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1dCachesize,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL1dCachegeometry`].
    pub fn new_at_l1d_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1dCachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL2Cachesize`].
    pub fn new_at_l2_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL2Cachesize,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL2Cachegeometry`].
    pub fn new_at_l2_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL2Cachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL3Cachesize`].
    pub fn new_at_l3_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL3Cachesize,
            data: AuxVarData::Value(val),
        }
    }

    /// Constructs a new aux var of type [`AuxVarType::AtL3Cachegeometry`].
    pub fn new_at_l3_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL3Cachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    /// Returns the key/ype of this entry. See [`AuxVarType`].
    pub fn key(&self) -> AuxVarType {
        self.key
    }

    /// Returns the data of this entry. See [`AuxVarData`].
    pub fn data(&self) -> &AuxVarData<'a> {
        &self.data
    }

    /// Asserts that the bytes represent a null-terminated C string. There is exactly one
    /// null byte required at the last position.
    fn assert_valid_null_terminated_cstr(bytes: &[u8]) {
        let last_byte = bytes.last().expect("must contain at least null byte");
        assert_eq!(
            *last_byte, 0,
            "last byte must be null byte. C-strings are null-terminated!"
        );
        let null_byte_count = bytes.iter().filter(|x| **x == 0).count();
        assert_eq!(
            null_byte_count, 1,
            "Null-bytes not allowed inside the string."
        );
    }
}

impl<'a> PartialEq for AuxVar<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.key().eq(&other.key())
    }
}

impl<'a> PartialOrd for AuxVar<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // this is important. It guarantees, that the terminating null entry is the last
        // in the vector and will always be written
        if self.key() != other.key() && self.key() == AuxVarType::AtNull {
            Some(Ordering::Greater)
        } else {
            self.key().partial_cmp(&other.key())
        }
    }
}

impl<'a> Eq for AuxVar<'a> {}

impl<'a> Ord for AuxVar<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

/// High-level abstraction over the data of an [`AuxVar`].
pub enum AuxVarData<'a> {
    /// The value of the AT variable is a simple value. Depending on the context,
    /// this can be a pointer into the user memory, a function pointer, or just
    /// a regular integer or boolean.
    Value(usize),

    /// The value of the [`AuxVar`] is a pointer into the auxiliary vector data area of the
    /// data structure/stack layout. The inner data of this type is a reference to the bytes.
    /// The bytes will either be a null-terminated C-string or depending on the [`AuxVarType`]
    /// a byte array with a well-known length.
    ///
    /// If an [`AuxVar`] is constructed (see [`crate::InitialLinuxLibcStackLayoutBuilder`]),
    /// this will point to some memory, that will be serialized into the final data structure.
    ReferencedData(&'a [u8]),
}

impl<'a> AuxVarData<'a> {
    /// Returns the raw value. If the entry references data, this will return the address.
    pub fn raw_value(&self) -> usize {
        match self {
            AuxVarData::Value(val) => *val,
            AuxVarData::ReferencedData(bytes) => bytes.as_ptr() as _,
        }
    }

    pub const fn value(&self) -> Option<usize> {
        match self {
            AuxVarData::Value(val) => Some(*val),
            _ => None,
        }
    }

    pub const fn referenced_data(&self) -> Option<&'a [u8]> {
        match self {
            AuxVarData::ReferencedData(bytes) => Some(*bytes),
            _ => None,
        }
    }

    /// Convenient wrapper around [`Self::referenced_data`], that creates a Rust string slice
    /// from the data. The caller must make sure that this is valid.
    ///
    /// # Safety
    /// Safe when the underlying data is a null-terminated C-string.
    pub unsafe fn cstr(&self) -> Option<&str> {
        self.referenced_data()
            .map(|bytes| core::str::from_utf8_unchecked(bytes))
    }
}

impl<'a> Debug for AuxVarData<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            AuxVarData::Value(val) => {
                write!(f, "AuxVarData::Value({:?})", *val as *const u8)?;
            }
            AuxVarData::ReferencedData(bytes) => {
                // don't output the data directly, because the addresses might
                // be invalid (for other address space)
                write!(f, "AuxVarData::ReferencedData(@ {:?})", bytes.as_ptr())?;
            }
        }
        Ok(())
    }
}

impl<'a> From<&AuxVarSerialized<'a>> for AuxVar<'a> {
    /// Creates the high-level type [`AuxVar`] from the serialized version of an
    /// AT variable/aux vec entry.
    fn from(serialized: &AuxVarSerialized<'a>) -> Self {
        let data = if !serialized.key().value_in_data_area() {
            AuxVarData::Value(serialized.val())
        } else {
            // so far, no memory errors can happen here, because no (user addresses) are accessed yet
            let data_ptr = serialized.val() as *const u8;
            let len = serialized
                .key()
                .data_area_val_size_hint()
                // + null byte
                .unwrap_or_else(|| c_str_len(data_ptr) + 1);
            let slice = unsafe { core::slice::from_raw_parts(data_ptr, len) };
            AuxVarData::ReferencedData(slice)
        };

        // Absolutely type safe construction. This way, one can easily detect memory errors,
        // i.e. read on illegal memory (that is still valid; i.e. no pagefault).

        match serialized.key() {
            AuxVarType::AtNull => AuxVar::new_at_null(),
            AuxVarType::AtIgnore => AuxVar::new_at_ignore(data.value().unwrap()),
            AuxVarType::AtExecFd => AuxVar::new_at_exec_fd(data.value().unwrap()),
            AuxVarType::AtPhdr => AuxVar::new_at_phdr(data.value().unwrap()),
            AuxVarType::AtPhent => AuxVar::new_at_phent(data.value().unwrap()),
            AuxVarType::AtPhnum => AuxVar::new_at_phnum(data.value().unwrap()),
            AuxVarType::AtPagesz => AuxVar::new_at_pagesz(data.value().unwrap()),
            AuxVarType::AtBase => AuxVar::new_at_base(data.value().unwrap()),
            AuxVarType::AtFlags => AuxVar::new_at_flags(data.value().unwrap()),
            AuxVarType::AtEntry => AuxVar::new_at_entry(data.value().unwrap()),
            AuxVarType::AtNotelf => AuxVar::new_at_notelf(data.value().unwrap()),
            AuxVarType::AtUid => AuxVar::new_at_uid(data.value().unwrap()),
            AuxVarType::AtEuid => AuxVar::new_at_euid(data.value().unwrap()),
            AuxVarType::AtGid => AuxVar::new_at_gid(data.value().unwrap()),
            AuxVarType::AtEgid => AuxVar::new_at_egid(data.value().unwrap()),
            AuxVarType::AtPlatform => AuxVar::new_at_platform(data.referenced_data().unwrap()),
            AuxVarType::AtHwcap => AuxVar::new_at_hwcap(data.value().unwrap()),
            AuxVarType::AtClktck => AuxVar::new_at_hwcap(data.value().unwrap()),
            AuxVarType::AtSecure => AuxVar::new_at_secure(data.value().unwrap() != 0),
            AuxVarType::AtBasePlatform => {
                AuxVar::new_at_base_platform(data.referenced_data().unwrap())
            }
            AuxVarType::AtRandom => AuxVar::new_at_random(data.referenced_data().unwrap()),
            AuxVarType::AtHwcap2 => AuxVar::new_at_hwcap2(data.value().unwrap()),
            AuxVarType::AtExecFn => AuxVar::new_at_exec_fn(data.referenced_data().unwrap()),
            AuxVarType::AtSysinfo => AuxVar::new_at_sysinfo(data.value().unwrap()),
            AuxVarType::AtSysinfoEhdr => AuxVar::new_at_sysinfo_ehdr(data.value().unwrap()),
            AuxVarType::AtL1iCachesize => AuxVar::new_at_l1i_cache_size(data.value().unwrap()),
            AuxVarType::AtL1iCachegeometry => {
                AuxVar::new_at_l1i_cache_geometry(data.value().unwrap())
            }
            AuxVarType::AtL1dCachesize => AuxVar::new_at_l1d_cache_size(data.value().unwrap()),
            AuxVarType::AtL1dCachegeometry => {
                AuxVar::new_at_l1d_cache_geometry(data.value().unwrap())
            }
            AuxVarType::AtL2Cachesize => AuxVar::new_at_l2_cache_size(data.value().unwrap()),
            AuxVarType::AtL2Cachegeometry => {
                AuxVar::new_at_l2_cache_geometry(data.value().unwrap())
            }
            AuxVarType::AtL3Cachesize => AuxVar::new_at_l3_cache_size(data.value().unwrap()),
            AuxVarType::AtL3Cachegeometry => {
                AuxVar::new_at_l3_cache_geometry(data.value().unwrap())
            }
            #[allow(unreachable_patterns)]
            _ => panic!(
                "invalid AT variable: {:?}: {:?} => {:?}",
                serialized as *const AuxVarSerialized,
                serialized.key(),
                serialized.val() as *const u8
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::mem::size_of;

    /// Tests that the ATNull entry always comes last in an ordered collection. This enables
    /// us to easily write all AT-VARs at once but keep the terminating null entry at the end.
    #[test]
    fn test_aux_var_order() {
        let mut set = BTreeSet::new();
        set.insert(AuxVar::new_at_exec_fn(b"./executable\0"));
        set.insert(AuxVar::new_at_platform(b"x86_64\0"));
        set.insert(AuxVar::new_at_null());
        set.insert(AuxVar::new_at_clktck(0x1337));
        set.insert(AuxVar::new_at_exec_fn(b"./executable\0"));
        assert_eq!(set.iter().last().unwrap().key(), AuxVarType::AtNull);
    }
}
