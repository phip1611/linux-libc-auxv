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
use linux_libc_auxv::{AuxVar, StackLayoutBuilder, StackLayoutRef};

/// Minimal example building a stack layout and parsing it.
fn main() {
    let builder = StackLayoutBuilder::new()
        // can contain terminating zero; not mandatory in the builder
        .add_argv("foo")
        .add_argv("hello")
        .add_envv("PATH=/bin")
        .add_auxv(AuxVar::ExecFn("/usr/bin/foo".into()));

    let layout = builder.build(None /* we create the layout in our address space */);
    let layout = StackLayoutRef::new(layout.as_ref(), None);

    // SAFETY: This is safe as all pointers point into our address space.
    for (i, arg) in unsafe { layout.argv_iter() }.enumerate() {
        println!("  [{i}] {}", arg.to_str().unwrap());
    }
}
