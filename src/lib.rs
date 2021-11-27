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
//! # libc-auxv - Build and Parse the Initial Linux Stack Layout for Different Address Spaces
//!
//! Linux passes an initial stack layout to applications, that contains `argc`, `argv`, `envp`, and the `auxiliary vector`
//! right above the stack pointer. The libc of a Linux program parses this sturcture in its `_start`-symbol ("crt0") and
//! passes the right pointers as arguments to `main` afterwards. This crate helps to construct and parse this data structure
//! in `no_std` environments and for different address spaces.
//!
//! ## How does this differ from <https://crates.io/crates/crt0stack> and <https://crates.io/crates/auxv>?
//! This crate supports `no_std`-contexts plus allows construction the data structure for a different address
//! space, i.e. the address space of a user application.
//!
//! When I started creating this crate, I only knew about the latter. It doesn't support `no_std`. Because
//! the first one supports `no_std` but not different address spaces, I still had to create this one.
//! The typical use case for me is to create the data structure for a different address space, like Linux does.
//!
//! ## Functionality
//! ✅ build data structure for current address space \
//! ✅ build data structure for **different address space** \
//! ✅ parse data structure for current address space + output referenced data/pointers \
//! ✅ parse data structure for **different address space** + prevent memory error / no dereferencing of pointers
//!
//! ## Code Example
//! ```rust
//! // Minimal example that builds the initial Linux libc stack layout. It includes args, envvs,
//! // and aux vars. It serializes them and parses the structure afterwards.
//!
//! use linux_libc_auxv::{
//!     AuxVar, AuxVarType, InitialLinuxLibcStackLayout, InitialLinuxLibcStackLayoutBuilder,
//! };
//!
//! let builder = InitialLinuxLibcStackLayoutBuilder::new()
//!     .add_arg_v(b"./first_arg\0")
//!     .add_arg_v(b"./second_arg\0")
//!     .add_env_v(b"FOO=BAR\0")
//!     .add_env_v(b"PATH=/bin\0")
//!     .add_aux_v(AuxVar::ReferencedData(AuxVarType::AtExecFn, b"./my_executable\0"))
//!     .add_aux_v(AuxVar::Value(AuxVarType::AtClktck, 1337))
//!     .add_aux_v(AuxVar::ReferencedData(AuxVarType::AtRandom, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]));
//! let mut buf = vec![0; builder.total_size()];
//!
//! // user base addr is the initial stack pointer in the user address space
//! let user_base_addr = buf.as_ptr() as u64;
//! unsafe {
//!     builder.serialize_into_buf(buf.as_mut_slice(), user_base_addr);
//! }
//!
//! let parsed = InitialLinuxLibcStackLayout::from(buf.as_slice());
//! dbg!(parsed.argc());
//! // ptr iter is safe for other address spaces
//! dbg!(parsed.argv_ptr_iter().collect::<Vec<_>>());
//! dbg!(parsed.argv_iter().collect::<Vec<_>>());
//! // ptr iter is safe for other address spaces
//! dbg!(parsed.envv_ptr_iter().collect::<Vec<_>>());
//! dbg!(parsed.envv_iter().collect::<Vec<_>>());
//!
//! // will segfault, if user_ptr != write_ptr (i.e. other address space)
//! dbg!(parsed.aux_iter().collect::<Vec<_>>());
//! ```
//!
//! ## Terminology (in Code)
//! The whole data structure is called `InitialLinuxLibcStackLayout` by me. There is no official name. It contains
//! the arguments (`argc` and `argv`), the environment variables (`envp` or `envv`), and the auxiliary vector
//! (`AT-variables`, `auxv`, `aux-pairs`, `aux entries`).
//!
//! The `argv`-array will reference data in the `argv data area`, the `envv`-array will reference data in the
//! `envv data area`, and some of the `auxv`-values might reference data in the `auxv data area`.
//!
//! ## Layout of the Data Structure
//! ```text
//! null                                   [HIGH ADDRESS]
//! filename (c string)
//! <env data area>
//! <args data area>
//! // round up to 16 byte
//! <aux vec data area>
//! // round up to 16 byte alignment
//! AT_VAR_3 = <points to aux vec data area>
//! AT_VAR_2 = integer
//! AT_VAR_1 = integer
//! // round up to 16 byte alignment
//! envv[2] = null
//! envv[1] = <points to env data area>
//! envv[0] = <points to env data area>
//! argv[2] = null
//! argv[1] = <points to args data area>
//! argv[0] = <points to args data area>
//! argc = integer <libc entry stack top>  [LOW ADDRESS]
//! ```
//!
//! ## MSRV
//! 1.56.1 stable / Rust edition 2021
//!
//! ## Background Information & Links
//! - <https://lwn.net/Articles/631631/> (good overview with ASCII graphics)
//! - <https://lwn.net/Articles/519085/>
//! - <https://elixir.bootlin.com/linux/v5.15.5/source/fs/binfmt_elf.c#L257> (code in Linux that constructs `auxv`)

#![deny(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    // clippy::restriction,
    // clippy::pedantic
)]
// now allow a few rules which are denied by the above statement
// --> they are ridiculous and not necessary
#![allow(
    clippy::suboptimal_flops,
    clippy::redundant_pub_crate,
    clippy::fallible_impl_from
)]
#![deny(missing_debug_implementations)]
#![deny(rustdoc::all)]
#![no_std]

mod aux_vars;
mod builder;
mod parser;

pub use aux_vars::*;
pub use builder::*;
pub use parser::*;

#[macro_use]
extern crate alloc;

#[cfg_attr(test, macro_use)]
#[cfg(test)]
extern crate std;
