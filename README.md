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

Last but not least, my crate supports more/all of Linux's AT tags.

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
use linux_libc_auxv::{
    AuxVar, AuxVarFlags, InitialLinuxLibcStackLayout, InitialLinuxLibcStackLayoutBuilder,
};

/// Minimal example that builds the initial Linux libc stack layout. It includes args, envvs,
/// and aux vars. It serializes them and parses the structure afterwards.
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
        // currently: Only AT_RANDOM
        if let Some(bytes) = aux.value_payload_bytes() {
            println!(
                "  {:>12?} => @ {:?}: {:?}",
                aux.key(),
                aux.value_raw() as *const u8,
                bytes,
            );
        } else if let Some(cstr) = aux.value_payload_cstr() {
            println!(
                "  {:>12?} => @ {:?}: {:?}",
                aux.key(),
                aux.value_raw() as *const u8,
                cstr,
            );
        } else if let Some(flags) = aux.value_flags() {
            println!(
                "  {:>12?} => {:?}",
                aux.key(),
                flags,
            );
        } else if let Some(boolean) = aux.value_boolean() {
            println!(
                "  {:>12?} => {:?}",
                aux.key(),
                boolean,
            );
        } else if let Some(ptr) = aux.value_ptr() {
            println!(
                "  {:>12?} => {:?}",
                aux.key(),
                ptr,
            );
        }
        else {
            println!("  {:>12?} => {}", aux.key(), aux.value_raw());
        }
    }
}
```

### Code Example Output
```text
There are 2 arguments:
  0x559e0f07ad80: ./first_arg
  0x559e0f07ad8c: ./second_arg
There are 2 environment variables:
  0x559e0f07ad99: FOO=BAR
  0x559e0f07ada1: PATH=/bin
There are 4 auxiliary vector entries/AT variables:
  AtClktck => 1337
  AtRandom => 0x559e0f07ad70: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
  AtExecFn => 0x559e0f07adab: ./my_executable
  AtNull => 0
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
