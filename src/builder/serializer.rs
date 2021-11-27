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
//! Module for [`AuxvSerializer`].
use crate::builder::{AuxVar, InitialLinuxLibcStackLayoutBuilder};
use crate::AuxVarType;
use core::mem::size_of;

/// Helper for [`AuxVectorStackLayoutBuilder`]. Helps to serialize the args, the env vars,
/// and the aux vector.
pub(super) struct AuxvSerializer<'a> {
    /// Required to check during runtime if too many values are written.
    builder: &'a InitialLinuxLibcStackLayoutBuilder<'a>,
    // all pointers are byte pointers, which simplifies coding the pointer arithmetic
    // a bit (.add() method)
    /// Pointer to the argc information. Pointer to the beginning of the data structure.
    argc_write_ptr: *mut u8,
    /// Pointer to the next argv key.
    argv_key_write_ptr: *mut u8,
    /// Pointer to the next argv payload (c-str destination).
    argv_data_write_ptr: *mut u8,
    /// Pointer to the next env key.
    envv_key_write_ptr: *mut u8,
    /// Pointer to the next env payload (c-str destination).
    envv_data_write_ptr: *mut u8,
    /// Pointer to the next aux vec key.
    aux_key_write_ptr: *mut u8,
    /// Pointer to the next aux vec value. Only relevant for AT variables,
    /// that reference data in the auxiliary vector data area.
    aux_data_write_ptr: *mut u8,
    /// [`crate::AuxVarType::AtExecFn`] will reference this pointer. The filename of the executable will be
    /// written here. As shown in <https://lwn.net/Articles/631631/>, this AT variable
    /// requires special treatment.
    filename_write_ptr: *mut u8,
    /// The final null pointer.
    final_null_ptr: *mut u8,
    /// Base pointer in user address space. Args, Env Vars, and AT vars with referenced data
    /// will point to a specific user address rather than a relative offset (why solve things
    /// easy when you can make it complicated?! thanks Linux). Therefore this is used to calc
    /// the address in user address space so that the libc can resolve all references on valid
    /// memory.
    user_addr: u64,

    /// Number of args written. Used for runtime checks.
    arg_write_count: usize,
    /// Number of env vars written. Used for runtime checks.
    env_write_count: usize,
    /// Number of aux vars written. Used for runtime checks.
    aux_write_count: usize,
}

impl<'a> AuxvSerializer<'a> {
    pub fn new(
        builder: &'a InitialLinuxLibcStackLayoutBuilder,
        begin_ptr: *mut u8,
        user_addr: u64,
    ) -> Self {
        unsafe {
            Self {
                builder,
                // all the offsets are known during runtime beforehand: prepare pointers
                argc_write_ptr: begin_ptr,
                argv_key_write_ptr: begin_ptr.add(size_of::<u64>()),
                argv_data_write_ptr: begin_ptr.add(builder.offset_to_argv_data_area()),
                envv_key_write_ptr: begin_ptr.add(builder.offset_to_envv_key_area()),
                envv_data_write_ptr: begin_ptr.add(builder.offset_to_env_data_area()),
                aux_key_write_ptr: begin_ptr.add(builder.offset_to_aux_key_area()),
                aux_data_write_ptr: begin_ptr.add(builder.offset_to_aux_data_area()),
                filename_write_ptr: begin_ptr.add(builder.offset_to_filename_data_area()),
                final_null_ptr: begin_ptr.add(builder.offset_to_final_null()),
                user_addr,
                arg_write_count: 0,
                env_write_count: 0,
                aux_write_count: 0,
            }
        }
    }
    /// Writes how many actual args are there.
    pub unsafe fn write_argc(&mut self, argc: u64) {
        core::ptr::write(self.argc_write_ptr.cast(), argc);
    }

    /// Writes the next arg into the data structure.
    pub unsafe fn write_arg(&mut self, c_str: &[u8]) {
        assert!(
            self.builder.arg_v.len() > self.arg_write_count,
            "More arguments have been written than capacity is available!"
        );

        core::ptr::write(
            self.argv_key_write_ptr.cast(),
            self.to_user_ptr(self.argv_data_write_ptr),
        );
        core::ptr::copy_nonoverlapping(c_str.as_ptr(), self.argv_data_write_ptr, c_str.len());

        self.argv_key_write_ptr = self.argv_key_write_ptr.add(size_of::<u64>());
        self.argv_data_write_ptr = self.argv_data_write_ptr.add(c_str.len());

        self.arg_write_count += 1;
    }

    /// Writes a NULL-ptr into the data structure.
    pub unsafe fn write_finish_argv(&mut self) {
        core::ptr::write(
            self.argv_key_write_ptr.cast::<*const u8>(),
            core::ptr::null(),
        );
    }

    /// Writes the next env var into the data structure.
    pub unsafe fn write_env(&mut self, c_str: &[u8]) {
        assert!(
            self.builder.env_v.len() > self.env_write_count,
            "More arguments have been written than capacity is available!"
        );

        core::ptr::write(
            self.envv_key_write_ptr.cast(),
            self.to_user_ptr(self.envv_data_write_ptr),
        );
        core::ptr::copy_nonoverlapping(c_str.as_ptr(), self.envv_data_write_ptr, c_str.len());

        self.envv_key_write_ptr = self.envv_key_write_ptr.add(size_of::<u64>());
        self.envv_data_write_ptr = self.envv_data_write_ptr.add(c_str.len());

        self.env_write_count += 1;
    }

    /// Writes a NULL-ptr into the data structure.
    pub unsafe fn write_finish_envv(&mut self) {
        core::ptr::write(
            self.envv_key_write_ptr.cast::<*const u8>(),
            core::ptr::null(),
        );
    }

    /// Writes an aux vector pair/AT variable into the data structure.
    pub unsafe fn write_aux_entry(&mut self, aux_var: AuxVar) {
        assert!(
            self.builder.aux_v.len() > self.aux_write_count,
            "More arguments have been written than capacity is available!"
        );

        // write key
        core::ptr::write(self.aux_key_write_ptr.cast(), aux_var.typ().val());
        // increment 1/2
        self.aux_key_write_ptr = self.aux_key_write_ptr.add(size_of::<usize>());

        if !aux_var.typ().value_in_data_area() {
            // write integer value; no pointer referencing data in aux data area
            core::ptr::write(
                self.aux_key_write_ptr.cast::<usize>(),
                aux_var.integer_value(),
            );
        } else {
            // special treatment; see https://lwn.net/Articles/631631/
            if aux_var.typ() == AuxVarType::AtExecFn {
                core::ptr::write(
                    self.aux_key_write_ptr.cast(),
                    self.to_user_ptr(self.filename_write_ptr),
                );
                core::ptr::copy_nonoverlapping(
                    aux_var.bytes_value().as_ptr(),
                    self.filename_write_ptr,
                    aux_var.bytes_value().len(),
                );
                // done only once; no need to update pointer
                // self.filename_write_ptr
            } else {
                core::ptr::write(
                    self.aux_key_write_ptr.cast(),
                    self.to_user_ptr(self.aux_data_write_ptr),
                );
                core::ptr::copy_nonoverlapping(
                    aux_var.bytes_value().as_ptr(),
                    self.aux_data_write_ptr,
                    aux_var.bytes_value().len(),
                );
                // update pointer for next iteration
                self.aux_data_write_ptr = self.aux_data_write_ptr.add(aux_var.bytes_value().len());
            }
        }

        // increment 2/2 (after value/ptr was written)
        self.aux_key_write_ptr = self.aux_key_write_ptr.add(size_of::<usize>());

        self.aux_write_count += 1;
    }

    /// Writes a final NULL-ptr into the data structure.
    pub unsafe fn write_finish(&mut self) {
        core::ptr::write(self.final_null_ptr.cast::<*const u8>(), core::ptr::null());
    }

    /// Calculates the offset of the write pointer from the beginning of the data structure.
    fn get_write_ptr_offset(&self, ptr: *const u8) -> usize {
        let ptr = ptr as usize;
        // argc pointer points to very bottom of data structure
        let base = self.argc_write_ptr as usize;
        ptr - base
    }

    /// Transforms the write pointer into the corresponding pointer in the user address space.
    fn to_user_ptr(&self, write_ptr: *const u8) -> u64 {
        self.user_addr + self.get_write_ptr_offset(write_ptr) as u64
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::serializer::AuxvSerializer;
    use crate::{AuxVar, AuxVarSerialized, AuxVarType, InitialLinuxLibcStackLayoutBuilder};
    use std::mem::size_of;

    /// Dedicated test for AuxV. I needed it to find a bug.
    #[test]
    fn test_byte_writer_auxv() {
        let builder = InitialLinuxLibcStackLayoutBuilder::new()
            .add_aux_v(AuxVar::Value(AuxVarType::AtClktck, 0x1337))
            .add_aux_v(AuxVar::ReferencedData(AuxVarType::AtPlatform, b"x86_64\0"));
        let mut buf = vec![0_u8; builder.total_size()];
        let ptr = buf.as_ptr();
        let mut writer = AuxvSerializer::new(&builder, buf.as_mut_ptr(), ptr as u64);

        // check AT values / aux vec
        unsafe {
            let initial_aux_data_write_ptr = writer.aux_data_write_ptr;
            let mut bytes_written = 0;
            for aux in builder
                .aux_v
                .iter()
                .filter(|x| x.typ().value_in_data_area())
                .filter(|x| x.typ() != AuxVarType::AtExecFn)
            {
                let dst_ptr = writer.aux_data_write_ptr;
                writer.write_aux_entry(*aux);
                bytes_written += aux.bytes_value().len();
                assert_eq!(
                    *writer
                        .aux_key_write_ptr
                        .sub(2 * size_of::<usize>())
                        .cast::<usize>(),
                    aux.typ().val(),
                    "must write the correct key"
                );
                assert_eq!(
                    *writer
                        .aux_key_write_ptr
                        .sub(size_of::<usize>())
                        .cast::<u64>(),
                    dst_ptr as u64,
                    "must write the correct ptr"
                );
                assert_eq!(
                    initial_aux_data_write_ptr.add(bytes_written),
                    writer.aux_data_write_ptr,
                    "must update the data write ptr correctly"
                );
            }
        }

        // do some final checks
        assert!(writer.final_null_ptr > writer.filename_write_ptr);
        // filename ptr not updated; this is fine
        assert_eq!(writer.filename_write_ptr, writer.envv_data_write_ptr);
        assert_eq!(writer.envv_data_write_ptr, writer.argv_data_write_ptr);
        // + padding
        assert!(writer.argv_data_write_ptr > writer.aux_data_write_ptr);
        assert!(writer.aux_data_write_ptr > writer.aux_key_write_ptr);
        assert!(writer.aux_key_write_ptr > writer.envv_key_write_ptr);
        assert!(writer.envv_key_write_ptr > writer.argv_key_write_ptr);
        assert!(writer.argv_key_write_ptr > writer.argc_write_ptr);
    }

    /// Example that includes all kinds of data (argv, env, different kinds of aux vars
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_byte_writer_full() {
        let builder = InitialLinuxLibcStackLayoutBuilder::new()
            .add_arg_v(b"arg1\0")
            .add_arg_v(b"arg2\0")
            .add_arg_v(b"arg3\0")
            .add_env_v(b"ENV1=FOO1\0")
            .add_env_v(b"ENV2=FOO2\0")
            .add_env_v(b"ENV3=FOO3\0")
            .add_aux_v(AuxVar::Value(AuxVarType::AtClktck, 0x1337))
            .add_aux_v(AuxVar::ReferencedData(AuxVarType::AtPlatform, b"x86_64\0"));
        let mut buf = vec![0_u8; builder.total_size()];
        let ptr = buf.as_ptr();
        let mut writer = AuxvSerializer::new(&builder, buf.as_mut_ptr(), ptr as u64);

        // check pre-conditions
        {
            assert!(builder.offset_to_final_null() > builder.offset_to_filename_data_area());
            assert!(builder.offset_to_filename_data_area() > builder.offset_to_env_data_area());
            assert!(builder.offset_to_env_data_area() > builder.offset_to_argv_data_area());
            assert!(builder.offset_to_argv_data_area() > builder.offset_to_aux_data_area());
            assert!(builder.offset_to_aux_data_area() > builder.offset_to_aux_key_area());
            assert!(builder.offset_to_aux_key_area() > builder.offset_to_envv_key_area());
            assert!(builder.offset_to_envv_key_area() > builder.offset_to_argv_key_area());
        }

        println!(
            "{:?} - {:?}",
            builder.aux_keys_size(),
            builder.aux_data_area_size()
        );

        println!(
            "{:?} - {:?}",
            writer.aux_data_write_ptr, writer.aux_key_write_ptr
        );

        // check argv
        unsafe {
            writer.write_argc(3);
            assert_eq!(*ptr.cast::<u64>(), 3);
        }

        // check args
        unsafe {
            let initial_argv_data_write_ptr = writer.argv_data_write_ptr;
            // the count in bytes for all the c-strings of the args
            let mut arg_byte_count = 0;
            for arg in &builder.arg_v {
                let previous_ptr = writer.argv_key_write_ptr;
                writer.write_arg(arg);
                let ptr_offset = writer.argv_key_write_ptr as usize - previous_ptr as usize;
                assert_eq!(
                    ptr_offset, 8,
                    "argv_key_write_ptr must point to next ptr address"
                );

                // check that the correct length was written into the data area
                // includes null byte already
                arg_byte_count += arg.len();
                let ptr_diff =
                    writer.argv_data_write_ptr as usize - initial_argv_data_write_ptr as usize;
                assert_eq!(ptr_diff, arg_byte_count, "must write the correct amount of bytes of all c-strings for the args and update the write pointers!");
            }
            writer.write_finish_argv();
        }

        // check envs
        unsafe {
            let initial_envv_data_write_ptr = writer.envv_data_write_ptr;
            // the count in bytes for all the c-strings of the args
            let mut env_byte_count = 0;
            for env in &builder.env_v {
                let previous_ptr = writer.envv_key_write_ptr;
                writer.write_env(env);
                let ptr_offset = writer.envv_key_write_ptr as usize - previous_ptr as usize;
                assert_eq!(
                    ptr_offset, 8,
                    "envv_key_write_ptr must point to next ptr address"
                );

                // check that the correct length was written into the data area
                // includes null byte already
                env_byte_count += env.len();
                let ptr_diff =
                    writer.envv_data_write_ptr as usize - initial_envv_data_write_ptr as usize;
                assert_eq!(ptr_diff, env_byte_count, "must write the correct amount of bytes of all c-strings for the env vars and update the write pointers!");
            }
            writer.write_finish_envv();
        }

        // check AT values / aux vec
        unsafe {
            assert_eq!(
                writer.aux_key_write_ptr,
                writer.envv_key_write_ptr.add(8),
                "the first aux key follows the null ptr after the last env var key"
            );

            let initial_aux_data_write_ptr = writer.aux_data_write_ptr;
            let mut aux_data_bytes_written = 0;

            for aux in &builder.aux_v {
                writer.write_aux_entry(*aux);
                assert_eq!(
                    *writer
                        .aux_key_write_ptr
                        .sub(size_of::<AuxVarSerialized>())
                        .cast::<usize>(),
                    aux.typ().val(),
                    "must write the correct key"
                );
                if !aux.typ().value_in_data_area() {
                    assert_eq!(
                        *writer
                            .aux_key_write_ptr
                            .sub(size_of::<AuxVarSerialized>() / 2)
                            .cast::<usize>(),
                        aux.integer_value(),
                        "must write the correct value"
                    );
                } else {
                    // special treatment for this key
                    if aux.typ() == AuxVarType::AtExecFn {
                        let slice = core::slice::from_raw_parts(
                            writer.filename_write_ptr,
                            aux.bytes_value().len(),
                        );
                        assert_eq!(
                            aux.bytes_value(),
                            slice,
                            "must write the correct filename into the right location"
                        );
                    } else {
                        aux_data_bytes_written += aux.bytes_value().len();
                        assert_eq!(
                            initial_aux_data_write_ptr.add(aux_data_bytes_written),
                            writer.aux_data_write_ptr,
                            "must update aux data write ptr in the correct way"
                        );
                    }
                }
            }

            assert!(
                initial_aux_data_write_ptr < writer.aux_data_write_ptr,
                "there must be bytes written to the additional aux vec data area"
            );

            writer.write_finish();

            // check for AtNull at end
            assert_eq!(
                *writer.aux_key_write_ptr.sub(size_of::<AuxVarSerialized>()) as usize,
                AuxVarType::AtNull.val(),
                "last AT var must be AtNull"
            );
            assert_eq!(
                *writer.aux_key_write_ptr.sub(8) as u64,
                0,
                "AtNull var must have null as value!"
            );

            // do some final checks
            assert!(writer.final_null_ptr > writer.filename_write_ptr);
            // filename ptr not updated; this is fine
            assert_eq!(writer.filename_write_ptr, writer.envv_data_write_ptr);
            assert!(writer.envv_data_write_ptr > writer.argv_data_write_ptr);
            assert!(writer.argv_data_write_ptr > writer.aux_data_write_ptr);

            println!("{}", builder.offset_to_aux_data_area());
            println!(
                "{:?} - {:?}",
                writer.get_write_ptr_offset(writer.envv_data_write_ptr),
                writer.get_write_ptr_offset(writer.aux_key_write_ptr)
            );
            println!(
                "{:?} - {:?}",
                writer.aux_data_write_ptr, writer.aux_key_write_ptr
            );
            assert!(writer.aux_data_write_ptr > writer.aux_key_write_ptr);

            assert!(writer.aux_key_write_ptr > writer.envv_key_write_ptr);
            assert!(writer.envv_key_write_ptr > writer.argv_key_write_ptr);
            assert!(writer.argv_key_write_ptr > writer.argc_write_ptr);
        }
    }
}
