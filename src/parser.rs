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
use crate::cstr_util::c_str_len_ptr;
use crate::{AuxVar, AuxVarSerialized, AuxVarType};
use core::fmt::Debug;
use core::marker::PhantomData;

/// Wrapper around a slice of data, that represents the data structure that Linux passes to the
/// libc on program startup. Usually this is a struct from `rsp` (stack pointer) to `x`. It is no
/// problem, if you pass for example a slice with 10000 bytes to it, because it will automatically
/// stop, when the end of the data structure is found. Hence, if the data structure is valid,
/// invalid memory (above the stack) will never be accessed.
///
/// It contains `argc`, `argv`, `envv`, and the auxiliary vector along with additional referenced
/// payload. The data structure is right above the stack. The initial stack pointer points
/// to `argc`. See <https://lwn.net/Articles/631631/> for more info.
///
/// Instances are created via `InitialLinuxLibcStackLayout::from::<[u8>]`.
#[derive(Debug)]
pub struct InitialLinuxLibcStackLayout<'a> {
    bytes: &'a [u8],
}

impl<'a> From<&'a [u8]> for InitialLinuxLibcStackLayout<'a> {
    /// Creates a new [`InitialLinuxLibcStackLayout`].
    fn from(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}

impl<'a> InitialLinuxLibcStackLayout<'a> {
    /// Returns the number of arguments.
    #[allow(clippy::missing_const_for_fn)]
    pub fn argc(&self) -> usize {
        unsafe { *self.bytes.as_ptr().cast() }
    }

    /// Returns the number of environment variables.
    pub fn envc(&self) -> usize {
        self.envv_ptr_iter().count()
    }

    /// Returns the pointer to the begin of argv.
    fn get_argv_ptr(&self) -> *const *const u8 {
        // + 1: skip argc
        let ptr = unsafe { self.bytes.as_ptr().cast::<u64>().add(1) };
        // C-str array: array of pointers => pointer to pointer to bytes of c-str
        ptr as *const *const u8
    }

    /// Iterates over the C-string arguments.
    /// See [`CstrIter`].
    pub fn argv_iter(&self) -> CstrIter {
        CstrIter::new(self.get_argv_ptr())
    }

    /// Iterates only over the pointers of the C-string arguments.
    /// See [`NullTerminatedArrIter`].
    pub fn argv_ptr_iter(&self) -> NullTerminatedArrIter {
        NullTerminatedArrIter {
            ptr: self.get_argv_ptr(),
        }
    }

    /// Returns the pointer to the beginning of environment variables.
    fn get_envv_ptr(&self) -> *const *const u8 {
        unsafe {
            self.get_argv_ptr()
                .add(self.argc())
                // final null ptr after the envv (+ 8 bytes)
                .add(1)
        }
    }

    /// Iterates over all environment variables.
    /// See [`CstrIter`].
    pub fn envv_iter(&self) -> CstrIter {
        CstrIter::new(self.get_envv_ptr())
    }

    /// Iterates only over the pointers to the environment variables.
    /// See [`NullTerminatedArrIter`].
    pub fn envv_ptr_iter(&self) -> NullTerminatedArrIter {
        NullTerminatedArrIter {
            ptr: self.get_envv_ptr(),
        }
    }

    /// Iterates over all entries in the auxiliary vector.
    /// See [`AuxVecIter`].
    pub fn aux_iter(&self) -> AuxVecIter {
        AuxVecIter::new(self.get_auxv_ptr())
    }

    /// Returns the pointer to the beginning of aux variables.
    fn get_auxv_ptr(&self) -> *const AuxVarSerialized {
        unsafe {
            self.get_envv_ptr()
                // skip all ENV values
                .add(self.envv_ptr_iter().count())
                // final null ptr after the envv (+ 8 bytes)
                .add(1)
                .cast()
        }
    }
}

/// Iterator that iterates over an array of pointers, that is terminated by a null pointer.
/// Useful to find all entries of a typical C-string array.
/// It only returns the pointer itself but doesn't dereferences the data.
#[derive(Debug)]
pub struct NullTerminatedArrIter {
    ptr: *const *const u8,
}

impl Iterator for NullTerminatedArrIter {
    type Item = *const u8;

    fn next(&mut self) -> Option<Self::Item> {
        if unsafe { (*self.ptr).is_null() } {
            None
        } else {
            let c_str_ptr = unsafe { *self.ptr };
            // + 8 bytes: to next array entry
            self.ptr = unsafe { self.ptr.add(1) };
            Some(c_str_ptr)
        }
    }
}

/// Iterator that iterates over an array of null terminated C-strings.
#[derive(Debug)]
pub struct CstrIter<'a> {
    arr_iter: NullTerminatedArrIter,
    _marker: PhantomData<&'a ()>,
}

impl<'a> CstrIter<'a> {
    fn new(ptr: *const *const u8) -> Self {
        Self {
            arr_iter: NullTerminatedArrIter { ptr },
            _marker: PhantomData::default(),
        }
    }
}

impl<'a> Iterator for CstrIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.arr_iter.next().map(|c_str_ptr| {
            // + null byte
            let c_str_bytes =
                unsafe { core::slice::from_raw_parts(c_str_ptr, c_str_len_ptr(c_str_ptr) + 1) };
            unsafe { core::str::from_utf8_unchecked(c_str_bytes) }
        })
    }
}

/// Iterator over all entries in the auxiliary vector.
#[derive(Debug)]
pub struct AuxVecIter<'a> {
    ptr: *const AuxVarSerialized<'a>,
    done: bool,
    _marker: PhantomData<&'a ()>,
}

impl<'a> AuxVecIter<'a> {
    fn new(ptr: *const AuxVarSerialized<'a>) -> Self {
        Self {
            ptr,
            done: false,
            _marker: PhantomData::default(),
        }
    }
}

impl<'a> Iterator for AuxVecIter<'a> {
    type Item = AuxVar<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            None
        } else {
            let aux_var_ser = unsafe { self.ptr.as_ref().unwrap() };
            if aux_var_ser.key() == AuxVarType::Null {
                if aux_var_ser.val() != 0 {
                    panic!(
                        "val of end key is not null but {}! Probably read wrong memory!",
                        aux_var_ser.val()
                    );
                }
                self.done = true;
            }

            self.ptr = unsafe { self.ptr.add(1) };

            unsafe { Some(AuxVar::from_serialized(aux_var_ser)) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuxVar, InitialLinuxLibcStackLayoutBuilder};
    use std::vec::Vec;

    // This test is not optimal, because its some kind of "self fulfilling prophecy".
    // I try to parse the format, that my builder creates..
    #[test]
    fn test_parser_with_dereference_data() {
        let builder = InitialLinuxLibcStackLayoutBuilder::new()
            .add_arg_v(b"first_arg\0")
            .add_arg_v(b"second_arg\0")
            .add_arg_v(b"third__arg\0")
            .add_env_v(b"ENV1=FOO\0")
            .add_env_v(b"ENV2=BAR\0")
            .add_env_v(b"ENV3=FOOBAR\0")
            .add_aux_v(AuxVar::Platform("x86_64"))
            .add_aux_v(AuxVar::Uid(0xdeadbeef));
        let mut buf = vec![0; builder.total_size()];

        unsafe {
            // user_addr == write_addr => easy debugging
            let user_ptr = buf.as_ptr() as u64;
            builder.serialize_into_buf(buf.as_mut_slice(), user_ptr);
        }

        let parsed = InitialLinuxLibcStackLayout::from(buf.as_slice());
        dbg!(parsed.argc());
        dbg!(parsed.argv_ptr_iter().collect::<Vec<_>>());
        dbg!(parsed.argv_iter().collect::<Vec<_>>());
        dbg!(parsed.envv_ptr_iter().collect::<Vec<_>>());
        dbg!(parsed.envv_iter().collect::<Vec<_>>());
        dbg!(parsed.aux_iter().collect::<Vec<_>>());
    }

    /// Test similar to the one above, but uses "0x1000" as user address. This
    /// makes it easy to check if everything is at the right offset.
    #[test]
    fn test_parser_different_user_ptr() {
        let builder = InitialLinuxLibcStackLayoutBuilder::new()
            .add_arg_v(b"first_arg\0")
            .add_arg_v(b"second_arg\0")
            .add_arg_v(b"third__arg\0")
            .add_env_v(b"ENV1=FOO\0")
            .add_env_v(b"ENV2=BAR\0")
            .add_env_v(b"ENV3=FOOBAR\0")
            .add_aux_v(AuxVar::Platform("x86_64\0"))
            .add_aux_v(AuxVar::Uid(0xdeadbeef));
        let mut buf = Vec::with_capacity(builder.total_size());
        unsafe {
            buf.set_len(buf.capacity());
            buf.fill(0);
        }

        unsafe {
            // this only works if the data is not dereferenced
            builder.serialize_into_buf(buf.as_mut_slice(), 0x1000);
        }

        let parsed = InitialLinuxLibcStackLayout::from(buf.as_slice());

        dbg!(parsed.argv_ptr_iter().collect::<Vec<_>>());
        dbg!(parsed.envv_ptr_iter().collect::<Vec<_>>());

        // TODO add more sensible test; check offsets etc

        // debug already resolves memory addresses => in this test => memory errors
        // dbg!(parsed.aux_iter().collect::<Vec<_>>());
    }
}
