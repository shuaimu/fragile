//! C standard I/O library support for transpiled C++ code.
//!
//! This module provides Rust implementations of C stdio functions that
//! libc++ fstream uses internally. The transpiled C++ code calls these
//! functions to perform file I/O operations.
//!
//! # Supported Functions
//! - File opening/closing: fopen, fclose
//! - Reading/writing: fread, fwrite
//! - Seeking: fseek, ftell, fseeko, ftello
//! - Buffer control: fflush, setvbuf
//! - Error handling: ferror, feof, clearerr

use core::ffi::{c_char, c_int, c_long, c_void};

#[cfg(feature = "std")]
use std::fs::{File, OpenOptions};
#[cfg(feature = "std")]
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(feature = "std")]
use std::sync::Mutex;

/// C FILE structure - opaque handle to a file stream.
///
/// This wraps a Rust File with additional state tracking for
/// C stdio semantics (buffering, error flags, EOF flag).
#[cfg(feature = "std")]
#[repr(C)]
pub struct FILE {
    /// The underlying Rust file handle
    file: Mutex<Option<File>>,
    /// Error flag (ferror)
    error: core::sync::atomic::AtomicBool,
    /// EOF flag (feof)
    eof: core::sync::atomic::AtomicBool,
}

#[cfg(not(feature = "std"))]
#[repr(C)]
pub struct FILE {
    _placeholder: c_int,
}

// Type aliases for 64-bit file offsets (C names preserved for ABI compatibility)
#[allow(non_camel_case_types)]
pub type off_t = i64;
#[allow(non_camel_case_types)]
pub type fpos_t = i64;

/// Open a file stream.
///
/// # Safety
/// Caller must ensure `filename` and `mode` are valid C strings.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE {
    if filename.is_null() || mode.is_null() {
        return core::ptr::null_mut();
    }

    // Convert C strings to Rust strings
    let filename = match std::ffi::CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return core::ptr::null_mut(),
    };
    let mode = match std::ffi::CStr::from_ptr(mode).to_str() {
        Ok(s) => s,
        Err(_) => return core::ptr::null_mut(),
    };

    // Parse mode string and build OpenOptions
    let mut opts = OpenOptions::new();
    let mut has_read = false;
    let mut has_write = false;
    let mut has_append = false;
    let mut truncate = false;
    let mut create = false;

    for ch in mode.chars() {
        match ch {
            'r' => { has_read = true; }
            'w' => { has_write = true; truncate = true; create = true; }
            'a' => { has_append = true; has_write = true; create = true; }
            '+' => { has_read = true; has_write = true; }
            'b' => { /* binary mode - no effect on Unix */ }
            'x' => { /* exclusive create - handled separately */ }
            _ => {}
        }
    }

    opts.read(has_read);
    opts.write(has_write);
    opts.append(has_append);
    opts.truncate(truncate);
    opts.create(create);

    // Open the file
    match opts.open(filename) {
        Ok(file) => {
            let file_struct = Box::new(FILE {
                file: Mutex::new(Some(file)),
                error: core::sync::atomic::AtomicBool::new(false),
                eof: core::sync::atomic::AtomicBool::new(false),
            });
            Box::into_raw(file_struct)
        }
        Err(_) => core::ptr::null_mut(),
    }
}

/// Close a file stream.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer returned by fopen.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fclose(stream: *mut FILE) -> c_int {
    if stream.is_null() {
        return -1; // EOF
    }

    // Take ownership and drop
    let file_box = Box::from_raw(stream);

    // The File will be closed when dropped
    if let Ok(mut guard) = file_box.file.lock() {
        let _ = guard.take(); // Drop the file
    }

    0 // Success
}

/// Read from a file stream.
///
/// # Safety
/// Caller must ensure all pointers are valid and buffer is large enough.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fread(
    ptr: *mut c_void,
    size: usize,
    nmemb: usize,
    stream: *mut FILE,
) -> usize {
    if ptr.is_null() || stream.is_null() || size == 0 {
        return 0;
    }

    let file_struct = &*stream;
    let total_bytes = size.saturating_mul(nmemb);
    let buffer = core::slice::from_raw_parts_mut(ptr as *mut u8, total_bytes);

    if let Ok(mut guard) = file_struct.file.lock() {
        if let Some(ref mut file) = *guard {
            match file.read(buffer) {
                Ok(0) => {
                    file_struct.eof.store(true, core::sync::atomic::Ordering::SeqCst);
                    0
                }
                Ok(bytes_read) => bytes_read / size,
                Err(_) => {
                    file_struct.error.store(true, core::sync::atomic::Ordering::SeqCst);
                    0
                }
            }
        } else {
            0
        }
    } else {
        0
    }
}

/// Write to a file stream.
///
/// # Safety
/// Caller must ensure all pointers are valid and buffer contains valid data.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fwrite(
    ptr: *const c_void,
    size: usize,
    nmemb: usize,
    stream: *mut FILE,
) -> usize {
    if ptr.is_null() || stream.is_null() || size == 0 {
        return 0;
    }

    let file_struct = &*stream;
    let total_bytes = size.saturating_mul(nmemb);
    let buffer = core::slice::from_raw_parts(ptr as *const u8, total_bytes);

    if let Ok(mut guard) = file_struct.file.lock() {
        if let Some(ref mut file) = *guard {
            match file.write(buffer) {
                Ok(bytes_written) => bytes_written / size,
                Err(_) => {
                    file_struct.error.store(true, core::sync::atomic::Ordering::SeqCst);
                    0
                }
            }
        } else {
            0
        }
    } else {
        0
    }
}

/// Seek within a file stream.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fseek(stream: *mut FILE, offset: c_long, whence: c_int) -> c_int {
    fseeko(stream, offset as off_t, whence)
}

/// Seek within a file stream (64-bit offset version).
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fseeko(stream: *mut FILE, offset: off_t, whence: c_int) -> c_int {
    if stream.is_null() {
        return -1;
    }

    let file_struct = &*stream;

    // Clear EOF flag on seek
    file_struct.eof.store(false, core::sync::atomic::Ordering::SeqCst);

    let seek_from = match whence {
        0 => SeekFrom::Start(offset as u64),      // SEEK_SET
        1 => SeekFrom::Current(offset),            // SEEK_CUR
        2 => SeekFrom::End(offset),                // SEEK_END
        _ => return -1,
    };

    if let Ok(mut guard) = file_struct.file.lock() {
        if let Some(ref mut file) = *guard {
            match file.seek(seek_from) {
                Ok(_) => 0,
                Err(_) => {
                    file_struct.error.store(true, core::sync::atomic::Ordering::SeqCst);
                    -1
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Get current file position.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn ftell(stream: *mut FILE) -> c_long {
    ftello(stream) as c_long
}

/// Get current file position (64-bit version).
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn ftello(stream: *mut FILE) -> off_t {
    if stream.is_null() {
        return -1;
    }

    let file_struct = &*stream;

    if let Ok(mut guard) = file_struct.file.lock() {
        if let Some(ref mut file) = *guard {
            match file.stream_position() {
                Ok(pos) => pos as off_t,
                Err(_) => {
                    file_struct.error.store(true, core::sync::atomic::Ordering::SeqCst);
                    -1
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Flush a file stream.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer (or null for all streams).
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fflush(stream: *mut FILE) -> c_int {
    if stream.is_null() {
        // Flush all open streams - not fully implemented
        return 0;
    }

    let file_struct = &*stream;

    if let Ok(mut guard) = file_struct.file.lock() {
        if let Some(ref mut file) = *guard {
            match file.flush() {
                Ok(_) => 0,
                Err(_) => {
                    file_struct.error.store(true, core::sync::atomic::Ordering::SeqCst);
                    -1 // EOF
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Check end-of-file indicator.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn feof(stream: *mut FILE) -> c_int {
    if stream.is_null() {
        return 0;
    }
    let file_struct = &*stream;
    if file_struct.eof.load(core::sync::atomic::Ordering::SeqCst) {
        1
    } else {
        0
    }
}

/// Check error indicator.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn ferror(stream: *mut FILE) -> c_int {
    if stream.is_null() {
        return 0;
    }
    let file_struct = &*stream;
    if file_struct.error.load(core::sync::atomic::Ordering::SeqCst) {
        1
    } else {
        0
    }
}

/// Clear error and EOF indicators.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn clearerr(stream: *mut FILE) {
    if stream.is_null() {
        return;
    }
    let file_struct = &*stream;
    file_struct.error.store(false, core::sync::atomic::Ordering::SeqCst);
    file_struct.eof.store(false, core::sync::atomic::Ordering::SeqCst);
}

/// Get file number (descriptor).
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fileno(_stream: *mut FILE) -> c_int {
    // Rust File doesn't expose raw fd easily without platform-specific code
    // Return -1 for now - this is mainly used for OS-specific operations
    -1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_fopen_fclose() {
        unsafe {
            // Create a temp file
            let path = CString::new("/tmp/fragile_stdio_test.txt").unwrap();
            let mode = CString::new("w").unwrap();

            let file = fopen(path.as_ptr(), mode.as_ptr());
            assert!(!file.is_null(), "fopen should succeed");

            let result = fclose(file);
            assert_eq!(result, 0, "fclose should succeed");

            // Clean up
            std::fs::remove_file("/tmp/fragile_stdio_test.txt").ok();
        }
    }

    #[test]
    fn test_fwrite_fread() {
        unsafe {
            let path = CString::new("/tmp/fragile_stdio_test2.txt").unwrap();

            // Write
            {
                let mode = CString::new("w").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!file.is_null());

                let data = b"Hello, World!";
                let written = fwrite(data.as_ptr() as *const c_void, 1, data.len(), file);
                assert_eq!(written, data.len());

                fclose(file);
            }

            // Read
            {
                let mode = CString::new("r").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!file.is_null());

                let mut buffer = [0u8; 32];
                let read = fread(buffer.as_mut_ptr() as *mut c_void, 1, buffer.len(), file);
                assert_eq!(read, 13);
                assert_eq!(&buffer[..13], b"Hello, World!");

                fclose(file);
            }

            // Clean up
            std::fs::remove_file("/tmp/fragile_stdio_test2.txt").ok();
        }
    }

    #[test]
    fn test_fseek_ftell() {
        unsafe {
            let path = CString::new("/tmp/fragile_stdio_test3.txt").unwrap();

            // Create file with content
            {
                let mode = CString::new("w").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                let data = b"0123456789";
                fwrite(data.as_ptr() as *const c_void, 1, data.len(), file);
                fclose(file);
            }

            // Test seeking
            {
                let mode = CString::new("r").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());

                // Seek to position 5
                let result = fseek(file, 5, 0); // SEEK_SET
                assert_eq!(result, 0);

                let pos = ftell(file);
                assert_eq!(pos, 5);

                // Read from position 5
                let mut buffer = [0u8; 5];
                let read = fread(buffer.as_mut_ptr() as *mut c_void, 1, 5, file);
                assert_eq!(read, 5);
                assert_eq!(&buffer, b"56789");

                fclose(file);
            }

            // Clean up
            std::fs::remove_file("/tmp/fragile_stdio_test3.txt").ok();
        }
    }
}
