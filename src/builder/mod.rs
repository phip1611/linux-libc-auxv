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
//! Module for [`InitialLinuxLibcStackLayoutBuilder`].
mod serializer;

use serializer::*;

use crate::{AuxVar, AuxVarSerialized, AuxVarType};
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::mem::size_of;

/// Builder to construct the stack layout that a libc implementation under Linux initially
/// expects. See <https://lwn.net/Articles/631631/> for more info. It helps to write the
/// arguments, the environment variables, and the auxiliary vector at a given address.
/// It will translate addresses (pointers) to user addresses. Serialization is done
/// with [`InitialLinuxLibcStackLayoutBuilder::serialize_into_buf`].
#[derive(Debug, Default)]
pub struct InitialLinuxLibcStackLayoutBuilder<'a> {
    /// List of C-strings for program arguments/argument variables.
    arg_v: Vec<&'a [u8]>,
    /// List of C-strings for environment variables.
    env_v: Vec<&'a [u8]>,
    /// List of (key=value)-pairs for the auxiliary vector.
    aux_v: BTreeSet<AuxVar<'a>>,
}

impl<'a> InitialLinuxLibcStackLayoutBuilder<'a> {
    /// Creates a new [`InitialLinuxLibcStackLayoutBuilder`]. The AUX entries [`AuxVarType::AtNull`]
    /// and [`AuxVarType::AtExecFn`] will be always present.
    pub fn new() -> Self {
        let mut map = BTreeSet::new();
        // this should always be present
        map.insert(AuxVar::new_at_exec_fn(b"\0"));
        // important; keep this in vector early => length calculation of total keys stays correct
        map.insert(AuxVar::new_at_null());
        Self {
            arg_v: vec![],
            env_v: vec![],
            aux_v: map,
        }
    }

    /// Serializes the data structure into the provided buffer.
    ///
    /// # Parameters
    /// * `write_buf`: Destination buffer that must be at least [`Self::total_size`] bytes long.
    /// * `user_ptr`: Stack pointer in user address space. Important, so that all pointers are valid
    ///               and can be dereferenced by libc (or the entity that parses the structure).
    ///
    /// # Safety
    /// This function is safe, as long as `write_buf` points to valid memory.
    pub unsafe fn serialize_into_buf(&self, write_buf: &mut [u8], user_ptr: u64) {
        assert!(
            write_buf.len() >= self.total_size(),
            "the buffer is not big enough!"
        );
        let write_ptr = write_buf.as_mut_ptr();
        let mut writer = AuxvSerializer::new(self, write_ptr, user_ptr);
        writer.write_argc(self.arg_v.len() as u64);
        for arg in &self.arg_v {
            writer.write_arg(arg);
        }
        writer.write_finish_argv();
        for env in &self.env_v {
            writer.write_env(env);
        }
        writer.write_finish_envv();

        // this will also write AT_NULL finally, because it is always at last position in `aux_v`.
        for aux in &self.aux_v {
            writer.write_aux_entry(aux)
        }

        writer.write_finish();
    }

    /// Adds an argument. An argument is an null-terminated C-string.
    ///
    /// # Parameters
    /// * `c_str` null-terminated c-string
    pub fn add_arg_v(mut self, c_str: &'a [u8]) -> Self {
        assert_eq!(c_str[c_str.len() - 1], 0, "c_str must be null-terminated!");
        self.arg_v.push(c_str);
        self
    }

    /// Adds an environmental variable. An envv is a null-terminated C-string with
    /// a format of `KEY=VALUE\0`.
    ///
    /// # Parameters
    /// * `c_str` null-terminated c-string
    pub fn add_env_v(mut self, c_str: &'a [u8]) -> Self {
        assert_eq!(c_str[c_str.len() - 1], 0, "c_str must be null-terminated!");
        self.env_v.push(c_str);
        self
    }

    /// Adds an aux entry.
    ///
    /// # Parameters
    /// * `var`: See [`AuxVar`]. Make sure that the payload is correct, i.e.
    ///          C-strings are null terminated.
    pub fn add_aux_v(mut self, var: AuxVar<'a>) -> Self {
        // insert alone is not enough
        if self.aux_v.contains(&var) {
            self.aux_v.replace(var);
        } else {
            self.aux_v.insert(var);
        }
        self
    }

    /// Returns the number in bytes the data structure will have including the final
    /// null byte.
    pub fn total_size(&self) -> usize {
        // final null is 64 byte long
        self.offset_to_final_null() + size_of::<u64>()
    }

    /// Returns the total offset from the begin pointer to the aux data area.
    const fn offset_to_argv_key_area(&self) -> usize {
        // there is only argc before this
        size_of::<u64>()
    }

    /// Returns the total offset from the begin pointer to the aux data area.
    fn offset_to_envv_key_area(&self) -> usize {
        self.offset_to_argv_key_area() + self.argv_keys_size()
    }

    /// Returns the total offset from the begin pointer to the aux data area.
    fn offset_to_aux_key_area(&self) -> usize {
        self.offset_to_envv_key_area() + self.envv_keys_size()
    }

    /// Returns the total offset from the begin pointer to the aux data area.
    fn offset_to_aux_data_area(&self) -> usize {
        let mut sum = self.offset_to_aux_key_area() + self.aux_keys_size();

        // TODO seems like Linux does some more magic for stack alignment
        //  https://elixir.bootlin.com/linux/v5.15.5/source/fs/binfmt_elf.c#L200
        //  Maybe solve this in the future?! IMHO this looks negligible.

        // align up to next 16 byte boundary
        if sum % 16 != 0 {
            sum += 16 - sum % 16;
        }
        sum
    }

    /// Returns the total offset from the begin pointer to the args data area.
    fn offset_to_argv_data_area(&self) -> usize {
        let mut sum = self.offset_to_aux_data_area() + self.aux_data_area_size();
        // align up to next 16 byte boundary
        if sum % 16 != 0 {
            sum += 16 - sum % 16;
        }
        sum
    }

    /// Returns the total offset from the begin pointer to the env data area.
    fn offset_to_env_data_area(&self) -> usize {
        self.offset_to_argv_data_area() + self.argv_data_area_size()
    }

    /// Returns the total offset from the begin pointer to the location of the file name.
    fn offset_to_filename_data_area(&self) -> usize {
        self.offset_to_env_data_area() + self.envv_data_area_size()
    }

    /// Returns the total offset from the begin pointer to the final null (u64).
    fn offset_to_final_null(&self) -> usize {
        let filename_bytes = self
            .filename()
            .map(|x| x.data().referenced_data())
            .flatten()
            .map(|x| x.len())
            .unwrap_or(0);
        self.offset_to_filename_data_area() + filename_bytes
    }

    /// Returns the number in bytes that all argv entries will occupy.
    /// Only the entries, but not the referenced data.
    fn argv_keys_size(&self) -> usize {
        // +1: null terminated
        size_of::<u64>() * (self.arg_v.len() + 1)
    }

    /// Returns the number in bytes that all env entries will occupy.
    /// Only the entries, but not the referenced data.
    fn envv_keys_size(&self) -> usize {
        // +1: null terminated
        size_of::<u64>() * (self.env_v.len() + 1)
    }

    /// Returns the number in bytes that all AT entries will occupy.
    /// Only the entries, but not the referenced data.
    fn aux_keys_size(&self) -> usize {
        size_of::<AuxVarSerialized>() * self.aux_v.len()
    }

    /// Returns the sum of bytes, required to store the C-string of each arg, including
    /// terminating null bytes.
    fn argv_data_area_size(&self) -> usize {
        self.arg_v.iter().map(|x| x.len()).sum()
    }

    /// Returns the sum of bytes, required to store the C-string of each env var, including
    /// terminating null bytes.
    fn envv_data_area_size(&self) -> usize {
        self.env_v.iter().map(|x| x.len()).sum()
    }

    /// Returns the number of all additional aux vec data in the aux data area, except for
    /// the executable name of [`AuxVarType::AtExecFn`], because it gets special treatment.
    fn aux_data_area_size(&self) -> usize {
        self.aux_v
            .iter()
            .filter(|x| x.key().value_in_data_area())
            // file name stands at end of the structure, before the final null byte
            // and not in the auxv data area
            .filter(|x| x.key() != AuxVarType::AtExecFn)
            .map(|x| x.data().referenced_data().unwrap().len())
            .sum()
    }

    /// Returns the filename/executable aux var, if it is present. It needs some special treatment,
    /// according to <https://lwn.net/Articles/631631/>.
    ///
    // Actually, I'm not sure if libc implementations care about the pointer location, as long as
    // the pointer is correct..
    fn filename(&self) -> Option<&AuxVar> {
        self.aux_v.iter().find(|x| x.key() == AuxVarType::AtExecFn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuxVarType;

    #[test]
    fn test_builder_write_size() {
        let builder = InitialLinuxLibcStackLayoutBuilder::new();

        let mut expected_size = 8;
        // 3 * 8: argc, argv[0]=0, envv[0]=0 + padding to 16 byte + null byte
        assert_eq!(builder.offset_to_argv_key_area(), expected_size);
        expected_size = 16;
        assert_eq!(builder.offset_to_envv_key_area(), expected_size);
        expected_size = 24;
        assert_eq!(builder.offset_to_aux_key_area(), expected_size);

        // there are two aux keys at minimum (null and file name - (key,value)-pairs)
        expected_size = 24 + 2 * size_of::<AuxVarSerialized>();
        if expected_size % 16 != 0 {
            expected_size += 16 - expected_size % 16;
        }
        assert_eq!(builder.offset_to_aux_data_area(), expected_size);
        // no additional aux data (file name (which is part of aux data) lives in dedicated data area
        assert_eq!(builder.offset_to_argv_data_area(), expected_size);
        // no args in this test
        assert_eq!(builder.offset_to_env_data_area(), expected_size);
        // no env vars in this test
        assert_eq!(builder.offset_to_filename_data_area(), expected_size);

        expected_size += 1;
        // file name is only one byte long
        assert_eq!(builder.offset_to_final_null(), expected_size);

        expected_size += 8;
        // final null value (u64)
        assert_eq!(builder.total_size(), expected_size);
    }

    #[test]
    fn test_builder_write_size_2() {
        let builder = InitialLinuxLibcStackLayoutBuilder::new()
            .add_arg_v(b"Foo\0")
            .add_env_v(b"BAR=FOO\0")
            .add_aux_v(AuxVar::new_at_platform(b"x86_64\0"))
            .add_aux_v(AuxVar::new_at_exec_fn(b"./executable\0"));

        assert_eq!(builder.offset_to_argv_key_area(), 8);
        // + 8 + 8 (one entry + null byte)
        assert_eq!(builder.offset_to_envv_key_area(), 24);
        // + 8 + 8 (one entry + null byte)
        assert_eq!(builder.offset_to_aux_key_area(), 40);
        // + three keys + align to 16 byte boundary
        let mut expected_size = 40 + 3 * size_of::<AuxVarSerialized>();
        if expected_size % 16 != 0 {
            expected_size += 16 - expected_size % 16;
        }
        assert_eq!(builder.offset_to_aux_data_area(), expected_size);

        expected_size += 7;
        if expected_size % 16 != 0 {
            expected_size += 16 - expected_size % 16;
        }
        // + 7 (length of "x86_64\0") + align to 16 byte boundary
        assert_eq!(builder.offset_to_argv_data_area(), expected_size);

        expected_size += 4;
        // + 4 (length of "Foo\0")
        assert_eq!(builder.offset_to_env_data_area(), expected_size);

        expected_size += 8;
        // + 8 (length of "BAR=FOO\0")
        assert_eq!(builder.offset_to_filename_data_area(), expected_size);

        expected_size += 13;
        // + 13 (length of "./executable\0")
        assert_eq!(builder.offset_to_final_null(), expected_size);
    }

    /// Make sure that the AtNull entry is always the last. It must always be present and written
    /// as last entry.
    #[test]
    fn test_builder_aux_final_at_null() {
        assert_eq!(
            InitialLinuxLibcStackLayoutBuilder::new()
                .aux_v
                .iter()
                .last()
                .unwrap()
                .key(),
            AuxVarType::AtNull
        );
        assert_eq!(
            InitialLinuxLibcStackLayoutBuilder::new()
                .add_aux_v(AuxVar::new_at_clktck(0x1337))
                .add_aux_v(AuxVar::new_at_null())
                .add_aux_v(AuxVar::new_at_platform(b"x86_64\0"))
                .aux_v
                .iter()
                .last()
                .unwrap()
                .key(),
            AuxVarType::AtNull
        );
    }

    #[test]
    fn test_builder_serializes_data() {
        let builder = InitialLinuxLibcStackLayoutBuilder::new()
            .add_arg_v(b"Foo\0")
            .add_env_v(b"BAR=FOO\0")
            .add_aux_v(AuxVar::new_at_platform(b"x86_64\0"))
            .add_aux_v(AuxVar::new_at_exec_fn(b"./executable\0"))
            .add_aux_v(AuxVar::new_at_uid(0xdeadbeef))
            .add_aux_v(AuxVar::new_at_clktck(123456));
        let mut buf = vec![0; builder.total_size()];

        unsafe {
            // user_addr == write_addr => easy debugging; segfaults otherwise when resolving pointers
            let user_ptr = buf.as_ptr();
            builder.serialize_into_buf(&mut buf, user_ptr as u64);
        }

        dbg!(&buf);

        /* to check the data structure in an existing C tool
        println!("unsigned char foo[] = {{");
        for byte in &buf {
            println!("     0x{:x},", byte);
        }
        println!("}};");*/
    }

    #[should_panic]
    #[test]
    fn test_panic_filename_not_null_terminated() {
        InitialLinuxLibcStackLayoutBuilder::new().add_aux_v(AuxVar::new_at_exec_fn(b""));
    }

    #[test]
    fn test_default_filename_gets_replaced() {
        let expected = b"foo\0";
        let b =
            InitialLinuxLibcStackLayoutBuilder::new().add_aux_v(AuxVar::new_at_exec_fn(expected));
        let actual = b.filename().unwrap().data().referenced_data().unwrap();
        assert_eq!(actual, expected);
    }
}
