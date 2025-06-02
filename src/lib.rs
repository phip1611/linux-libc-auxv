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

//! # linux-libc-auxv
//!
//! A parser and builder for the Linux process initial stack layout - use it to
//! decode or construct `argc`, `argv`, `envp`/`envv`, and `auxv` (auxiliary
//! vector).
//!
//! **Keywords**: crt0, stack layout, AT values, AT pairs, auxvec, auxiliary vector
//!
//! ## Terminology
//!
//! I use `argv`, `envv`, and `auxv` as names for the null-terminated arrays
//! (vectors) of the corresponding data. They have corresponding "data areas" where
//! pointers point to: _argv data area_, _envp data area_, and _auxv data area_.
//!
//! ## About the Stack Layout
//!
//! Linux passes an initial stack layout to applications that contains `argc`,
//! `argv`, `envp`/`envv`, and the `auxiliary vector`. Normal applications don't
//! see this as the libc (crt0 component) abstracts this away. For more low-level
//! developers and kernel hackers creating and parsing this feature becomes
//! relevant.
//!
//!
//! This crate has been tested successfully by myself in a custom runtime system for
//! a Microkernel which loads and starts unmodified Linux binaries. The Linux binary
//! (the libc) was able to find all arguments, environment variables, and the data
//! from the auxiliary vector. Everything was printed properly to stdout.
//!
//! ### Layout Structure
//!
//! The following figure shows the technical details of the layout. The figure is
//! taken from <https://lwn.net/Articles/631631/>. As the structure needs to
//! know the pointers beforehand, creating the structure is not trivial. The
//! builder helps to construct this complex binary structure with references
//! into itself.
//!
//! ```text
//!    ------------------------------------------------------------- 0x7fff6c845000
//!     0x7fff6c844ff8: 0x0000000000000000
//!            _  4fec: './stackdump\0'                      <------+
//!      env  /   4fe2: 'ENVVAR2=2\0'                               |    <----+
//!           \_  4fd8: 'ENVVAR1=1\0'                               |   <---+ |
//!           /   4fd4: 'two\0'                                     |       | |     <----+
//!     args |    4fd0: 'one\0'                                     |       | |    <---+ |
//!           \_  4fcb: 'zero\0'                                    |       | |   <--+ | |
//!               3020: random gap padded to 16B boundary           |       | |      | | |
//!    - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -|       | |      | | |
//!               3019: 'x86_64\0'                        <-+       |       | |      | | |
//!     auxv      3009: random data: ed99b6...2adcc7        | <-+   |       | |      | | |
//!     data      3000: zero padding to align stack         |   |   |       | |      | | |
//!    . . . . . . . . . . . . . . . . . . . . . . . . . . .|. .|. .|       | |      | | |
//!               2ff0: AT_NULL(0)=0                        |   |   |       | |      | | |
//!               2fe0: AT_PLATFORM(15)=0x7fff6c843019    --+   |   |       | |      | | |
//!               2fd0: AT_EXECFN(31)=0x7fff6c844fec      ------|---+       | |      | | |
//!               2fc0: AT_RANDOM(25)=0x7fff6c843009      ------+           | |      | | |
//!      ELF      2fb0: AT_SECURE(23)=0                                     | |      | | |
//!    auxiliary  2fa0: AT_EGID(14)=1000                                    | |      | | |
//!     vector:   2f90: AT_GID(13)=1000                                     | |      | | |
//!    (id,val)   2f80: AT_EUID(12)=1000                                    | |      | | |
//!      pairs    2f70: AT_UID(11)=1000                                     | |      | | |
//!               2f60: AT_ENTRY(9)=0x4010c0                                | |      | | |
//!               2f50: AT_FLAGS(8)=0                                       | |      | | |
//!               2f40: AT_BASE(7)=0x7ff6c1122000                           | |      | | |
//!               2f30: AT_PHNUM(5)=9                                       | |      | | |
//!               2f20: AT_PHENT(4)=56                                      | |      | | |
//!               2f10: AT_PHDR(3)=0x400040                                 | |      | | |
//!               2f00: AT_CLKTCK(17)=100                                   | |      | | |
//!               2ef0: AT_PAGESZ(6)=4096                                   | |      | | |
//!               2ee0: AT_HWCAP(16)=0xbfebfbff                             | |      | | |
//!               2ed0: AT_SYSINFO_EHDR(33)=0x7fff6c86b000                  | |      | | |
//!    . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .        | |      | | |
//!               2ec8: environ[2]=(nil)                                    | |      | | |
//!               2ec0: environ[1]=0x7fff6c844fe2         ------------------|-+      | | |
//!               2eb8: environ[0]=0x7fff6c844fd8         ------------------+        | | |
//!               2eb0: argv[3]=(nil)                                                | | |
//!               2ea8: argv[2]=0x7fff6c844fd4            ---------------------------|-|-+
//!               2ea0: argv[1]=0x7fff6c844fd0            ---------------------------|-+
//!               2e98: argv[0]=0x7fff6c844fcb            ---------------------------+
//!     0x7fff6c842e90: argc=3
//!```
//!_Credits: <https://lwn.net/Articles/631631/>_
//!
//! ## Differences to Crates [`crt0stack`] and [`auxv`]
//!
//! This crate supports `no_std`-contexts plus **allows construction** of the data
//! structure **for a different address space**, i.e. the address space of a user
//! application.
//!
//! [`crt0stack`]: https://crates.io/crates/crt0stack
//! [`auxv`]: https://crates.io/crates/auxv
//!
//! ## Functionality
//!
//! ✅ build data structure for current address space \
//! ✅ build data structure for **different address space** \
//! ✅ parse data structure for current address space + output referenced data/pointers \
//! ✅ parse data structure for **different address space** + prevent memory error / no dereferencing of pointers
//!
//!
//!
//! ## MSRV
//! 1.85.0 stable
//!
//! ## Background Information & Links
//! - <https://lwn.net/Articles/631631/> (good overview with ASCII graphics)
//! - <https://lwn.net/Articles/519085/>
//! - <https://elixir.bootlin.com/linux/v5.15.5/source/fs/binfmt_elf.c#L257> (code in Linux that constructs `auxv`)
//! - <https://man7.org/linux/man-pages/man3/getauxval.3.html>
//! - <https://refspecs.linuxfoundation.org/ELF/zSeries/lzsabi0_zSeries/x895.html>

#![deny(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::must_use_candidate,
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
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(rustdoc::all)]
#![no_std]

#[cfg_attr(feature = "alloc", macro_use)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg_attr(test, macro_use)]
#[cfg(test)]
extern crate std;

pub use aux_var::{AuxVar, AuxVarFlags, AuxVarRaw, AuxVarType};
#[cfg(feature = "builder")]
pub use builder::StackLayoutBuilder;
pub use parser::StackLayoutRef;

mod aux_var;
#[cfg(feature = "builder")]
mod builder;
mod parser;
mod util;
