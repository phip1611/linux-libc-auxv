# libc-auxv - Build and Parse the Initial Linux Stack Layout for Different Address Spaces

Linux passes an initial stack layout to applications, that contains `argc`, `argv`, `envp`, and the `auxiliary vector`
right above the stack pointer. The libc of a Linux program parses this sturcture in its `_start`-symbol ("crt0") and
passes the right pointers as arguments to `main` afterwards. This crate helps to construct and parse this data structure
in `no_std` environments and for different address spaces.

**Keywords**: crt0, stack layout, AT values, AT pairs, auxvec, auxiliary vector

This crate has been tested successfully by myself in a custom runtime system for a Microkernel, that is able to load
and start unmodified Linux binaries. The Linux binary (the libc) could find all arguments,
environment variables, and the data from the auxiliary vector and print it to stdout.

## How does this differ from <https://crates.io/crates/crt0stack> and <https://crates.io/crates/auxv>?
This crate supports `no_std`-contexts plus allows construction the data structure for a different address
space, i.e. the address space of a user application.

When I started creating this crate, I only knew about the latter. It doesn't support `no_std`. Because
the first one supports `no_std` but not different address spaces, I still had to create this one.
The typical use case for me is to create the data structure for a different address space, like Linux does.

## Functionality
✅ build data structure for current address space \
✅ build data structure for **different address space** \
✅ parse data structure for current address space + output referenced data/pointers \
✅ parse data structure for **different address space** + prevent memory error / no dereferencing of pointers


## Limitations

### 32 vs 64 bit
The auxiliary vector contains pairs of type `(usize, usize)`. Hence, each entry takes 8 bytes on 32-bit systems
and 16 byte on 64-bit systems. Currently, this crate produces the auxiliary vector for the architecture it is
compiled with. If necessary, create an issue or a PR and this will be a runtime setting.

### Auxiliary Vector vs Stack Layout
Right now, this crate can only build and serialize the whole initial stack layout but not the auxiliary vector
standalone.

## Code Example
```rust
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
        .add_aux_v(AuxVar::ReferencedData(AuxVarType::AtExecFn, b"./my_executable\0"))
        .add_aux_v(AuxVar::Value(AuxVarType::AtClktck, 1337))
        .add_aux_v(AuxVar::ReferencedData(AuxVarType::AtRandom, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]));
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
    println!("There are {} environment variables:", parsed.envv_ptr_iter().count());
    // ptr iter is safe for other address spaces; the other only because here user_addr == write_addr
    for (env_ptr, env_val) in parsed.envv_ptr_iter().zip(parsed.envv_iter()) {
        println!("  {:?}: {}", env_ptr, env_val);
    }

    println!("There are {} auxiliary vector entries/AT variables:", parsed.aux_iter().count());
    // will segfault, if user_ptr != write_ptr (i.e. other address space)
    for aux in parsed.aux_iter() {
        if unsafe { aux.data().is_some() } {
            if aux.key() == AuxVarType::AtRandom {
                println!("  {:>12?} => {:?}: {:?}", aux.key(), aux.val() as *const u8, unsafe { aux.data().unwrap() });
            } else {
                println!("  {:>12?} => {:?}: {}", aux.key(), aux.val() as *const u8, unsafe { aux.c_str().unwrap() });
            }
        } else {
            println!("  {:>12?} => {}", aux.key(), aux.val());
        }
    }
}
```

## Terminology (in Code)
The whole data structure is called `InitialLinuxLibcStackLayout` by me. There is no official name. It contains
the arguments (`argc` and `argv`), the environment variables (`envp` or `envv`), and the auxiliary vector
(`AT-variables`, `auxv`, `aux-pairs`, `aux entries`).

The `argv`-array will reference data in the `argv data area`, the `envv`-array will reference data in the
`envv data area`, and some of the `auxv`-values might reference data in the `auxv data area`.

## Layout of the Data Structure
```text
null                                   [HIGH ADDRESS]
filename (c string)
<env data area>
<args data area>
// round up to 16 byte
<aux vec data area>
// round up to 16 byte alignment
AT_VAR_3 = <points to aux vec data area>
AT_VAR_2 = integer
AT_VAR_1 = integer
// round up to 16 byte alignment
envv[2] = null
envv[1] = <points to env data area>
envv[0] = <points to env data area>
argv[2] = null
argv[1] = <points to args data area>
argv[0] = <points to args data area>
argc = integer <libc entry stack top>  [LOW ADDRESS]
```

## MSRV
1.56.1 stable / Rust edition 2021

## Background Information & Links
- <https://lwn.net/Articles/631631/> (good overview with ASCII graphics)
- <https://lwn.net/Articles/519085/>
- <https://elixir.bootlin.com/linux/v5.15.5/source/fs/binfmt_elf.c#L257> (code in Linux that constructs `auxv`)
- <https://man7.org/linux/man-pages/man3/getauxval.3.html>
- <https://refspecs.linuxfoundation.org/ELF/zSeries/lzsabi0_zSeries/x895.html>
