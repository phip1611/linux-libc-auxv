//! Utility functions for parsing and serializing null-terminated C-strings.

/// Checks that a C-string doesn't contain a null byte somewhere in the middle.
pub(crate) fn cstr_contains_at_most_terminating_null_byte(cstr: &[u8]) -> bool {
    let null_count = cstr.iter().filter(|x| **x == 0).count();
    if null_count == 0 {
        true
    } else {
        null_count == 1 && c_str_null_terminated(cstr)
    }
}

/// Checks if a C-string has a terminating null byte.
pub(crate) fn c_str_null_terminated(cstr: &[u8]) -> bool {
    cstr.last()
        .copied()
        .map(|last_byte| last_byte == 0)
        .unwrap_or(false)
}

/// Returns the length of a C-string including the final null byte.
/// If the cstring already contains the terminating null byte, it returns
/// it's length. Otherwise it adds +1 to the length.
pub(crate) fn cstr_len_with_nullbyte(cstr: &[u8]) -> usize {
    if c_str_null_terminated(cstr) {
        cstr.len()
    } else {
        cstr.len() + 1
    }
}

/// Determines the length of a C-string without the terminating null byte
/// by iterating over the memory from the begin pointer.
/// Panics, if no null-byte was found after `100000` iterations.
pub(crate) fn c_str_len_ptr(mut ptr: *const u8) -> usize {
    let mut i = 0;
    while unsafe { *ptr != 0 } {
        ptr = unsafe { ptr.add(1) };
        i += 1;

        // in my use case there will be no strings that are longer than this
        if i >= 100000 {
            panic!("memory error? not null terminated C-string?");
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_str_len() {
        assert_eq!(c_str_len_ptr("hallo\0".as_ptr()), 5);
        assert_eq!(c_str_len_ptr("\0".as_ptr()), 0);
        assert_eq!(c_str_len_ptr("hallo welt\0".as_ptr()), 10);
    }

    #[should_panic]
    #[test]
    fn test_c_str_len_panic() {
        // expect panic because there is no terminating null
        let _ = c_str_len_ptr("hallo".repeat(100000).as_ptr());
    }

    #[test]
    fn test_c_str_null_terminated() {
        assert!(!c_str_null_terminated(b"foobar"));
        assert!(!c_str_null_terminated(b""));
        assert!(c_str_null_terminated(b"\0"));
        assert!(c_str_null_terminated(b"hallo\0"));
    }

    #[test]
    fn test_cstr_contains_at_most_terminating_null_byte() {
        assert!(cstr_contains_at_most_terminating_null_byte(b"foobar"));
        assert!(cstr_contains_at_most_terminating_null_byte(b"foobar\0"));
        assert!(cstr_contains_at_most_terminating_null_byte(b"\0"));
        assert!(cstr_contains_at_most_terminating_null_byte(b"foobar\0"));
        assert!(!cstr_contains_at_most_terminating_null_byte(b"\0\0"));
        assert!(!cstr_contains_at_most_terminating_null_byte(b"foo\0bar\0"));
    }

    #[test]
    fn test_cstr_len_with_nullbyte() {
        assert_eq!(cstr_len_with_nullbyte(b"foo"), 4);
        assert_eq!(cstr_len_with_nullbyte(b"foo\0"), 4);
        assert_eq!(cstr_len_with_nullbyte(b""), 1);
        assert_eq!(cstr_len_with_nullbyte(b"\0"), 1);
    }
}
