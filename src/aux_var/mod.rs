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

use core::cmp::Ordering;
use core::fmt::{Debug, Formatter, Write};
pub use serialized::*;
pub use typ::*;

/// High-level version of [`AuxVarSerialized`]. It is used to construct the auxiliary vector in
/// [`crate::InitialLinuxLibcStackLayoutBuilder`] and returned by
#[derive(Debug)]
pub struct AuxVar<'a> {
    key: AuxVarType,
    data: AuxVarData<'a>,
}

impl<'a> AuxVar<'a> {
    /// Generic constructor. It is recommended to use the specific constructors for
    /// better validation and type safety.
    pub fn new_generic(key: AuxVarType, data: AuxVarData<'a>) -> Self {
        Self { key, data }
    }

    pub fn new_at_null() -> Self {
        Self {
            key: AuxVarType::AtNull,
            data: AuxVarData::Value(0),
        }
    }

    pub fn new_at_ignore(val: usize) -> Self {
        Self {
            key: AuxVarType::AtIgnore,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_exec_fd(val: usize) -> Self {
        Self {
            key: AuxVarType::AtExecFd,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_phdr(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPhdr,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_phent(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPhent,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_phnum(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPhnum,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_pagesz(val: usize) -> Self {
        Self {
            key: AuxVarType::AtPagesz,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_base(val: usize) -> Self {
        Self {
            key: AuxVarType::AtBase,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_flags(val: usize) -> Self {
        Self {
            key: AuxVarType::AtFlags,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_entry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtEntry,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_notelf(val: usize) -> Self {
        Self {
            key: AuxVarType::AtNotelf,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_uid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtUid,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_euid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtEuid,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_gid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtGid,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_egid(val: usize) -> Self {
        Self {
            key: AuxVarType::AtEgid,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_platform(c_str_bytes: &'a [u8]) -> Self {
        Self::assert_valid_null_terminated_cstr(c_str_bytes);
        Self {
            key: AuxVarType::AtPlatform,
            data: AuxVarData::ReferencedData(c_str_bytes),
        }
    }

    pub fn new_at_hwcap(val: usize) -> Self {
        Self {
            key: AuxVarType::AtHwcap,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_clktck(val: usize) -> Self {
        Self {
            key: AuxVarType::AtClktck,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_secure(val: usize) -> Self {
        Self {
            key: AuxVarType::AtSecure,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_base_platform(c_str_bytes: &'a [u8]) -> Self {
        Self::assert_valid_null_terminated_cstr(c_str_bytes);
        Self {
            key: AuxVarType::AtBasePlatform,
            data: AuxVarData::ReferencedData(c_str_bytes),
        }
    }

    pub fn new_at_random(bytes: &'a [u8; 16]) -> Self {
        Self {
            key: AuxVarType::AtRandom,
            data: AuxVarData::ReferencedData(&bytes[..]),
        }
    }

    pub fn new_at_hwcap2(val: usize) -> Self {
        Self {
            key: AuxVarType::AtHwcap2,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_exec_fn(c_str_bytes: &'a [u8]) -> Self {
        Self::assert_valid_null_terminated_cstr(c_str_bytes);
        Self {
            key: AuxVarType::AtExecFn,
            data: AuxVarData::ReferencedData(c_str_bytes),
        }
    }

    pub fn new_at_sysinfo(val: usize) -> Self {
        Self {
            key: AuxVarType::AtSysinfo,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_sysinfo_ehdr(val: usize) -> Self {
        Self {
            key: AuxVarType::AtSysinfoEhdr,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l1i_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1iCachesize,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l1i_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1iCachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l1d_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1dCachesize,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l1d_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL1dCachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l2_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL2Cachesize,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l2_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL2Cachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l3_cache_size(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL3Cachesize,
            data: AuxVarData::Value(val),
        }
    }

    pub fn new_at_l3_cache_geometry(val: usize) -> Self {
        Self {
            key: AuxVarType::AtL3Cachegeometry,
            data: AuxVarData::Value(val),
        }
    }

    pub fn key(&self) -> AuxVarType {
        self.key
    }

    pub fn data(&self) -> &AuxVarData<'a> {
        &self.data
    }

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

pub enum AuxVarData<'a> {
    Value(usize),
    ReferencedData(&'a [u8]),
}

impl<'a> AuxVarData<'a> {
    pub(crate) fn new_value(value: usize) -> Self {
        Self::Value(value)
    }

    pub(crate) fn new_referenced_data(referenced_data: &'a [u8]) -> Self {
        Self::ReferencedData(referenced_data)
    }

    /// Returns the raw value. If the entry references data, this will return the address.
    pub fn raw_value(&self) -> usize {
        match self {
            AuxVarData::Value(val) => *val,
            AuxVarData::ReferencedData(bytes) => bytes.as_ptr() as _,
        }
    }

    pub fn value(&self) -> Option<usize> {
        match self {
            AuxVarData::Value(val) => Some(*val),
            _ => None,
        }
    }

    pub fn referenced_data(&self) -> Option<&'a [u8]> {
        match self {
            AuxVarData::ReferencedData(bytes) => Some(*bytes),
            _ => None,
        }
    }

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
