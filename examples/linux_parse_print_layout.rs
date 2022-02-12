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
#![no_start]
#![feature(start)]

use linux_libc_auxv::{AuxVar, InitialLinuxLibcStackLayout, InitialLinuxLibcStackLayoutBuilder};
use std::mem::size_of;

/// Example that parses the layout and prints it. Only runs on Linux.
#[start]
fn start(_argc: isize, argv: *const *const u8) -> isize {
    let buf = unsafe {
        // the stack layout begins `size_of::<usize>()` bytes before argv (at the address of argc)
        let ptr_layout_begin = argv.cast::<u8>().sub(size_of::<usize>());
        core::slice::from_raw_parts(ptr_layout_begin.cast(), 10000)
    };
    let parsed = InitialLinuxLibcStackLayout::from(buf);

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
    for (i, arg) in unsafe { parsed.argv_iter() }.enumerate() {
        println!("    [{}] @ {:?}", i, arg);
    }

    println!("  envp");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (i, env) in unsafe { parsed.envv_iter() }.enumerate() {
        println!("    [{}] @ {:?}", i, env);
    }

    println!("  aux");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for aux in unsafe { parsed.aux_var_iter() } {
        if aux.key().value_in_data_area() {
            println!("    {:?} => @ {:?}", aux.key(), aux);
        } else {
            println!("    {:?} => {:?}", aux.key(), aux);
        }
    }

    0
}
