# linux-libc-auxv: Build and Parse the Initial Linux Stack Layout for Different Address Spaces

Linux passes an initial stack layout to applications, that contains `argc`, `argv`, `envp`, and the `auxiliary vector`
right above the stack pointer. The libc of a Linux program parses this structure in its `_start`-symbol ("crt0") and
passes the right pointers as arguments to `main` afterwards. This crate helps to construct and parse this data structure
in `no_std` environments and for different address spaces.

**Keywords**: crt0, stack layout, AT values, AT pairs, auxvec, auxiliary vector

This crate has been tested successfully by myself in a custom runtime system for a Microkernel, that is able to load
and start unmodified Linux binaries. The Linux binary (the libc) could find all arguments,
environment variables, and the data from the auxiliary vector and print it to stdout.

## How does this differ from [`crt0stack`] and [`auxv`]?
This crate supports `no_std`-contexts plus allows construction the data structure for a different address
space, i.e. the address space of a user application.

When I started creating this crate, I only knew about the latter. It doesn't support `no_std`. Because
the first one supports `no_std` but not different address spaces, I still had to create this one.
The typical use case for me is to create the data structure for a different address space, like Linux does.

Last but not least, my crate supports more/all of Linux's AT variables.

[`crt0stack`]: https://crates.io/crates/crt0stack
[`auxv`]: https://crates.io/crates/auxv

## Functionality
✅ build data structure for current address space \
✅ build data structure for **different address space** \
✅ parse data structure for current address space + output referenced data/pointers \
✅ parse data structure for **different address space** + prevent memory error / no dereferencing of pointers


## Limitations

### 32 vs 64 bit
The auxiliary vector contains pairs of type `(usize, usize)`. Hence, each entry takes 8 bytes on 32-bit systems
and 16 byte on 64-bit systems. Currently, this crate produces the auxiliary vector for the architecture it is
compiled with. If necessary, create an issue or a PR and this will be a runtime setting. I never tested it
on a 32-bit system, but I am confident it will work.

### Auxiliary Vector vs Stack Layout
Right now, this crate can only build and serialize the whole initial stack layout but not the auxiliary vector
standalone.

## Code Example
There are multiple code examples in the repository!
`cargo run --example linux_parse_print_layout` shows you a
real world example.

### Minimal: Build + Parse
```rust
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
```

### Code Example Output
```text
There are 2 arguments.
There are 2 environment variables.
There are 5 auxiliary vector entries/AT variables.
  argv
    [0] @ 0x7fff00b0
    [1] @ 0x7fff00bc
  envp
    [0] @ 0x7fff00c9
    [1] @ 0x7fff00d1
  aux
    Platform => @ 0x7fff0090
    Clktck => 0x64
    Random => @ 0x7fff0097
    ExecFn => @ 0x7fff00db
    Null => 0x0
```

## Terminology (in Code)
The whole data structure is called `InitialLinuxLibcStackLayout` by me. There is no official name. It contains
the arguments (`argc` and `argv`), the environment variables (`envp` or `envv`), and the auxiliary vector
(`AT-variables`, `auxv`, `aux-pairs`, `aux entries`).

The `argv`-array will reference data in the `argv data area`, the `envv`-array will reference data in the
`envv data area`, and some of the `auxv`-values might reference data in the `auxv data area`.

Sometimes (in some articles), the auxiliary vector even describes the whole data structure.

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
1.85.0 stable

## Background Information & Links
- <https://lwn.net/Articles/631631/> (good overview with ASCII graphics)
- <https://lwn.net/Articles/519085/>
- <https://elixir.bootlin.com/linux/v5.15.5/source/fs/binfmt_elf.c#L257> (code in Linux that constructs `auxv`)
- <https://man7.org/linux/man-pages/man3/getauxval.3.html>
- <https://refspecs.linuxfoundation.org/ELF/zSeries/lzsabi0_zSeries/x895.html>
