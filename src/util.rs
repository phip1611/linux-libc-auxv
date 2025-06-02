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

/// Returns the index of the first null byte in the given slice.
///
/// If this returns `None`, the slice doesn't contain a NUL byte.
pub fn get_null_index(bytes: &[u8]) -> Option<usize> {
    bytes.iter().position(|&b| b == 0)
}

/// Returns the number of bytes until the first null byte in the given slice.
///
/// If this returns `None`, the slice doesn't contain a NUL byte.
pub fn count_bytes_until_null(bytes: &[u8]) -> Option<usize> {
    get_null_index(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_bytes_until_null() {
        assert_eq!(get_null_index(b"hello\0world"), Some(5));
        assert_eq!(count_bytes_until_null(b"hello\0world"), Some(5));

        assert_eq!(get_null_index(b"hello\0b\0"), Some(5));
        assert_eq!(count_bytes_until_null(b"hello\0b\0"), Some(5));

        assert_eq!(get_null_index(b"\0\0"), Some(0));
        assert_eq!(count_bytes_until_null(b"\0\0"), Some(0));

        assert_eq!(get_null_index(b"1\0\0"), Some(1));
        assert_eq!(count_bytes_until_null(b"1\0\0"), Some(1));

        assert_eq!(get_null_index(b""), None);
        assert_eq!(count_bytes_until_null(b""), None);

        assert_eq!(get_null_index(b"abc"), None);
        assert_eq!(count_bytes_until_null(b"abc"), None);
    }
}
