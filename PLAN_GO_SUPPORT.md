# Plan: Go Support with Conservative GC

## Vision

Use Go's official toolchain for parsing/type-checking, then convert to rustc MIR with GC runtime calls. Implement a TinyGo-style conservative non-moving GC in the Fragile runtime.

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   Go Source                                                     │
│       │                                                         │
│       ▼                                                         │
│   ┌───────────────┐                                             │
│   │  go/parser    │  (Official Go parser)                       │
│   │  go/types     │  (Official Go type checker)                 │
│   │  go/ssa       │  (Official Go SSA builder)                  │
│   └───────┬───────┘                                             │
│           │                                                     │
│           ▼                                                     │
│   ┌───────────────┐                                             │
│   │   Go SSA IR   │  (Typed, SSA form)                          │
│   └───────┬───────┘                                             │
│           │                                                     │
│           ▼                                                     │
│   ┌───────────────────────────────────────────────────────┐    │
│   │              Fragile Go → MIR Converter               │    │
│   │                                                       │    │
│   │  • Allocations → fragile_gc_alloc()                  │    │
│   │  • Pointer writes → fragile_gc_write_barrier()       │    │
│   │  • go keyword → fragile_goroutine_spawn()            │    │
│   │  • chan ops → fragile_chan_send/recv()               │    │
│   │  • defer → fragile_defer_push/run()                  │    │
│   │  • interface → fragile_interface_*()                 │    │
│   │                                                       │    │
│   └───────────────────────┬───────────────────────────────┘    │
│                           │                                     │
│                           ▼                                     │
│   ┌───────────────────────────────────────────────────────┐    │
│   │                    rustc MIR                          │    │
│   └───────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Why Use Go's Official Toolchain?

| Component | What It Gives Us |
|-----------|------------------|
| `go/parser` | Full Go syntax support |
| `go/types` | Type inference, type checking |
| `go/ssa` | SSA form (similar to MIR!) |

Go's SSA is already close to MIR - both are SSA form with basic blocks!

## Go SSA vs rustc MIR

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   Go SSA                           rustc MIR                    │
│   ──────                           ─────────                    │
│                                                                 │
│   func add(a, b int) int {         fn add(_1: i32, _2: i32)     │
│   entry:                               -> i32 {                 │
│       t0 = a + b                   bb0: {                       │
│       return t0                        _0 = Add(_1, _2);        │
│   }                                    return;                  │
│                                    }                            │
│                                    }                            │
│                                                                 │
│   Very similar! Both SSA with basic blocks                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## New Crates

```
fragile/crates/
├── fragile-go/                  # Go SSA → rustc MIR
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── parse.rs             # Call go/parser via FFI or subprocess
│       ├── ssa.rs               # Parse Go SSA output
│       └── convert.rs           # SSA → MIR conversion
│
├── fragile-gc/                  # Conservative GC implementation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── heap.rs              # Block-based heap
│       ├── mark.rs              # Conservative mark phase
│       ├── sweep.rs             # Sweep phase
│       └── roots.rs             # Root scanning
│
└── fragile-runtime/             # Extended for Go
    └── src/
        ├── gc.rs                # GC integration
        ├── goroutine.rs         # Goroutine scheduler
        ├── channel.rs           # Channel implementation
        ├── defer.rs             # Defer stack
        └── interface.rs         # Interface dispatch
```

## Conservative GC Design

### Memory Layout

```
┌─────────────────────────────────────────────────────────────────┐
│                        Fragile Heap                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   Block Metadata                         │   │
│  │  ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐    │   │
│  │  │ F │ H │ T │ T │ F │ H │ M │ T │ F │ F │ H │ T │    │   │
│  │  └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘    │   │
│  │    2 bits per block: 00=Free, 01=Head, 10=Tail, 11=Mark│   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   Heap Blocks                            │   │
│  │  ┌────────┬────────┬────────┬────────┬────────┬─────┐  │   │
│  │  │ Block0 │ Block1 │ Block2 │ Block3 │ Block4 │ ... │  │   │
│  │  │ 32 B   │ 32 B   │ 32 B   │ 32 B   │ 32 B   │     │  │   │
│  │  └────────┴────────┴────────┴────────┴────────┴─────┘  │   │
│  │                                                         │   │
│  │  Block size = 4 * sizeof(usize) = 32 bytes on 64-bit   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### GC Implementation

```rust
// fragile-gc/src/lib.rs

#![no_std]

use core::sync::atomic::{AtomicU8, Ordering};

const BLOCK_SIZE: usize = 4 * core::mem::size_of::<usize>();  // 32 bytes on 64-bit

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
enum BlockState {
    Free = 0b00,
    Head = 0b01,
    Tail = 0b10,
    Mark = 0b11,
}

pub struct GcHeap {
    heap_start: *mut u8,
    heap_end: *mut u8,
    metadata: *mut u8,      // 2 bits per block, packed
    num_blocks: usize,
}

impl GcHeap {
    /// Allocate `size` bytes from the GC heap
    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        let blocks_needed = (size + BLOCK_SIZE - 1) / BLOCK_SIZE;

        // Find consecutive free blocks
        if let Some(start_block) = self.find_free_blocks(blocks_needed) {
            // Mark as allocated
            self.set_block_state(start_block, BlockState::Head);
            for i in 1..blocks_needed {
                self.set_block_state(start_block + i, BlockState::Tail);
            }

            return self.block_to_ptr(start_block);
        }

        // No space - trigger GC
        self.collect();

        // Try again
        if let Some(start_block) = self.find_free_blocks(blocks_needed) {
            self.set_block_state(start_block, BlockState::Head);
            for i in 1..blocks_needed {
                self.set_block_state(start_block + i, BlockState::Tail);
            }
            return self.block_to_ptr(start_block);
        }

        // Out of memory
        core::ptr::null_mut()
    }

    /// Run garbage collection
    pub fn collect(&mut self) {
        self.mark_phase();
        self.sweep_phase();
    }

    fn mark_phase(&mut self) {
        // Scan stack for roots
        let stack_bottom = get_stack_bottom();
        let stack_top = get_stack_top();

        self.scan_range(stack_bottom, stack_top);

        // Scan global variables
        let globals = get_globals_range();
        self.scan_range(globals.start, globals.end);
    }

    /// Conservative scan: treat every word as potential pointer
    fn scan_range(&mut self, start: *const usize, end: *const usize) {
        let mut ptr = start;
        while ptr < end {
            let value = unsafe { *ptr };

            // Is this value a pointer into our heap?
            if self.is_heap_pointer(value as *const u8) {
                self.mark_object(value as *mut u8);
            }

            ptr = unsafe { ptr.add(1) };
        }
    }

    fn mark_object(&mut self, ptr: *mut u8) {
        let block = self.ptr_to_block(ptr);

        // Find the head block
        let head = self.find_head_block(block);

        if self.get_block_state(head) == BlockState::Mark {
            return;  // Already marked
        }

        // Mark all blocks of this object
        self.set_block_state(head, BlockState::Mark);

        // Recursively scan object contents
        let obj_start = self.block_to_ptr(head);
        let obj_end = self.find_object_end(head);
        self.scan_range(obj_start as *const usize, obj_end as *const usize);
    }

    fn sweep_phase(&mut self) {
        for block in 0..self.num_blocks {
            match self.get_block_state(block) {
                BlockState::Head => {
                    // Unreached object - free it
                    self.free_object(block);
                }
                BlockState::Mark => {
                    // Reached object - keep it, reset to Head
                    self.set_block_state(block, BlockState::Head);
                }
                _ => {}
            }
        }
    }

    fn free_object(&mut self, head_block: usize) {
        self.set_block_state(head_block, BlockState::Free);

        // Free tail blocks
        let mut block = head_block + 1;
        while block < self.num_blocks &&
              self.get_block_state(block) == BlockState::Tail {
            self.set_block_state(block, BlockState::Free);
            block += 1;
        }
    }

    fn is_heap_pointer(&self, ptr: *const u8) -> bool {
        ptr >= self.heap_start && ptr < self.heap_end
    }
}
```

### Runtime API

```rust
// fragile-runtime/src/gc.rs

static mut GC_HEAP: Option<GcHeap> = None;

/// Initialize the GC heap
#[no_mangle]
pub extern "C" fn fragile_gc_init(heap_size: usize) {
    unsafe {
        GC_HEAP = Some(GcHeap::new(heap_size));
    }
}

/// Allocate from GC heap (called by Go code)
#[no_mangle]
pub extern "C" fn fragile_gc_alloc(size: usize) -> *mut u8 {
    unsafe {
        GC_HEAP.as_mut().unwrap().alloc(size)
    }
}

/// Force a GC collection
#[no_mangle]
pub extern "C" fn fragile_gc_collect() {
    unsafe {
        GC_HEAP.as_mut().unwrap().collect();
    }
}

/// Write barrier (for generational GC, optional)
#[no_mangle]
pub extern "C" fn fragile_gc_write_barrier(dest: *mut *mut u8, value: *mut u8) {
    // For non-moving conservative GC, this can be a no-op
    // Just do the write
    unsafe {
        *dest = value;
    }
}
```

## Go Feature Lowering

### Allocations

```go
// Go
x := new(int)
y := make([]int, 10)
z := &Point{X: 1, Y: 2}
```

```
// MIR
bb0: {
    _x = call fragile_gc_alloc(8);         // size of int

    _y_data = call fragile_gc_alloc(80);   // 10 * 8 bytes
    _y = Slice { data: _y_data, len: 10, cap: 10 };

    _z = call fragile_gc_alloc(16);        // size of Point
    (*_z).X = 1;
    (*_z).Y = 2;
}
```

### Goroutines

```go
// Go
go func() {
    fmt.Println("hello")
}()
```

```
// MIR
bb0: {
    _closure = call fragile_gc_alloc(sizeof(Closure));
    _closure.func = anonymous_func_1;
    _closure.env = ...;

    call fragile_goroutine_spawn(_closure);
    goto bb1;
}
```

### Channels

```go
// Go
ch := make(chan int, 10)
ch <- 42
x := <-ch
```

```
// MIR
bb0: {
    _ch = call fragile_chan_make(8, 10);  // elem_size, capacity

    call fragile_chan_send(_ch, &42);

    _x = call fragile_chan_recv(_ch);
}
```

### Defer

```go
// Go
func example() {
    defer cleanup()
    doWork()
}
```

```
// MIR
fn example() {
    bb0: {
        call fragile_defer_push(cleanup, null);
        call doWork();
        goto bb1;
    }

    bb1: {
        call fragile_defer_run();  // runs all deferred functions
        return;
    }
}
```

### Interfaces

```go
// Go
type Stringer interface {
    String() string
}

func print(s Stringer) {
    fmt.Println(s.String())
}
```

```
// MIR

// Interface represented as fat pointer
struct Interface {
    data: *mut u8,      // pointer to concrete value
    itab: *const ITab,  // interface table
}

struct ITab {
    type_info: *const TypeInfo,
    methods: [fn_ptr; N],  // method pointers
}

fn print(_s: Interface) {
    bb0: {
        // Dynamic dispatch
        _string_fn = _s.itab.methods[0];  // String() is method 0
        _result = call _string_fn(_s.data);
        call fmt_Println(_result);
        return;
    }
}
```

## Goroutine Scheduler

Simple M:N scheduler (can be enhanced later):

```rust
// fragile-runtime/src/goroutine.rs

use alloc::collections::VecDeque;

struct Goroutine {
    id: u64,
    stack: *mut u8,
    stack_size: usize,
    context: Context,  // saved registers
    state: GoroutineState,
}

enum GoroutineState {
    Runnable,
    Running,
    Blocked,
    Dead,
}

struct Scheduler {
    current: Option<Goroutine>,
    run_queue: VecDeque<Goroutine>,
    blocked: Vec<Goroutine>,
    next_id: u64,
}

impl Scheduler {
    pub fn spawn(&mut self, func: fn(*mut u8), arg: *mut u8) {
        let stack = allocate_stack(DEFAULT_STACK_SIZE);
        let goroutine = Goroutine {
            id: self.next_id,
            stack,
            stack_size: DEFAULT_STACK_SIZE,
            context: Context::new(func, arg, stack),
            state: GoroutineState::Runnable,
        };
        self.next_id += 1;
        self.run_queue.push_back(goroutine);
    }

    pub fn yield_current(&mut self) {
        if let Some(mut current) = self.current.take() {
            current.state = GoroutineState::Runnable;
            self.run_queue.push_back(current);
        }
        self.schedule();
    }

    fn schedule(&mut self) {
        if let Some(mut next) = self.run_queue.pop_front() {
            next.state = GoroutineState::Running;
            let prev = self.current.replace(next);

            // Context switch
            if let Some(prev) = prev {
                unsafe { switch_context(&prev.context, &self.current.as_ref().unwrap().context) };
            } else {
                unsafe { restore_context(&self.current.as_ref().unwrap().context) };
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn fragile_goroutine_spawn(closure: *mut Closure) {
    SCHEDULER.lock().spawn(goroutine_entry, closure as *mut u8);
}

extern "C" fn goroutine_entry(arg: *mut u8) {
    let closure = unsafe { &*(arg as *const Closure) };
    (closure.func)(closure.env);

    // Goroutine finished
    SCHEDULER.lock().current.as_mut().unwrap().state = GoroutineState::Dead;
    SCHEDULER.lock().schedule();
}
```

## Channel Implementation

```rust
// fragile-runtime/src/channel.rs

use alloc::collections::VecDeque;

struct Channel {
    elem_size: usize,
    capacity: usize,
    buffer: VecDeque<*mut u8>,

    // Blocked goroutines
    send_waiters: VecDeque<Goroutine>,
    recv_waiters: VecDeque<Goroutine>,

    closed: bool,
}

impl Channel {
    pub fn send(&mut self, value: *const u8) {
        if self.buffer.len() < self.capacity {
            // Buffer has space
            let copy = fragile_gc_alloc(self.elem_size);
            unsafe { core::ptr::copy(value, copy, self.elem_size) };
            self.buffer.push_back(copy);

            // Wake a receiver if any
            if let Some(waiter) = self.recv_waiters.pop_front() {
                SCHEDULER.lock().run_queue.push_back(waiter);
            }
        } else {
            // Buffer full - block
            let current = SCHEDULER.lock().current.take().unwrap();
            self.send_waiters.push_back(current);
            SCHEDULER.lock().schedule();

            // Resumed - do the send
            self.send(value);
        }
    }

    pub fn recv(&mut self) -> *mut u8 {
        if let Some(value) = self.buffer.pop_front() {
            // Wake a sender if any
            if let Some(waiter) = self.send_waiters.pop_front() {
                SCHEDULER.lock().run_queue.push_back(waiter);
            }
            value
        } else {
            // Buffer empty - block
            let current = SCHEDULER.lock().current.take().unwrap();
            self.recv_waiters.push_back(current);
            SCHEDULER.lock().schedule();

            // Resumed - do the recv
            self.recv()
        }
    }
}

#[no_mangle]
pub extern "C" fn fragile_chan_make(elem_size: usize, capacity: usize) -> *mut Channel {
    let ch = fragile_gc_alloc(core::mem::size_of::<Channel>()) as *mut Channel;
    unsafe {
        (*ch) = Channel {
            elem_size,
            capacity,
            buffer: VecDeque::with_capacity(capacity),
            send_waiters: VecDeque::new(),
            recv_waiters: VecDeque::new(),
            closed: false,
        };
    }
    ch
}

#[no_mangle]
pub extern "C" fn fragile_chan_send(ch: *mut Channel, value: *const u8) {
    unsafe { (*ch).send(value) };
}

#[no_mangle]
pub extern "C" fn fragile_chan_recv(ch: *mut Channel) -> *mut u8 {
    unsafe { (*ch).recv() }
}
```

## Cross-Language Interaction

### Go → Rust/C++

```go
// Go calling Rust
//fragile:extern "Rust"
func rust_process(data []byte) int

func main() {
    result := rust_process([]byte("hello"))
}
```

```
// MIR
bb0: {
    // Go slice → raw pointer + length
    _data = fragile_gc_alloc(5);
    copy("hello", _data, 5);

    // Call Rust function with raw pointer
    // Note: Rust must not hold pointer after return (Go might GC it)
    _result = call rust_process(_data, 5);
}
```

### Rust/C++ → Go

```rust
// Rust calling Go
extern "Go" {
    fn go_compute(x: i32) -> i32;
}

fn main() {
    let result = unsafe { go_compute(42) };
}
```

```
// MIR
bb0: {
    // May need to register thread with GC if not already
    call fragile_gc_register_thread();

    _result = call go_compute(42);
}
```

### GC Interaction at Boundaries

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Go Object → Rust                                               │
│  ────────────────                                               │
│                                                                 │
│  1. Go passes pointer to Rust                                   │
│  2. Rust stores pointer (raw pointer, not tracked)              │
│  3. GC runs...                                                  │
│  4. GC scans Rust stack (conservative!)                         │
│  5. Finds the pointer → marks Go object as live                 │
│  6. Object NOT collected, NOT moved                             │
│  7. Rust can still use pointer safely!                          │
│                                                                 │
│  This works because:                                            │
│  • Conservative GC scans ALL memory for pointers                │
│  • Non-moving GC doesn't relocate objects                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Go Parsing (2 iterations)

1. Call `go/parser` and `go/types` via Go subprocess
2. Output Go SSA as JSON/binary format
3. Parse in Rust

```bash
# Helper tool written in Go
fragile-go-frontend input.go --output-ssa=output.ssa
```

### Phase 2: Conservative GC (3 iterations)

1. Block-based heap allocator
2. Conservative mark phase (scan stack/globals)
3. Sweep phase

### Phase 3: SSA → MIR Converter (4 iterations)

1. Basic values and operations
2. Control flow (if, for, switch)
3. Function calls
4. Allocations with GC

### Phase 4: Go Runtime Features (4 iterations)

1. Goroutine scheduler
2. Channels
3. Defer
4. Interfaces

### Phase 5: Integration (2 iterations)

1. Cross-language calls
2. Unified build system

## Summary

| Component | Approach |
|-----------|----------|
| Parsing | Go's `go/parser`, `go/types`, `go/ssa` |
| GC | Conservative, non-moving (like TinyGo) |
| Goroutines | M:N scheduler in Fragile runtime |
| Channels | Runtime implementation |
| Conversion | Go SSA → rustc MIR |

This gives us full Go support while maintaining seamless interop with Rust and C++!
