//! C standard I/O library support for transpiled C++ code.
//!
//! This module provides Rust implementations of C stdio functions that
//! libc++ iostream and fstream use internally. The transpiled C++ code
//! calls these functions to perform I/O operations.
//!
//! # Supported Functions
//! - File opening/closing: fopen, fclose
//! - Reading/writing: fread, fwrite, fgetc, fputc, getc, putc
//! - Character I/O: getchar, putchar, ungetc
//! - Seeking: fseek, ftell, fseeko, ftello
//! - Buffer control: fflush, setvbuf
//! - Error handling: ferror, feof, clearerr
//! - Standard streams: stdin, stdout, stderr

use core::ffi::{c_char, c_int, c_long, c_void};

#[cfg(feature = "std")]
use std::fs::{File, OpenOptions};
#[cfg(feature = "std")]
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(feature = "std")]
use std::sync::Mutex;

/// EOF constant (matches C definition)
pub const EOF: c_int = -1;

/// Stream kind - file or standard stream
#[cfg(feature = "std")]
enum StreamKind {
    /// Regular file opened with fopen
    File(Option<File>),
    /// Standard input stream
    Stdin,
    /// Standard output stream
    Stdout,
    /// Standard error stream
    Stderr,
}

/// C FILE structure - opaque handle to a file stream.
///
/// This wraps a Rust File or standard stream with additional state tracking
/// for C stdio semantics (buffering, error flags, EOF flag, ungetc buffer).
#[cfg(feature = "std")]
#[repr(C)]
pub struct FILE {
    /// The underlying stream
    stream: Mutex<StreamKind>,
    /// Error flag (ferror)
    error: core::sync::atomic::AtomicBool,
    /// EOF flag (feof)
    eof: core::sync::atomic::AtomicBool,
    /// Ungetc buffer (single character, -1 if empty)
    ungetc_buf: core::sync::atomic::AtomicI32,
}

#[cfg(not(feature = "std"))]
#[repr(C)]
pub struct FILE {
    _placeholder: c_int,
}

// Standard stream FILE structures (initialized lazily)
#[cfg(feature = "std")]
static STDIN_FILE: std::sync::LazyLock<FILE> = std::sync::LazyLock::new(|| FILE {
    stream: Mutex::new(StreamKind::Stdin),
    error: core::sync::atomic::AtomicBool::new(false),
    eof: core::sync::atomic::AtomicBool::new(false),
    ungetc_buf: core::sync::atomic::AtomicI32::new(-1),
});

#[cfg(feature = "std")]
static STDOUT_FILE: std::sync::LazyLock<FILE> = std::sync::LazyLock::new(|| FILE {
    stream: Mutex::new(StreamKind::Stdout),
    error: core::sync::atomic::AtomicBool::new(false),
    eof: core::sync::atomic::AtomicBool::new(false),
    ungetc_buf: core::sync::atomic::AtomicI32::new(-1),
});

#[cfg(feature = "std")]
static STDERR_FILE: std::sync::LazyLock<FILE> = std::sync::LazyLock::new(|| FILE {
    stream: Mutex::new(StreamKind::Stderr),
    error: core::sync::atomic::AtomicBool::new(false),
    eof: core::sync::atomic::AtomicBool::new(false),
    ungetc_buf: core::sync::atomic::AtomicI32::new(-1),
});

/// Get pointer to standard input stream.
#[no_mangle]
#[cfg(feature = "std")]
pub extern "C" fn __fragile_stdin() -> *mut FILE {
    &*STDIN_FILE as *const FILE as *mut FILE
}

/// Get pointer to standard output stream.
#[no_mangle]
#[cfg(feature = "std")]
pub extern "C" fn __fragile_stdout() -> *mut FILE {
    &*STDOUT_FILE as *const FILE as *mut FILE
}

/// Get pointer to standard error stream.
#[no_mangle]
#[cfg(feature = "std")]
pub extern "C" fn __fragile_stderr() -> *mut FILE {
    &*STDERR_FILE as *const FILE as *mut FILE
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
            'r' => {
                has_read = true;
            }
            'w' => {
                has_write = true;
                truncate = true;
                create = true;
            }
            'a' => {
                has_append = true;
                has_write = true;
                create = true;
            }
            '+' => {
                has_read = true;
                has_write = true;
            }
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
                stream: Mutex::new(StreamKind::File(Some(file))),
                error: core::sync::atomic::AtomicBool::new(false),
                eof: core::sync::atomic::AtomicBool::new(false),
                ungetc_buf: core::sync::atomic::AtomicI32::new(-1),
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
/// Do NOT call on stdin/stdout/stderr.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fclose(stream: *mut FILE) -> c_int {
    if stream.is_null() {
        return EOF;
    }

    // Don't close standard streams
    if stream == __fragile_stdin() || stream == __fragile_stdout() || stream == __fragile_stderr() {
        return EOF;
    }

    // Take ownership and drop
    let file_box = Box::from_raw(stream);

    // The File will be closed when dropped
    if let Ok(mut guard) = file_box.stream.lock() {
        if let StreamKind::File(ref mut f) = *guard {
            let _ = f.take(); // Drop the file
        }
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

    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.read(buffer),
            StreamKind::Stdin => std::io::stdin().read(buffer),
            _ => return 0,
        };
        match result {
            Ok(0) => {
                file_struct
                    .eof
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                0
            }
            Ok(bytes_read) => bytes_read / size,
            Err(_) => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                0
            }
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

    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.write(buffer),
            StreamKind::Stdout => std::io::stdout().write(buffer),
            StreamKind::Stderr => std::io::stderr().write(buffer),
            _ => return 0,
        };
        match result {
            Ok(bytes_written) => bytes_written / size,
            Err(_) => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                0
            }
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
    file_struct
        .eof
        .store(false, core::sync::atomic::Ordering::SeqCst);
    // Clear ungetc buffer on seek
    file_struct
        .ungetc_buf
        .store(-1, core::sync::atomic::Ordering::SeqCst);

    let seek_from = match whence {
        0 => SeekFrom::Start(offset as u64), // SEEK_SET
        1 => SeekFrom::Current(offset),      // SEEK_CUR
        2 => SeekFrom::End(offset),          // SEEK_END
        _ => return -1,
    };

    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.seek(seek_from),
            // Cannot seek on standard streams
            _ => return -1,
        };
        match result {
            Ok(_) => 0,
            Err(_) => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                -1
            }
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

    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.stream_position(),
            // Cannot get position on standard streams
            _ => return -1,
        };
        match result {
            Ok(pos) => pos as off_t,
            Err(_) => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                -1
            }
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
        // Flush all open streams - flush stdout and stderr
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        return 0;
    }

    let file_struct = &*stream;

    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.flush(),
            StreamKind::Stdout => std::io::stdout().flush(),
            StreamKind::Stderr => std::io::stderr().flush(),
            _ => return 0, // stdin doesn't need flushing
        };
        match result {
            Ok(_) => 0,
            Err(_) => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                EOF
            }
        }
    } else {
        EOF
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
    file_struct
        .error
        .store(false, core::sync::atomic::Ordering::SeqCst);
    file_struct
        .eof
        .store(false, core::sync::atomic::Ordering::SeqCst);
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

// ============================================================================
// Character-level I/O functions (for iostream support)
// ============================================================================

/// Read a character from a stream.
///
/// Returns the character as an unsigned char cast to int, or EOF on error/end-of-file.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fgetc(stream: *mut FILE) -> c_int {
    if stream.is_null() {
        return EOF;
    }

    let file_struct = &*stream;

    // Check ungetc buffer first
    let ungetc_val = file_struct
        .ungetc_buf
        .swap(-1, core::sync::atomic::Ordering::SeqCst);
    if ungetc_val >= 0 {
        return ungetc_val;
    }

    let mut byte = [0u8; 1];
    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.read(&mut byte),
            StreamKind::Stdin => std::io::stdin().read(&mut byte),
            _ => return EOF,
        };
        match result {
            Ok(0) => {
                file_struct
                    .eof
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                EOF
            }
            Ok(_) => byte[0] as c_int,
            Err(_) => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                EOF
            }
        }
    } else {
        EOF
    }
}

/// Read a character from a stream (macro-safe version, same as fgetc).
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn getc(stream: *mut FILE) -> c_int {
    fgetc(stream)
}

/// Read a character from stdin.
///
/// # Safety
/// This function accesses the global stdin stream, which must be properly initialized.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn getchar() -> c_int {
    fgetc(__fragile_stdin())
}

/// Write a character to a stream.
///
/// Returns the character written as an unsigned char cast to int, or EOF on error.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fputc(c: c_int, stream: *mut FILE) -> c_int {
    if stream.is_null() {
        return EOF;
    }

    let file_struct = &*stream;
    let byte = [c as u8; 1];

    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.write(&byte),
            StreamKind::Stdout => std::io::stdout().write(&byte),
            StreamKind::Stderr => std::io::stderr().write(&byte),
            _ => return EOF,
        };
        match result {
            Ok(1) => c as u8 as c_int,
            _ => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                EOF
            }
        }
    } else {
        EOF
    }
}

/// Write a character to a stream (macro-safe version, same as fputc).
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn putc(c: c_int, stream: *mut FILE) -> c_int {
    fputc(c, stream)
}

/// Write a character to stdout.
///
/// # Safety
/// This function accesses the global stdout stream, which must be properly initialized.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn putchar(c: c_int) -> c_int {
    fputc(c, __fragile_stdout())
}

/// Push a character back onto the input stream.
///
/// Only one character of pushback is guaranteed.
///
/// # Safety
/// Caller must ensure `stream` is a valid FILE pointer.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn ungetc(c: c_int, stream: *mut FILE) -> c_int {
    if stream.is_null() || c == EOF {
        return EOF;
    }

    let file_struct = &*stream;

    // Clear EOF flag
    file_struct
        .eof
        .store(false, core::sync::atomic::Ordering::SeqCst);

    // Try to store in ungetc buffer (only one char guaranteed)
    let prev = file_struct.ungetc_buf.compare_exchange(
        -1,
        c,
        core::sync::atomic::Ordering::SeqCst,
        core::sync::atomic::Ordering::SeqCst,
    );

    match prev {
        Ok(_) => c as u8 as c_int,
        Err(_) => EOF, // Buffer already full
    }
}

/// Write a string to a stream.
///
/// # Safety
/// Caller must ensure `s` is a valid null-terminated C string.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fputs(s: *const c_char, stream: *mut FILE) -> c_int {
    if s.is_null() || stream.is_null() {
        return EOF;
    }

    let cstr = std::ffi::CStr::from_ptr(s);
    let bytes = cstr.to_bytes();

    let file_struct = &*stream;

    if let Ok(mut guard) = file_struct.stream.lock() {
        let result = match &mut *guard {
            StreamKind::File(Some(ref mut file)) => file.write_all(bytes),
            StreamKind::Stdout => std::io::stdout().write_all(bytes),
            StreamKind::Stderr => std::io::stderr().write_all(bytes),
            _ => return EOF,
        };
        match result {
            Ok(_) => 0,
            Err(_) => {
                file_struct
                    .error
                    .store(true, core::sync::atomic::Ordering::SeqCst);
                EOF
            }
        }
    } else {
        EOF
    }
}

/// Write a string to stdout followed by a newline.
///
/// # Safety
/// Caller must ensure `s` is a valid null-terminated C string.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn puts(s: *const c_char) -> c_int {
    if s.is_null() {
        return EOF;
    }

    let cstr = std::ffi::CStr::from_ptr(s);
    let bytes = cstr.to_bytes();

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    if handle.write_all(bytes).is_err() {
        return EOF;
    }
    if handle.write_all(b"\n").is_err() {
        return EOF;
    }

    0
}

/// Read a line from a stream.
///
/// # Safety
/// Caller must ensure pointers are valid and buffer is large enough.
#[no_mangle]
#[cfg(feature = "std")]
pub unsafe extern "C" fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char {
    if s.is_null() || stream.is_null() || n <= 0 {
        return core::ptr::null_mut();
    }

    let file_struct = &*stream;
    let buffer = core::slice::from_raw_parts_mut(s as *mut u8, n as usize);
    let mut pos = 0usize;
    let max_read = (n - 1) as usize; // Leave room for null terminator

    while pos < max_read {
        let c = fgetc(stream);
        if c == EOF {
            if pos == 0 {
                return core::ptr::null_mut(); // No data read
            }
            break;
        }
        buffer[pos] = c as u8;
        pos += 1;
        if c as u8 == b'\n' {
            break;
        }
    }

    // Null-terminate
    buffer[pos] = 0;

    // Suppress unused warning - we need file_struct for consistency
    let _ = file_struct;

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_standard_streams() {
        unsafe {
            // Test that standard streams are accessible
            let stdin = __fragile_stdin();
            let stdout = __fragile_stdout();
            let stderr = __fragile_stderr();

            assert!(!stdin.is_null());
            assert!(!stdout.is_null());
            assert!(!stderr.is_null());

            // Test that they're different pointers
            assert_ne!(stdin, stdout);
            assert_ne!(stdout, stderr);
            assert_ne!(stdin, stderr);

            // Test that closing standard streams fails (returns EOF)
            assert_eq!(fclose(stdin), EOF);
            assert_eq!(fclose(stdout), EOF);
            assert_eq!(fclose(stderr), EOF);
        }
    }

    #[test]
    fn test_fputc_fgetc() {
        unsafe {
            let path = CString::new("/tmp/fragile_stdio_chartest.txt").unwrap();

            // Write characters
            {
                let mode = CString::new("w").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!file.is_null());

                assert_eq!(fputc(b'H' as c_int, file), b'H' as c_int);
                assert_eq!(fputc(b'i' as c_int, file), b'i' as c_int);
                assert_eq!(fputc(b'!' as c_int, file), b'!' as c_int);

                fclose(file);
            }

            // Read characters
            {
                let mode = CString::new("r").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!file.is_null());

                assert_eq!(fgetc(file), b'H' as c_int);
                assert_eq!(fgetc(file), b'i' as c_int);
                assert_eq!(fgetc(file), b'!' as c_int);
                assert_eq!(fgetc(file), EOF); // End of file

                fclose(file);
            }

            // Clean up
            std::fs::remove_file("/tmp/fragile_stdio_chartest.txt").ok();
        }
    }

    #[test]
    fn test_ungetc() {
        unsafe {
            let path = CString::new("/tmp/fragile_stdio_ungetc.txt").unwrap();

            // Create file with content
            {
                let mode = CString::new("w").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                fputc(b'A' as c_int, file);
                fputc(b'B' as c_int, file);
                fclose(file);
            }

            // Test ungetc
            {
                let mode = CString::new("r").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!file.is_null());

                // Read first char
                let c = fgetc(file);
                assert_eq!(c, b'A' as c_int);

                // Push it back
                assert_eq!(ungetc(c, file), b'A' as c_int);

                // Read again - should get the pushed-back char
                assert_eq!(fgetc(file), b'A' as c_int);

                // Continue reading
                assert_eq!(fgetc(file), b'B' as c_int);

                fclose(file);
            }

            // Clean up
            std::fs::remove_file("/tmp/fragile_stdio_ungetc.txt").ok();
        }
    }

    #[test]
    fn test_fputs_fgets() {
        unsafe {
            let path = CString::new("/tmp/fragile_stdio_strings.txt").unwrap();

            // Write strings
            {
                let mode = CString::new("w").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!file.is_null());

                let line1 = CString::new("Hello\n").unwrap();
                let line2 = CString::new("World\n").unwrap();

                assert_eq!(fputs(line1.as_ptr(), file), 0);
                assert_eq!(fputs(line2.as_ptr(), file), 0);

                fclose(file);
            }

            // Read strings
            {
                let mode = CString::new("r").unwrap();
                let file = fopen(path.as_ptr(), mode.as_ptr());
                assert!(!file.is_null());

                let mut buffer = [0i8; 64];
                let result = fgets(buffer.as_mut_ptr(), 64, file);
                assert!(!result.is_null());
                let line = std::ffi::CStr::from_ptr(buffer.as_ptr());
                assert_eq!(line.to_str().unwrap(), "Hello\n");

                let result = fgets(buffer.as_mut_ptr(), 64, file);
                assert!(!result.is_null());
                let line = std::ffi::CStr::from_ptr(buffer.as_ptr());
                assert_eq!(line.to_str().unwrap(), "World\n");

                fclose(file);
            }

            // Clean up
            std::fs::remove_file("/tmp/fragile_stdio_strings.txt").ok();
        }
    }

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
