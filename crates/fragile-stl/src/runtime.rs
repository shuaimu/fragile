// fragile_runtime stub for memory allocation
pub mod fragile_runtime {
    static mut FRAGILE_UNIT_SINGLETON: () = ();

    #[inline]
    pub unsafe fn fragile_malloc(size: usize) -> *mut () {
        c_malloc(size) as *mut ()
    }
    #[inline]
    pub unsafe fn fragile_free(ptr: *mut u8) {
        c_free(ptr as *mut std::ffi::c_void);
    }
    unsafe extern "C" {
        #[link_name = "malloc"] fn c_malloc(size: usize) -> *mut std::ffi::c_void;
        #[link_name = "calloc"] fn c_calloc(nmemb: usize, size: usize) -> *mut std::ffi::c_void;
        #[link_name = "realloc"] fn c_realloc(ptr: *mut std::ffi::c_void, size: usize) -> *mut std::ffi::c_void;
        #[link_name = "free"] fn c_free(ptr: *mut std::ffi::c_void);
        #[link_name = "fopen"] fn c_fopen(path: *const i8, mode: *const i8) -> *mut std::ffi::c_void;
        #[link_name = "fclose"] fn c_fclose(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "fread"] fn c_fread(ptr: *mut std::ffi::c_void, size: usize, nmemb: usize, stream: *mut std::ffi::c_void) -> usize;
        #[link_name = "fwrite"] fn c_fwrite(ptr: *const std::ffi::c_void, size: usize, nmemb: usize, stream: *mut std::ffi::c_void) -> usize;
        #[link_name = "fseek"] fn c_fseek(stream: *mut std::ffi::c_void, offset: i64, whence: i32) -> i32;
        #[link_name = "fseeko"] fn c_fseeko(stream: *mut std::ffi::c_void, offset: i64, whence: i32) -> i32;
        #[link_name = "ftell"] fn c_ftell(stream: *mut std::ffi::c_void) -> i64;
        #[link_name = "ftello"] fn c_ftello(stream: *mut std::ffi::c_void) -> i64;
        #[link_name = "fflush"] fn c_fflush(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "feof"] fn c_feof(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "ferror"] fn c_ferror(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "clearerr"] fn c_clearerr(stream: *mut std::ffi::c_void);
        #[link_name = "fileno"] fn c_fileno(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "fgetc"] fn c_fgetc(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "getc"] fn c_getc(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "getchar"] fn c_getchar() -> i32;
        #[link_name = "fputc"] fn c_fputc(c: i32, stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "putc"] fn c_putc(c: i32, stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "putchar"] fn c_putchar(c: i32) -> i32;
        #[link_name = "ungetc"] fn c_ungetc(c: i32, stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "fputs"] fn c_fputs(s: *const i8, stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "puts"] fn c_puts(s: *const i8) -> i32;
        #[link_name = "fgets"] fn c_fgets(s: *mut i8, n: i32, stream: *mut std::ffi::c_void) -> *mut i8;
        #[link_name = "popen"] fn c_popen(command: *const i8, mode: *const i8) -> *mut std::ffi::c_void;
        #[link_name = "pclose"] fn c_pclose(stream: *mut std::ffi::c_void) -> i32;
        #[link_name = "rand_r"] fn c_rand_r(seed: *mut u32) -> i32;
        #[link_name = "backtrace"] fn c_backtrace(buffer: *mut *mut std::ffi::c_void, size: i32) -> i32;
        #[link_name = "backtrace_symbols"] fn c_backtrace_symbols(
            buffer: *const *mut std::ffi::c_void,
            size: i32,
        ) -> *mut *mut i8;
        #[link_name = "_ZN7testing14InitGoogleTestEPiPPc"]
        fn c_gtest_init(argc: *mut i32, argv: *mut *mut i8);
        #[link_name = "_ZN7testing8UnitTest11GetInstanceEv"]
        fn c_gtest_get_instance() -> *mut fragile_gtest_unit_test;
        #[link_name = "_ZN7testing8UnitTest3RunEv"]
        fn c_gtest_run(this_: *mut fragile_gtest_unit_test) -> i32;
    }
    #[inline]
    pub unsafe fn fragile_calloc(nmemb: usize, size: usize) -> *mut () { c_calloc(nmemb, size) as *mut () }
    #[inline]
    pub unsafe fn fragile_realloc(ptr: *mut u8, size: usize) -> *mut () { c_realloc(ptr as *mut std::ffi::c_void, size) as *mut () }
    #[inline]
    pub unsafe fn fopen(path: *const i8, mode: *const i8) -> *mut std::ffi::c_void { c_fopen(path, mode) }
    #[inline]
    pub unsafe fn fclose(stream: *mut std::ffi::c_void) -> i32 { c_fclose(stream) }
    #[inline]
    pub unsafe fn fread(ptr: *mut (), size: u64, nmemb: u64, stream: *mut std::ffi::c_void) -> u64 { c_fread(ptr as *mut std::ffi::c_void, size as usize, nmemb as usize, stream) as u64 }
    #[inline]
    pub unsafe fn fwrite(ptr: *const (), size: u64, nmemb: u64, stream: *mut std::ffi::c_void) -> u64 { c_fwrite(ptr as *const std::ffi::c_void, size as usize, nmemb as usize, stream) as u64 }
    #[inline]
    pub unsafe fn fseek(stream: *mut std::ffi::c_void, offset: i64, whence: i32) -> i32 { c_fseek(stream, offset, whence) }
    #[inline]
    pub unsafe fn fseeko(stream: *mut std::ffi::c_void, offset: i64, whence: i32) -> i32 { c_fseeko(stream, offset, whence) }
    #[inline]
    pub unsafe fn ftell(stream: *mut std::ffi::c_void) -> i64 { c_ftell(stream) }
    #[inline]
    pub unsafe fn ftello(stream: *mut std::ffi::c_void) -> i64 { c_ftello(stream) }
    #[inline]
    pub unsafe fn fflush(stream: *mut std::ffi::c_void) -> i32 { c_fflush(stream) }
    #[inline]
    pub unsafe fn feof(stream: *mut std::ffi::c_void) -> i32 { c_feof(stream) }
    #[inline]
    pub unsafe fn ferror(stream: *mut std::ffi::c_void) -> i32 { c_ferror(stream) }
    #[inline]
    pub unsafe fn clearerr(stream: *mut std::ffi::c_void) { c_clearerr(stream) }
    #[inline]
    pub unsafe fn fileno(stream: *mut std::ffi::c_void) -> i32 { c_fileno(stream) }
    #[inline]
    pub unsafe fn fgetc(stream: *mut std::ffi::c_void) -> i32 { c_fgetc(stream) }
    #[inline]
    pub unsafe fn getc(stream: *mut std::ffi::c_void) -> i32 { c_getc(stream) }
    #[inline]
    pub unsafe fn getchar() -> i32 { c_getchar() }
    #[inline]
    pub unsafe fn fputc(c: i32, stream: *mut std::ffi::c_void) -> i32 { c_fputc(c, stream) }
    #[inline]
    pub unsafe fn putc(c: i32, stream: *mut std::ffi::c_void) -> i32 { c_putc(c, stream) }
    #[inline]
    pub unsafe fn putchar(c: i32) -> i32 { c_putchar(c) }
    #[inline]
    pub unsafe fn ungetc(c: i32, stream: *mut std::ffi::c_void) -> i32 { c_ungetc(c, stream) }
    #[inline]
    pub unsafe fn fputs(s: *const i8, stream: *mut std::ffi::c_void) -> i32 { c_fputs(s, stream) }
    #[inline]
    pub unsafe fn puts(s: *const i8) -> i32 { c_puts(s) }
    #[inline]
    pub unsafe fn fgets(s: *mut i8, n: i32, stream: *mut std::ffi::c_void) -> *mut i8 { c_fgets(s, n, stream) }
    #[inline]
    pub unsafe fn popen(command: *const i8, mode: *const i8) -> *mut std::ffi::c_void { c_popen(command, mode) }
    #[inline]
    pub unsafe fn pclose(stream: *mut std::ffi::c_void) -> i32 { c_pclose(stream) }
    #[inline]
    pub fn fragile_rand_r(seed: *mut u32) -> i32 { unsafe { c_rand_r(seed) } }

    #[repr(C)]
    pub struct fragile_gtest_unit_test {
        pub _opaque: [u8; 0],
    }

    #[inline]
    pub fn fragile_gtest_init(argc: *mut i32, argv: *mut *mut i8) {
        unsafe { c_gtest_init(argc, argv) }
    }

    #[inline]
    pub fn fragile_gtest_run_all_tests() -> i32 {
        unsafe {
            let unit = c_gtest_get_instance();
            if unit.is_null() {
                1
            } else {
                c_gtest_run(unit)
            }
        }
    }
    #[inline]
    pub unsafe fn backtrace(buffer: *mut *mut (), size: i32) -> i32 {
        c_backtrace(buffer as *mut *mut std::ffi::c_void, size)
    }
    #[inline]
    pub unsafe fn backtrace_symbols(
        buffer: *const *mut (),
        size: i32,
    ) -> *mut *mut i8 {
        c_backtrace_symbols(buffer as *const *mut std::ffi::c_void, size)
    }
    #[inline]
    pub unsafe fn fragile_unit_mut() -> &'static mut () {
        &mut FRAGILE_UNIT_SINGLETON
    }

    #[inline]
    pub unsafe fn fragile_zeroed_mut<T>() -> &'static mut T {
        std::boxed::Box::leak(std::boxed::Box::new(std::mem::zeroed::<T>()))
    }
    
    // pthread implementations using Rust std::thread
    
    // Wrapper to make function pointer and arg Send-safe
    struct ThreadStartInfo { func: usize, arg: usize }
    unsafe impl Send for ThreadStartInfo {}
    
    static THREAD_HANDLES: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<u64, std::thread::JoinHandle<usize>>>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    static NEXT_THREAD_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    
    pub unsafe fn fragile_pthread_create(
        thread_id: *mut u64,
        _attr: *const (),
        start_routine: std::option::Option<extern "C" fn(*mut ()) -> *mut ()>,
        arg: *mut (),
    ) -> i32 {
        let func = match start_routine { Some(f) => f, None => return 22 };
        let func_ptr = func as usize;
        let info = ThreadStartInfo { func: func_ptr, arg: arg as usize };
        let tid = NEXT_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let handle = std::thread::spawn(move || {
            let f: extern "C" fn(*mut ()) -> *mut () = std::mem::transmute(info.func);
            let result = f(info.arg as *mut ());
            result as usize
        });
        THREAD_HANDLES.lock().unwrap().insert(tid, handle);
        *thread_id = tid;
        0
    }
    
    pub unsafe fn fragile_pthread_join(thread_id: u64, retval: *mut *mut ()) -> i32 {
        let handle = THREAD_HANDLES.lock().unwrap().remove(&thread_id);
        match handle {
            Some(h) => match h.join() {
                Ok(result) => { if !retval.is_null() { *retval = result as *mut (); } 0 }
                Err(_) => 1
            },
            None => 3
        }
    }
    pub fn fragile_pthread_self() -> u64 { 0 }
    pub fn fragile_pthread_equal(_: u64, _: u64) -> i32 { 1 }
    pub unsafe fn fragile_pthread_detach(_: u64) -> i32 { 0 }
    pub fn fragile_pthread_exit(_: *mut std::ffi::c_void) -> ! { std::process::exit(0) }
    pub unsafe fn fragile_pthread_attr_init(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_attr_destroy(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_attr_setdetachstate(_: *mut std::ffi::c_void, _: i32) -> i32 { 0 }
    pub unsafe fn fragile_pthread_attr_getdetachstate(_: *const std::ffi::c_void, _: *mut i32) -> i32 { 0 }
    pub unsafe fn fragile_pthread_mutex_init(mutex: *mut usize, _: *const super::pthread_mutexattr_t) -> i32 { if mutex.is_null() { return 22; } std::ptr::write(mutex, 0usize); 0 }
    pub unsafe fn fragile_pthread_mutex_destroy(mutex: *mut usize) -> i32 { if mutex.is_null() { return 22; } std::ptr::write(mutex, 0usize); 0 }
    pub unsafe fn fragile_pthread_mutex_lock(mutex: *mut usize) -> i32 { if mutex.is_null() { return 22; } let atomic = &*(mutex as *const std::sync::atomic::AtomicUsize); while atomic.compare_exchange_weak(0, 1, std::sync::atomic::Ordering::Acquire, std::sync::atomic::Ordering::Relaxed).is_err() { std::thread::yield_now(); } 0 }
    pub unsafe fn fragile_pthread_mutex_trylock(mutex: *mut usize) -> i32 { if mutex.is_null() { return 22; } let atomic = &*(mutex as *const std::sync::atomic::AtomicUsize); if atomic.compare_exchange(0, 1, std::sync::atomic::Ordering::Acquire, std::sync::atomic::Ordering::Relaxed).is_ok() { 0 } else { 16 } }
    pub unsafe fn fragile_pthread_mutex_unlock(mutex: *mut usize) -> i32 { if mutex.is_null() { return 22; } let atomic = &*(mutex as *const std::sync::atomic::AtomicUsize); atomic.store(0, std::sync::atomic::Ordering::Release); 0 }
    pub unsafe fn fragile_pthread_mutexattr_init(_: *mut super::pthread_mutexattr_t) -> i32 { 0 }
    pub unsafe fn fragile_pthread_mutexattr_destroy(_: *mut super::pthread_mutexattr_t) -> i32 { 0 }
    pub unsafe fn fragile_pthread_mutexattr_settype(_: *mut super::pthread_mutexattr_t, _: i32) -> i32 { 0 }
    pub unsafe fn fragile_pthread_mutexattr_gettype(_: *const super::pthread_mutexattr_t, _: *mut i32) -> i32 { 0 }
    pub unsafe fn fragile_pthread_cond_init(_: *mut usize, _: *const std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_cond_destroy(_: *mut usize) -> i32 { 0 }
    pub unsafe fn fragile_pthread_cond_wait(_: *mut usize, _: *mut usize) -> i32 { 0 }
    pub unsafe fn fragile_pthread_cond_timedwait(_: *mut usize, _: *mut usize, _: *const std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_cond_signal(_: *mut usize) -> i32 { 0 }
    pub unsafe fn fragile_pthread_cond_broadcast(_: *mut usize) -> i32 { 0 }
    pub unsafe fn fragile_pthread_condattr_init(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_condattr_destroy(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlock_init(_: *mut std::ffi::c_void, _: *const std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlock_destroy(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlock_rdlock(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlock_tryrdlock(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlock_wrlock(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlock_trywrlock(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlock_unlock(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlockattr_init(_: *mut std::ffi::c_void) -> i32 { 0 }
    pub unsafe fn fragile_pthread_rwlockattr_destroy(_: *mut std::ffi::c_void) -> i32 { 0 }
}

// libnuma fallback stubs used by some high-performance runtimes.
// Keep deterministic defaults when NUMA libraries are unavailable at transpile time.
#[inline]
pub fn numa_num_configured_nodes() -> i32 { 1 }

#[inline]
pub fn numa_num_configured_cpus() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(1)
}
