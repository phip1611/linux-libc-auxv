//! Utility functions for parsing and serializing null-terminated C-strings.

pub fn c_str_null_terminated(cstr: &[u8]) -> bool {
    cstr.last()
        .copied()
        .map(|last_byte| last_byte == 0)
        .unwrap_or(false)
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
    use crate::cstr_util::{c_str_len_ptr, c_str_null_terminated};

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
}
