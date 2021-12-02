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
use linux_libc_auxv::{
    AuxVar, AuxVarType, InitialLinuxLibcStackLayout, InitialLinuxLibcStackLayoutBuilder,
};

/// Minimal example that builds the initial Linux libc stack layout. It includes args, envvs,
/// and aux vars. It serializes them and parses the structure afterwards.
fn main() {
    let builder = InitialLinuxLibcStackLayoutBuilder::new()
        .add_arg_v(b"./first_arg\0")
        .add_arg_v(b"./second_arg\0")
        .add_env_v(b"FOO=BAR\0")
        .add_env_v(b"PATH=/bin\0")
        .add_aux_v(AuxVar::ReferencedData(
            AuxVarType::AtExecFn,
            b"./my_executable\0",
        ))
        .add_aux_v(AuxVar::Value(AuxVarType::AtClktck, 1337))
        .add_aux_v(AuxVar::ReferencedData(
            AuxVarType::AtRandom,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        ));
    let mut buf = vec![0; builder.total_size()];

    // user base addr is the initial stack pointer in the user address space
    let user_base_addr = buf.as_ptr() as u64;
    unsafe {
        builder.serialize_into_buf(buf.as_mut_slice(), user_base_addr);
    }

    let parsed = InitialLinuxLibcStackLayout::from(buf.as_slice());

    println!("There are {} arguments:", parsed.argc());
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (arg_ptr, arg_val) in parsed.argv_ptr_iter().zip(parsed.argv_iter()) {
        println!("  {:?}: {}", arg_ptr, arg_val);
    }
    println!(
        "There are {} environment variables:",
        parsed.envv_ptr_iter().count()
    );
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (env_ptr, env_val) in parsed.envv_ptr_iter().zip(parsed.envv_iter()) {
        println!("  {:?}: {}", env_ptr, env_val);
    }

    println!(
        "There are {} auxiliary vector entries/AT variables:",
        parsed.aux_iter().count()
    );
    // will segfault, if user_ptr != write_ptr (i.e. other address space)
    for aux in parsed.aux_iter() {
        if unsafe { aux.data().is_some() } {
            if aux.key() == AuxVarType::AtRandom {
                println!(
                    "  {:>12?} => {:?}: {:?}",
                    aux.key(),
                    aux.val() as *const u8,
                    unsafe { aux.data().unwrap() }
                );
            } else {
                println!(
                    "  {:>12?} => {:?}: {}",
                    aux.key(),
                    aux.val() as *const u8,
                    unsafe { aux.c_str().unwrap() }
                );
            }
        } else {
            println!("  {:>12?} => {}", aux.key(), aux.val());
        }
    }
}
