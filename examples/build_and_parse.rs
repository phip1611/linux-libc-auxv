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
    AuxVar, AuxVarFlags, InitialLinuxLibcStackLayout, InitialLinuxLibcStackLayoutBuilder,
};

/// Example that builds the initial linux libc stack layout and parses it again.
fn main() {
    let builder = InitialLinuxLibcStackLayoutBuilder::new()
        // can contain terminating zero; not mandatory in the builder
        .add_arg_v("./first_arg\0")
        .add_arg_v("./second_arg")
        .add_env_v("FOO=BAR\0")
        .add_env_v("PATH=/bin")
        .add_aux_v(AuxVar::Sysinfo(0x7ffd963e9000 as *const _))
        .add_aux_v(AuxVar::HwCap(0x1000))
        .add_aux_v(AuxVar::Clktck(100))
        .add_aux_v(AuxVar::Phdr(0x5627e17a6040 as *const _))
        .add_aux_v(AuxVar::Phent(56))
        .add_aux_v(AuxVar::Phnum(13))
        .add_aux_v(AuxVar::Base(0x7f51b886e000 as *const _))
        .add_aux_v(AuxVar::Flags(AuxVarFlags::empty()))
        .add_aux_v(AuxVar::Entry(0x5627e17a8850 as *const _))
        .add_aux_v(AuxVar::Uid(1001))
        .add_aux_v(AuxVar::EUid(1001))
        .add_aux_v(AuxVar::Gid(1001))
        .add_aux_v(AuxVar::EGid(1001))
        .add_aux_v(AuxVar::Secure(false))
        .add_aux_v(AuxVar::Random([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ]))
        .add_aux_v(AuxVar::HwCap2(0x2))
        .add_aux_v(AuxVar::ExecFn("/usr/bin/foo"))
        .add_aux_v(AuxVar::Platform("x86_64"));

    // memory where we serialize the data structure into
    let mut buf = vec![0; builder.total_size()];

    // User base addr is the initial stack pointer in the user address space.
    // In this example: same as write address => enables us to parse the data structure
    // let user_base_addr = buf.as_ptr() as u64;
    let user_base_addr = 0x1000;
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

    println!("===== 1/2: only pointers");
    parse_memory_safe(&parsed);

    if user_base_addr == buf.as_ptr() as u64 {
        println!("===== 2/2: dereferenced data");
        // this will not work, if you change the "user_base_addr" above to another address
        unsafe {
            parse_memory_unsafe(&parsed);
        }
    }
}

fn parse_memory_safe(parsed: &InitialLinuxLibcStackLayout) {
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
        println!("    {:?} => {:?}", aux.key(), aux.val() as *const u8);
    }
}

// will segfault/page fault or read invalid memory, if user_ptr != write_ptr
// (i.e. other address space)
unsafe fn parse_memory_unsafe(parsed: &InitialLinuxLibcStackLayout) {
    println!("  argv");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (i, arg) in parsed.argv_iter().enumerate() {
        println!("    [{}] {}", i, arg);
    }

    println!("  envp");
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (i, env) in parsed.envv_iter().enumerate() {
        println!("    [{}] {}", i, env);
    }

    println!("  aux");
    for aux in parsed.aux_var_iter() {
        // currently: Only AT_RANDOM
        if let Some(bytes) = aux.value_payload_bytes() {
            println!(
                "    {:>12?} => @ {:?}: {:?}",
                aux.key(),
                aux.value_raw() as *const u8,
                bytes,
            );
        } else if let Some(cstr) = aux.value_payload_cstr() {
            println!(
                "    {:>12?} => @ {:?}: {:?}",
                aux.key(),
                aux.value_raw() as *const u8,
                cstr,
            );
        } else if let Some(flags) = aux.value_flags() {
            println!("    {:>12?} => {:?}", aux.key(), flags,);
        } else if let Some(boolean) = aux.value_boolean() {
            println!("    {:>12?} => {:?}", aux.key(), boolean,);
        } else if let Some(ptr) = aux.value_ptr() {
            println!("    {:>12?} => {:?}", aux.key(), ptr,);
        } else {
            println!("    {:>12?} => {}", aux.key(), aux.value_raw());
        }
    }
}
