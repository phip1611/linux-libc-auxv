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
#![no_main]

use linux_libc_auxv::StackLayoutRef;
use std::slice;

/// Example that parses the layout and prints it. Only runs on Linux.
#[unsafe(no_mangle)]
fn main(argc: isize, argv: *const *const u8) -> isize {
    let buffer = unsafe {
        // 100 KiB, reasonably big.
        // On my Linux machine, the structure needs 23 KiB
        slice::from_raw_parts(argv.cast::<u8>(), 0x19000)
    };

    let parsed = StackLayoutRef::new(buffer, Some(argc as usize));

    println!("There are {} arguments.", parsed.argc());
    println!("  argv (raw)");
    for (i, arg) in parsed.argv_raw_iter().enumerate() {
        println!("    [{i}] @ {arg:?}");
    }
    println!("  argv");
    // SAFETY: The pointers are valid in the address space of this process.
    for (i, arg) in unsafe { parsed.argv_iter() }.enumerate() {
        println!("    [{i}] {arg:?}");
    }

    println!("There are {} environment variables.", parsed.envc());
    println!("  envv (raw)");
    for (i, env) in parsed.envv_raw_iter().enumerate() {
        println!("    [{i}] {env:?}");
    }
    println!("  envv");
    // SAFETY: The pointers are valid in the address space of this process.
    for (i, env) in unsafe { parsed.envv_iter() }.enumerate() {
        println!("    [{i}] {env:?}");
    }

    println!(
        "There are {} auxiliary vector entries/AT variables.",
        parsed.auxv_raw_iter().count()
    );
    println!("  aux");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for aux in unsafe { parsed.auxv_iter() } {
        if aux.key().value_in_data_area() {
            println!("    {:?} => @ {:?}", aux.key(), aux);
        } else {
            println!("    {:?} => {:?}", aux.key(), aux);
        }
    }

    0
}
