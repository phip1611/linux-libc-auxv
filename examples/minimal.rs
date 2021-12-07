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
use linux_libc_auxv::{AuxVar, InitialLinuxLibcStackLayout, InitialLinuxLibcStackLayoutBuilder};

/// Minimal example that builds the initial linux libc stack layout and parses it again.
fn main() {
    let builder = InitialLinuxLibcStackLayoutBuilder::new()
        // can contain terminating zero; not mandatory in the builder
        .add_arg_v("./first_arg\0")
        .add_arg_v("./second_arg")
        .add_env_v("FOO=BAR\0")
        .add_env_v("PATH=/bin")
        .add_aux_v(AuxVar::Clktck(100))
        .add_aux_v(AuxVar::Random([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ]))
        .add_aux_v(AuxVar::ExecFn("/usr/bin/foo"))
        .add_aux_v(AuxVar::Platform("x86_64"));

    // memory where we serialize the data structure into
    let mut buf = vec![0; builder.total_size()];

    // assume user stack is at 0x7fff0000
    let user_base_addr = 0x7fff0000;
    unsafe {
        builder.serialize_into_buf(buf.as_mut_slice(), user_base_addr);
    }

    // So far, this is memory safe, as long as the slice is valid memory. No pointers are
    // dereferenced yet.
    let parsed = InitialLinuxLibcStackLayout::from(buf.as_slice());

    println!("There are {} arguments.", parsed.argc());
    println!(
        "There are {} environment variables.",
        parsed.envv_ptr_iter().count()
    );
    println!(
        "There are {} auxiliary vector entries/AT variables.",
        parsed.aux_serialized_iter().count()
    );

    println!("  argv");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (i, arg) in parsed.argv_ptr_iter().enumerate() {
        println!("    [{}] @ {:?}", i, arg);
    }

    println!("  envp");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (i, env) in parsed.envv_ptr_iter().enumerate() {
        println!("    [{}] @ {:?}", i, env);
    }

    println!("  aux");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for aux in parsed.aux_serialized_iter() {
        if aux.key().value_in_data_area() {
            println!("    {:?} => @ {:?}", aux.key(), aux.val() as *const u8);
        } else {
            println!("    {:?} => {:?}", aux.key(), aux.val() as *const u8);
        }
    }
}
