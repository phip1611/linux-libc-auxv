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
use crate::aux_var::{AuxVar, AuxVarRaw, AuxVarType};
use crate::util::count_bytes_until_null;
use core::ffi::CStr;
use core::fmt::Debug;

/// Wraps a slice of bytes representing a Linux stack layout allowing to
/// conveniently parse its content.
///
/// The stack layout under Linux contains `argc`, `argv`, `envv`/`envp`, and the
/// auxiliary vector, all with the additional referenced payloads. More
/// precisely, the structure contains data in the following order:
/// - `argc`: Amount of arguments
/// - `argv`: null-terminated array of pointers into _argv data area_
/// - `NULL pointer`
/// - `envv`: null-terminated array of pointers into _envv data area_
/// - `NULL pointer`
/// - `auxv` Array of auxiliary variables (AT variables), terminated by an
///   [`AuxVarType::Null`] entry.
/// - `NUL[s] for padding`
/// - `auxv data area`: Possibly payload of auxiliary variables
/// - `argv data area`: Null-terminated C strings representing the arguments
/// - `envv data area`: Null-terminated C strings representing the environment
/// - `NUL[s] for padding`
///
/// The parsing code will determine at runtime how long the data actually is,
/// therefore, it is recommended to pass in a slice that is long enough to
/// hold the stack layout. For example, passing in a 1 MiB slice is perfectly
/// fine.
///
/// # Safety
/// Each function that loads data from one of the
///
/// ## More Info
/// - <See <https://lwn.net/Articles/631631/>>
#[derive(Debug)]
pub struct StackLayoutRef<'a> {
    // Might cover more data than the actual content of the stack layout.
    bytes: &'a [u8],
    argc: Option<usize>,
}

impl<'a> StackLayoutRef<'a> {
    /// Creates a new view into the stack layout.
    ///
    /// The `argc` determines whether `bytes` start with the `argc` argument
    /// (=> `None`) or if `bytes` already point to the start of `argv`.
    pub fn new(bytes: &'a [u8], argc: Option<usize>) -> Self {
        assert_eq!(bytes.as_ptr().align_offset(align_of::<usize>()), 0);
        Self { bytes, argc }
    }

    // ========== BEGIN buffer get functions ==========

    /// Returns a view into the underlying buffer where the Argument Vector
    /// (`argv`) begins. The slice ends at the end of the structure.
    ///
    /// This enables parsing the data until the end of that area is found.
    fn get_slice_argv(&self) -> &'a [u8] {
        match self.argc {
            None => {
                let start = size_of::<usize>();
                // We skip the `argc` argument
                &self.bytes[start..]
            }
            Some(_) => self.bytes,
        }
    }

    /// Returns a view into the underlying buffer where the Environmental
    /// Variable Vector (`envv`) begins. The slice ends at the end of the
    /// structure.
    ///
    /// This enables parsing the data until the end of that area is found.
    fn get_slice_envv(&self) -> &'a [u8] {
        // envv starts after argv
        let base_slice = self.get_slice_argv();

        let start = self.argc() * size_of::<usize>() + size_of::<usize>() /* NUL */;
        &base_slice[start..]
    }

    /// Returns a view into the underlying buffer where the Auxiliary Vector
    /// (`auxv`) begins. The slice ends at the end of the structure.
    ///
    /// This enables parsing the data until the end of that area is found.
    fn get_slice_auxv(&self) -> &'a [u8] {
        // auxv starts after envv
        let base_slice = self.get_slice_envv();

        // We skip the terminating null ptr after the envv
        let start = self.envc() * size_of::<usize>() + size_of::<usize>() /* NUL */;
        &base_slice[start..]
    }

    // ========== END buffer get functions ==========

    /// Returns the number of arguments.
    pub fn argc(&self) -> usize {
        self.argc.unwrap_or_else(|| unsafe {
            // the first `usize` is the `argc` argument
            self.bytes
                .as_ptr()
                .cast::<usize>()
                .as_ref()
                .copied()
                .unwrap()
        })
    }

    /// Returns the number of environment variables.
    pub fn envc(&self) -> usize {
        self.envv_raw_iter().count()
    }

    /// Returns the number of auxiliary vector entries.
    pub fn auxvc(&self) -> usize {
        self.auxv_raw_iter().count()
    }

    /// Returns an iterator over the raw argument vector's (`argv`)
    /// [`CStr`] pointers.
    ///
    /// # Safety
    /// The pointers must point to valid memory. If dereferenced, the memory
    /// **must** be in the address space of the application. Otherwise,
    /// segmentation faults or UB will occur.
    pub fn argv_raw_iter(&self) -> impl Iterator<Item = *const u8> {
        let buffer = self.get_slice_argv();
        unsafe { NullTermArrIter::new(buffer) }
    }

    /// Returns an iterator over the raw environment vector's (`envv`)
    /// [`CStr`] pointers.
    ///
    /// # Safety
    /// The pointers must point to valid memory. If dereferenced, the memory
    /// **must** be in the address space of the application. Otherwise,
    /// segmentation faults or UB will occur.
    pub fn envv_raw_iter(&self) -> impl Iterator<Item = *const u8> {
        let buffer = self.get_slice_envv();
        unsafe { NullTermArrIter::new(buffer) }
    }

    /// Returns an iterator over the auxiliary variables vector's (`auxv`)
    /// [`AuxVarRaw`] elements.
    ///
    /// # Safety
    /// Any pointers must point to valid memory. If dereferenced, the memory
    /// **must** be in the address space of the application. Otherwise,
    /// segmentation faults or UB will occur.
    pub fn auxv_raw_iter(&self) -> impl Iterator<Item = AuxVarRaw> {
        AuxVarRawIter::new(self.get_slice_auxv())
    }

    /// Unsafe version of [`Self::argv_raw_iter`] that only works if all pointers
    /// are valid. It emits high-level items of type [`CStr`].
    ///
    /// This is typically safe if you parse the stack layout you've got from
    /// Linux but not if you parse some other's stack layout.
    ///
    /// # Safety
    /// The pointers must point to valid memory. If dereferenced, the memory
    /// **must** be in the address space of the application. Otherwise,
    /// segmentation faults or UB will occur.
    pub unsafe fn argv_iter(&self) -> impl Iterator<Item = &'a CStr> {
        let buffer = self.get_slice_argv();
        unsafe { CStrArrayIter::new(buffer) }
    }
    /// Unsafe version of [`Self::envv_raw_iter`] that only works if all pointers
    /// are valid. It emits high-level items of type [`CStr`].
    ///
    /// This is typically safe if you parse the stack layout you've got from
    /// Linux but not if you parse some other's stack layout.
    ///
    /// # Safety
    /// The pointers must point to valid memory. If dereferenced, the memory
    /// **must** be in the address space of the application. Otherwise,
    /// segmentation faults or UB will occur.
    pub unsafe fn envv_iter(&self) -> impl Iterator<Item = &'a CStr> {
        let buffer = self.get_slice_envv();
        unsafe { CStrArrayIter::new(buffer) }
    }

    /// Unsafe version of [`Self::argv_raw_iter`] that only works if all pointers
    /// are valid. It emits high-level items of type [`AuxVar`].
    ///
    /// This is typically safe if you parse the stack layout you've got from
    /// Linux but not if you parse some other's stack layout.
    ///
    /// # Safety
    /// Any pointers must point to valid memory. If dereferenced, the memory
    /// **must** be in the address space of the application. Otherwise,
    /// segmentation faults or UB will occur.
    pub unsafe fn auxv_iter(&self) -> impl Iterator<Item = AuxVar<'a>> {
        unsafe { AuxVarIter::new(self.get_slice_auxv()) }
    }
}

/// Iterator over the entries of a null-terminated array of pointers.
///
/// This should not be used to read the raw pointer into a [`CStr`], so that
/// Miri can verify all our memory accesses are valid.
#[derive(Debug)]
struct NullTermArrIter<'a> {
    // Buffer holds more bytes than necessary because the size of the auxv
    // array is not known at compile time.
    buffer: &'a [u8],
    i: usize,
}

impl<'a> NullTermArrIter<'a> {
    // SAFETY: If the pointers point to invalid memory, UB will occur.
    unsafe fn new(buffer: &'a [u8]) -> Self {
        assert_eq!(buffer.as_ptr().align_offset(align_of::<usize>()), 0);

        Self { buffer, i: 0 }
    }
}

impl Iterator for NullTermArrIter<'_> {
    type Item = *const u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.buffer.len() {
            panic!("null terminated array ended prematurely");
        }

        let entry_ptr = unsafe {
            self.buffer
                .as_ptr()
                .cast::<*const u8>()
                // skip i pointers
                .add(self.i)
        };
        let entry = unsafe { entry_ptr.as_ref().copied().unwrap() };
        if entry.is_null() {
            return None;
        }

        self.i += 1;
        Some(entry)
    }
}

/// Iterator over the [`CStr`]s of a null-terminated C-style array.
///
/// This should only be used when you know that the memory being referenced is
/// valid. Otherwise, segmentation faults or UB occur.
#[derive(Debug)]
struct CStrArrayIter<'a> {
    // Buffer holds more bytes than necessary because the size of the auxv
    // array is not known at compile time.
    buffer: &'a [u8],
    i: usize,
}

impl<'a> CStrArrayIter<'a> {
    // SAFETY: If the pointers point to invalid memory, UB will occur.
    unsafe fn new(buffer: &'a [u8]) -> Self {
        assert_eq!(buffer.as_ptr().align_offset(align_of::<usize>()), 0);

        Self { buffer, i: 0 }
    }
}

impl<'a> Iterator for CStrArrayIter<'a> {
    type Item = &'a CStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.buffer.len() {
            panic!("null terminated array ended prematurely");
        }

        let entry_ptr = unsafe { self.buffer.as_ptr().cast::<*const u8>().add(self.i) };
        let entry = unsafe { entry_ptr.as_ref().copied().unwrap() };
        if entry.is_null() {
            return None;
        }

        // Assert in range
        {
            let end = &raw const self.buffer[self.buffer.len() - 1];
            assert!(entry > self.buffer.as_ptr());
            assert!(entry <= end);
        }

        // offset of the pointer within the buffer
        let begin_index = entry as usize - self.buffer.as_ptr() as usize;
        let end_index_rel =
            count_bytes_until_null(&self.buffer[begin_index..]).expect("should have NUL byte");
        let end_index = begin_index + end_index_rel + 1 /* NUL byte */;
        let cstr = CStr::from_bytes_with_nul(&self.buffer[begin_index..end_index]).unwrap();

        self.i += 1;
        Some(cstr)
    }
}

/// Iterates over the `auxv` array with dynamic size until the end key is found.
///
/// Emits elements of type [`AuxVarRaw`].
#[derive(Debug)]
pub struct AuxVarRawIter<'a> {
    // Buffer holds more bytes than necessary because the size of the auxv
    // array is not known at compile time.
    auxv: &'a [u8],
    i: usize,
}

impl<'a> AuxVarRawIter<'a> {
    const fn new(auxv: &'a [u8]) -> Self {
        Self { auxv, i: 0 }
    }
}

impl<'a> Iterator for AuxVarRawIter<'a> {
    type Item = AuxVarRaw;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = unsafe {
            let entry_ptr = self.auxv.as_ptr().cast::<AuxVarRaw>().add(self.i);
            entry_ptr.as_ref().unwrap()
        };

        if let Ok(key) = entry.key() {
            if key == AuxVarType::Null {
                None
            } else {
                self.i += 1;
                Some(*entry)
            }
        } else {
            // log error?
            // invalid data, stop
            None
        }
    }
}

/// Iterates the [`AuxVar`]s of the stack layout.
#[derive(Debug)]
pub struct AuxVarIter<'a> {
    // Buffer holds more bytes than necessary because the size of the auxv
    // array is not known at compile time.
    auxv: &'a [u8],
    serialized_iter: AuxVarRawIter<'a>,
}

impl<'a> AuxVarIter<'a> {
    // SAFETY: If the pointers point to invalid memory, UB will occur.
    const unsafe fn new(auxv: &'a [u8]) -> Self {
        Self {
            serialized_iter: AuxVarRawIter::new(auxv),
            auxv,
        }
    }
}

impl<'a> Iterator for AuxVarIter<'a> {
    type Item = AuxVar<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.serialized_iter
                .next()
                .map(|ref x| AuxVar::from_raw(x, self.auxv))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::StackLayoutRef;

    #[repr(C, align(8))]
    struct Aligned8<T>(T);

    impl<T> AsRef<T> for Aligned8<T> {
        fn as_ref(&self) -> &T {
            &self.0
        }
    }

    // Extracted from a Linux application during startup.
    #[cfg(target_arch = "x86_64")]
    const TEST_DATA_X86_64: Aligned8<[u8; 1592]> = Aligned8([
        4, 0, 0, 0, 0, 0, 0, 0, 190, 252, 185, 93, 255, 127, 0, 0, 237, 252, 185, 93, 255, 127, 0,
        0, 243, 252, 185, 93, 255, 127, 0, 0, 250, 252, 185, 93, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 253, 185, 93, 255, 127, 0, 0, 109, 253, 185, 93, 255, 127, 0, 0, 191, 253, 185,
        93, 255, 127, 0, 0, 224, 253, 185, 93, 255, 127, 0, 0, 22, 254, 185, 93, 255, 127, 0, 0,
        88, 254, 185, 93, 255, 127, 0, 0, 144, 254, 185, 93, 255, 127, 0, 0, 70, 0, 186, 93, 255,
        127, 0, 0, 133, 0, 186, 93, 255, 127, 0, 0, 155, 0, 186, 93, 255, 127, 0, 0, 179, 0, 186,
        93, 255, 127, 0, 0, 210, 0, 186, 93, 255, 127, 0, 0, 237, 0, 186, 93, 255, 127, 0, 0, 46,
        1, 186, 93, 255, 127, 0, 0, 76, 1, 186, 93, 255, 127, 0, 0, 100, 1, 186, 93, 255, 127, 0,
        0, 126, 1, 186, 93, 255, 127, 0, 0, 152, 1, 186, 93, 255, 127, 0, 0, 178, 1, 186, 93, 255,
        127, 0, 0, 201, 1, 186, 93, 255, 127, 0, 0, 24, 2, 186, 93, 255, 127, 0, 0, 78, 2, 186, 93,
        255, 127, 0, 0, 100, 2, 186, 93, 255, 127, 0, 0, 122, 2, 186, 93, 255, 127, 0, 0, 133, 2,
        186, 93, 255, 127, 0, 0, 179, 2, 186, 93, 255, 127, 0, 0, 192, 2, 186, 93, 255, 127, 0, 0,
        134, 15, 186, 93, 255, 127, 0, 0, 226, 15, 186, 93, 255, 127, 0, 0, 243, 15, 186, 93, 255,
        127, 0, 0, 8, 16, 186, 93, 255, 127, 0, 0, 216, 16, 186, 93, 255, 127, 0, 0, 119, 18, 186,
        93, 255, 127, 0, 0, 215, 18, 186, 93, 255, 127, 0, 0, 251, 18, 186, 93, 255, 127, 0, 0,
        110, 29, 186, 93, 255, 127, 0, 0, 134, 29, 186, 93, 255, 127, 0, 0, 167, 29, 186, 93, 255,
        127, 0, 0, 190, 29, 186, 93, 255, 127, 0, 0, 33, 31, 186, 93, 255, 127, 0, 0, 244, 33, 186,
        93, 255, 127, 0, 0, 8, 34, 186, 93, 255, 127, 0, 0, 189, 35, 186, 93, 255, 127, 0, 0, 236,
        35, 186, 93, 255, 127, 0, 0, 81, 36, 186, 93, 255, 127, 0, 0, 181, 36, 186, 93, 255, 127,
        0, 0, 37, 37, 186, 93, 255, 127, 0, 0, 60, 37, 186, 93, 255, 127, 0, 0, 77, 37, 186, 93,
        255, 127, 0, 0, 100, 37, 186, 93, 255, 127, 0, 0, 130, 37, 186, 93, 255, 127, 0, 0, 157,
        37, 186, 93, 255, 127, 0, 0, 181, 37, 186, 93, 255, 127, 0, 0, 201, 37, 186, 93, 255, 127,
        0, 0, 224, 37, 186, 93, 255, 127, 0, 0, 245, 37, 186, 93, 255, 127, 0, 0, 14, 38, 186, 93,
        255, 127, 0, 0, 34, 38, 186, 93, 255, 127, 0, 0, 194, 39, 186, 93, 255, 127, 0, 0, 227, 39,
        186, 93, 255, 127, 0, 0, 43, 40, 186, 93, 255, 127, 0, 0, 14, 41, 186, 93, 255, 127, 0, 0,
        78, 41, 186, 93, 255, 127, 0, 0, 190, 41, 186, 93, 255, 127, 0, 0, 207, 41, 186, 93, 255,
        127, 0, 0, 239, 41, 186, 93, 255, 127, 0, 0, 108, 49, 186, 93, 255, 127, 0, 0, 124, 49,
        186, 93, 255, 127, 0, 0, 12, 50, 186, 93, 255, 127, 0, 0, 63, 50, 186, 93, 255, 127, 0, 0,
        81, 50, 186, 93, 255, 127, 0, 0, 188, 50, 186, 93, 255, 127, 0, 0, 231, 50, 186, 93, 255,
        127, 0, 0, 140, 51, 186, 93, 255, 127, 0, 0, 193, 51, 186, 93, 255, 127, 0, 0, 253, 51,
        186, 93, 255, 127, 0, 0, 133, 52, 186, 93, 255, 127, 0, 0, 56, 53, 186, 93, 255, 127, 0, 0,
        117, 53, 186, 93, 255, 127, 0, 0, 200, 53, 186, 93, 255, 127, 0, 0, 242, 53, 186, 93, 255,
        127, 0, 0, 253, 53, 186, 93, 255, 127, 0, 0, 210, 56, 186, 93, 255, 127, 0, 0, 220, 56,
        186, 93, 255, 127, 0, 0, 3, 57, 186, 93, 255, 127, 0, 0, 60, 58, 186, 93, 255, 127, 0, 0,
        165, 58, 186, 93, 255, 127, 0, 0, 187, 58, 186, 93, 255, 127, 0, 0, 222, 58, 186, 93, 255,
        127, 0, 0, 15, 59, 186, 93, 255, 127, 0, 0, 38, 59, 186, 93, 255, 127, 0, 0, 120, 59, 186,
        93, 255, 127, 0, 0, 157, 59, 186, 93, 255, 127, 0, 0, 165, 59, 186, 93, 255, 127, 0, 0, 10,
        60, 186, 93, 255, 127, 0, 0, 51, 60, 186, 93, 255, 127, 0, 0, 83, 60, 186, 93, 255, 127, 0,
        0, 130, 60, 186, 93, 255, 127, 0, 0, 152, 60, 186, 93, 255, 127, 0, 0, 172, 60, 186, 93,
        255, 127, 0, 0, 209, 60, 186, 93, 255, 127, 0, 0, 223, 61, 186, 93, 255, 127, 0, 0, 20, 62,
        186, 93, 255, 127, 0, 0, 47, 62, 186, 93, 255, 127, 0, 0, 67, 62, 186, 93, 255, 127, 0, 0,
        81, 62, 186, 93, 255, 127, 0, 0, 99, 62, 186, 93, 255, 127, 0, 0, 112, 62, 186, 93, 255,
        127, 0, 0, 138, 62, 186, 93, 255, 127, 0, 0, 192, 62, 186, 93, 255, 127, 0, 0, 237, 64,
        186, 93, 255, 127, 0, 0, 43, 66, 186, 93, 255, 127, 0, 0, 69, 66, 186, 93, 255, 127, 0, 0,
        29, 76, 186, 93, 255, 127, 0, 0, 52, 76, 186, 93, 255, 127, 0, 0, 83, 76, 186, 93, 255,
        127, 0, 0, 106, 76, 186, 93, 255, 127, 0, 0, 132, 76, 186, 93, 255, 127, 0, 0, 157, 76,
        186, 93, 255, 127, 0, 0, 193, 76, 186, 93, 255, 127, 0, 0, 237, 76, 186, 93, 255, 127, 0,
        0, 30, 77, 186, 93, 255, 127, 0, 0, 76, 77, 186, 93, 255, 127, 0, 0, 109, 77, 186, 93, 255,
        127, 0, 0, 159, 77, 186, 93, 255, 127, 0, 0, 180, 77, 186, 93, 255, 127, 0, 0, 220, 77,
        186, 93, 255, 127, 0, 0, 43, 78, 186, 93, 255, 127, 0, 0, 60, 78, 186, 93, 255, 127, 0, 0,
        81, 78, 186, 93, 255, 127, 0, 0, 115, 78, 186, 93, 255, 127, 0, 0, 211, 78, 186, 93, 255,
        127, 0, 0, 236, 78, 186, 93, 255, 127, 0, 0, 18, 79, 186, 93, 255, 127, 0, 0, 53, 79, 186,
        93, 255, 127, 0, 0, 95, 79, 186, 93, 255, 127, 0, 0, 116, 79, 186, 93, 255, 127, 0, 0, 141,
        79, 186, 93, 255, 127, 0, 0, 170, 79, 186, 93, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 33,
        0, 0, 0, 0, 0, 0, 0, 0, 208, 160, 43, 202, 127, 0, 0, 51, 0, 0, 0, 0, 0, 0, 0, 48, 14, 0,
        0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 255, 251, 235, 191, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0,
        0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0,
        0, 0, 0, 0, 0, 64, 144, 123, 202, 20, 86, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 56, 0, 0, 0, 0, 0,
        0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 240, 160,
        43, 202, 127, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0,
        64, 25, 124, 202, 20, 86, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0, 0, 0, 0, 0, 12, 0,
        0, 0, 0, 0, 0, 0, 232, 3, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0,
        0, 14, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 23, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 153, 240, 185, 93, 255, 127, 0, 0, 26, 0, 0, 0, 0, 0,
        0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 31, 0, 0, 0, 0, 0, 0, 0, 201, 79, 186, 93, 255, 127, 0, 0,
        15, 0, 0, 0, 0, 0, 0, 0, 169, 240, 185, 93, 255, 127, 0, 0, 27, 0, 0, 0, 0, 0, 0, 0, 28, 0,
        0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 253, 120, 161, 11, 82, 13, 91, 238, 102,
        222, 133, 171, 66, 146, 247, 165, 120, 56, 54, 95, 54, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);

    // Extracted from a Linux application during startup.
    #[cfg(target_arch = "x86")]
    const TEST_DATA_X86: Aligned8<[u8; 804]> = Aligned8([
        4, 0, 0, 0, 148, 92, 235, 255, 213, 92, 235, 255, 219, 92, 235, 255, 226, 92, 235, 255, 0,
        0, 0, 0, 232, 92, 235, 255, 85, 93, 235, 255, 167, 93, 235, 255, 200, 93, 235, 255, 254,
        93, 235, 255, 64, 94, 235, 255, 120, 94, 235, 255, 30, 95, 235, 255, 93, 95, 235, 255, 115,
        95, 235, 255, 139, 95, 235, 255, 170, 95, 235, 255, 197, 95, 235, 255, 6, 96, 235, 255, 36,
        96, 235, 255, 60, 96, 235, 255, 86, 96, 235, 255, 112, 96, 235, 255, 138, 96, 235, 255,
        161, 96, 235, 255, 240, 96, 235, 255, 38, 97, 235, 255, 60, 97, 235, 255, 82, 97, 235, 255,
        93, 97, 235, 255, 139, 97, 235, 255, 152, 97, 235, 255, 94, 110, 235, 255, 186, 110, 235,
        255, 203, 110, 235, 255, 224, 110, 235, 255, 176, 111, 235, 255, 55, 114, 235, 255, 151,
        114, 235, 255, 186, 114, 235, 255, 45, 125, 235, 255, 69, 125, 235, 255, 102, 125, 235,
        255, 125, 125, 235, 255, 224, 126, 235, 255, 179, 129, 235, 255, 199, 129, 235, 255, 124,
        131, 235, 255, 171, 131, 235, 255, 16, 132, 235, 255, 116, 132, 235, 255, 228, 132, 235,
        255, 251, 132, 235, 255, 12, 133, 235, 255, 35, 133, 235, 255, 65, 133, 235, 255, 92, 133,
        235, 255, 116, 133, 235, 255, 136, 133, 235, 255, 159, 133, 235, 255, 180, 133, 235, 255,
        205, 133, 235, 255, 225, 133, 235, 255, 176, 135, 235, 255, 209, 135, 235, 255, 25, 136,
        235, 255, 252, 136, 235, 255, 60, 137, 235, 255, 172, 137, 235, 255, 189, 137, 235, 255,
        221, 137, 235, 255, 90, 145, 235, 255, 106, 145, 235, 255, 250, 145, 235, 255, 45, 146,
        235, 255, 63, 146, 235, 255, 170, 146, 235, 255, 213, 146, 235, 255, 122, 147, 235, 255,
        175, 147, 235, 255, 235, 147, 235, 255, 115, 148, 235, 255, 38, 149, 235, 255, 99, 149,
        235, 255, 182, 149, 235, 255, 224, 149, 235, 255, 235, 149, 235, 255, 192, 152, 235, 255,
        202, 152, 235, 255, 241, 152, 235, 255, 42, 154, 235, 255, 147, 154, 235, 255, 169, 154,
        235, 255, 204, 154, 235, 255, 253, 154, 235, 255, 20, 155, 235, 255, 102, 155, 235, 255,
        139, 155, 235, 255, 147, 155, 235, 255, 248, 155, 235, 255, 33, 156, 235, 255, 65, 156,
        235, 255, 112, 156, 235, 255, 134, 156, 235, 255, 154, 156, 235, 255, 191, 156, 235, 255,
        205, 157, 235, 255, 2, 158, 235, 255, 29, 158, 235, 255, 49, 158, 235, 255, 63, 158, 235,
        255, 81, 158, 235, 255, 94, 158, 235, 255, 120, 158, 235, 255, 174, 158, 235, 255, 219,
        160, 235, 255, 25, 162, 235, 255, 51, 162, 235, 255, 11, 172, 235, 255, 34, 172, 235, 255,
        65, 172, 235, 255, 88, 172, 235, 255, 114, 172, 235, 255, 139, 172, 235, 255, 175, 172,
        235, 255, 219, 172, 235, 255, 12, 173, 235, 255, 58, 173, 235, 255, 91, 173, 235, 255, 141,
        173, 235, 255, 162, 173, 235, 255, 202, 173, 235, 255, 25, 174, 235, 255, 42, 174, 235,
        255, 63, 174, 235, 255, 97, 174, 235, 255, 193, 174, 235, 255, 218, 174, 235, 255, 0, 175,
        235, 255, 35, 175, 235, 255, 77, 175, 235, 255, 98, 175, 235, 255, 123, 175, 235, 255, 152,
        175, 235, 255, 0, 0, 0, 0, 32, 0, 0, 0, 176, 117, 245, 247, 33, 0, 0, 0, 0, 112, 245, 247,
        51, 0, 0, 0, 48, 14, 0, 0, 16, 0, 0, 0, 255, 251, 235, 191, 6, 0, 0, 0, 0, 16, 0, 0, 17, 0,
        0, 0, 100, 0, 0, 0, 3, 0, 0, 0, 52, 128, 4, 8, 4, 0, 0, 0, 32, 0, 0, 0, 5, 0, 0, 0, 8, 0,
        0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 201, 170, 4, 8, 11, 0, 0,
        0, 232, 3, 0, 0, 12, 0, 0, 0, 232, 3, 0, 0, 13, 0, 0, 0, 100, 0, 0, 0, 14, 0, 0, 0, 100, 0,
        0, 0, 23, 0, 0, 0, 0, 0, 0, 0, 25, 0, 0, 0, 219, 77, 235, 255, 26, 0, 0, 0, 2, 0, 0, 0, 31,
        0, 0, 0, 183, 175, 235, 255, 15, 0, 0, 0, 235, 77, 235, 255, 27, 0, 0, 0, 28, 0, 0, 0, 28,
        0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 110, 213, 250, 168, 134, 233, 229,
        101, 88, 100, 213, 132, 214, 57, 104, 200, 105, 54, 56, 54, 0, 0, 0, 0, 0,
    ]);

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_parse_real_data() {
        let data = TEST_DATA_X86_64.as_ref();
        let layout = StackLayoutRef::new(data, None);

        assert_eq!(layout.argc(), 4);

        // argv
        {
            assert_eq!(
                layout
                    .get_slice_argv()
                    .as_ptr()
                    .align_offset(align_of::<usize>()),
                0
            );
            assert_eq!(layout.argv_raw_iter().count(), 4);
            // Just printing uncovers memory errors
            layout
                .argv_raw_iter()
                .enumerate()
                .for_each(|(i, ptr)| eprintln!("  arg {i:>2}: {ptr:?}"));
        }

        // envv
        {
            assert_eq!(
                layout
                    .get_slice_envv()
                    .as_ptr()
                    .align_offset(align_of::<usize>()),
                0
            );
            assert_eq!(layout.envv_raw_iter().count(), 139);
            // Just printing uncovers memory errors
            layout
                .envv_raw_iter()
                .enumerate()
                .for_each(|(i, ptr)| eprintln!("  env {i:>2}: {ptr:?}"));
        }

        // auxv
        {
            assert_eq!(
                layout
                    .get_slice_auxv()
                    .as_ptr()
                    .align_offset(align_of::<usize>()),
                0
            );
            // Just printing uncovers memory errors
            assert_eq!(layout.auxv_raw_iter().count(), 20);
            layout
                .auxv_raw_iter()
                .enumerate()
                .for_each(|(i, ptr)| eprintln!("  aux {i:>2}: {ptr:?}"));
        }
    }

    #[test]
    #[cfg(target_arch = "x86")]
    fn test_parse_real_data() {
        let data = TEST_DATA_X86.as_ref();
        let layout = StackLayoutRef::new(data, None);

        assert_eq!(layout.argc(), 4);

        // argv
        {
            assert_eq!(
                layout
                    .get_slice_argv()
                    .as_ptr()
                    .align_offset(align_of::<usize>()),
                0
            );
            
            // Just printing uncovers memory errors
            layout
                .argv_raw_iter()
                .enumerate()
                .for_each(|(i, ptr)| eprintln!("  arg {i:>2}: {ptr:?}"));

            assert_eq!(layout.argv_raw_iter().count(), 4);
        }

        // envv
        {
            assert_eq!(
                layout
                    .get_slice_envv()
                    .as_ptr()
                    .align_offset(align_of::<usize>()),
                0
            );
            
            // Just printing uncovers memory errors
            layout
                .envv_raw_iter()
                .enumerate()
                .for_each(|(i, ptr)| eprintln!("  env {i:>2}: {ptr:?}"));
            
            assert_eq!(layout.envv_raw_iter().count(), 139);
        }

        // auxv
        {
            assert_eq!(
                layout
                    .get_slice_auxv()
                    .as_ptr()
                    .align_offset(align_of::<usize>()),
                0
            );
            
            // Just printing uncovers memory errors
            layout
                .auxv_raw_iter()
                .enumerate()
                .for_each(|(i, ptr)| eprintln!("  aux {i:>2}: {ptr:?}"));
            assert_eq!(layout.auxv_raw_iter().count(), 21);
        }
    }
}
