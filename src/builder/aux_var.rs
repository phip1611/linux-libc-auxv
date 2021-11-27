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
//! Module for [`AuxVar`].
use crate::AuxVarType;
use core::cmp::Ordering;

/// An aux vector entry/AT variable is either followed by a u64 value or by a pointer,
/// that points into the auxiliary vector data area. The referenced data in the area
/// is a null-terminated C-string or data with a well-known length.
///
/// This type is a high-level type to construct Aux vars in the [`super::InitialLinuxLibcStackLayoutBuilder`]
/// and does not correspond to the serialized format in the binary data structure.
#[derive(Copy, Clone, Debug)]
pub enum AuxVar<'a> {
    /// `val` in the `(key,val)-pair` is not a pointer into the auxiliary vector data area.
    Value(AuxVarType, usize),
    /// `val` in the `(key,val)-pair` is a pointer into the auxiliary vector data area.
    /// The data can be a null-terminated C-string or data with a well-known length.
    ReferencedData(AuxVarType, &'a [u8]),
}

impl<'a> AuxVar<'a> {
    /// Returns the type. See [`AuxVarType`].
    pub const fn typ(&self) -> AuxVarType {
        match self {
            AuxVar::Value(typ, _) => *typ,
            AuxVar::ReferencedData(typ, _) => *typ,
        }
    }

    /// Returns the integer value this belongs to.
    pub fn integer_value(&self) -> usize {
        assert!(!self.typ().value_in_data_area());
        match self {
            AuxVar::Value(_, val) => *val,
            _ => panic!("invalid variant"),
        }
    }

    /// Returns the referenced data.
    pub fn bytes_value(&self) -> &'a [u8] {
        assert!(self.typ().value_in_data_area());
        match self {
            AuxVar::ReferencedData(_, bytes) => bytes,
            _ => panic!("invalid variant"),
        }
    }
}

impl<'a> PartialEq for AuxVar<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.typ().eq(&other.typ())
    }
}

impl<'a> PartialOrd for AuxVar<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // this is important. It guarantees, that the terminating null entry is the last
        // in the vector and will always be written
        if self.typ() != other.typ() && self.typ() == AuxVarType::AtNull {
            Some(Ordering::Greater)
        } else {
            self.typ().partial_cmp(&other.typ())
        }
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
    use crate::{AuxVar, AuxVarType};
    use std::collections::BTreeSet;

    /// Tests that the ATNull entry always comes last in an ordered collection. This enables
    /// us to easily write all AT-VARs at once but keep the terminating null entry at the end.
    #[test]
    fn test_aux_var_order() {
        let mut set = BTreeSet::new();
        set.insert(AuxVar::ReferencedData(
            AuxVarType::AtExecFn,
            b"./executable\0",
        ));
        set.insert(AuxVar::ReferencedData(AuxVarType::AtPlatform, b"x86_64\0"));
        set.insert(AuxVar::Value(AuxVarType::AtNull, 0));
        set.insert(AuxVar::Value(AuxVarType::AtClktck, 1337));
        assert_eq!(set.iter().last().unwrap().typ(), AuxVarType::AtNull);
    }
}
