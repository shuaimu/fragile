# Plan: C++20/23 Support for Compiling Mako

## Goal

Enable Fragile to compile the [Mako](https://github.com/makodb/mako) distributed transactional key-value store, a C++23 project with ~18 .cpp files and ~344 headers.

## Mako Project Analysis

### Project Stats
- **C++ Standard**: C++23 (requires GCC 12+)
- **Source Files**: ~18 .cpp, ~344 .h/.hpp
- **Key Dependencies**: DPDK, Boost, RocksDB, eRPC, Python3

### C++ Features Used (by frequency)

| Category | Features | Occurrences |
|----------|----------|-------------|
| **Containers** | vector, string, map, unordered_map, list, set | 5000+ |
| **OOP** | virtual, override, classes, inheritance | 2400+ |
| **Concurrency** | atomic, mutex, thread, condition_variable | 400+ |
| **Smart Pointers** | unique_ptr, shared_ptr + custom RustyCpp types | 500+ |
| **Coroutines** | co_await, coroutine_handle, promise_type | 50+ |
| **Templates** | Function/class templates, SFINAE, concepts | 1000+ |

### C++20/23 Specific Features

1. **Coroutines** (Critical)
   - `<coroutine>` header
   - `co_await`, `co_return`, `co_yield`
   - `std::coroutine_handle<>`
   - `std::suspend_always`, `std::suspend_never`
   - Promise types

2. **Concepts & Constraints**
   - `requires` clauses
   - `std::is_base_of_v`, `std::is_trivially_copyable_v`

3. **Modern Features**
   - Structured bindings: `auto [time, task] = tasks.top();`
   - `constexpr` functions
   - `[[nodiscard]]`, `[[maybe_unused]]` attributes
   - Designated initializers

4. **Type Traits & Metaprogramming**
   - `<type_traits>` heavily used
   - SFINAE patterns
   - `std::enable_if_t`, `std::conditional_t`

---

## Implementation Phases

### Phase A: Core C++ Infrastructure (Iterations 1-5)

#### A.1: Namespaces
- [ ] Parse `namespace foo { }` declarations
- [ ] Parse `namespace foo::bar { }` (nested)
- [ ] Parse `using namespace foo;`
- [ ] Name resolution across namespaces
- [ ] Anonymous namespaces

#### A.2: Classes Complete
- [ ] Access specifiers (public/private/protected)
- [ ] Constructors (default, parameterized, copy, move)
- [ ] Destructors
- [ ] Copy/move assignment operators
- [ ] Member initializer lists
- [ ] Static members
- [ ] Friend declarations

#### A.3: Inheritance
- [ ] Single inheritance
- [ ] Multiple inheritance
- [ ] Virtual inheritance
- [ ] Virtual functions and vtables
- [ ] Pure virtual functions (= 0)
- [ ] Override/final specifiers
- [ ] Base class initialization
- [ ] Upcasting/downcasting

#### A.4: Operator Overloading
- [ ] Binary operators (+, -, *, /, etc.)
- [ ] Comparison operators (==, !=, <, >, etc.)
- [ ] Assignment operators (=, +=, etc.)
- [ ] Subscript operator []
- [ ] Function call operator ()
- [ ] Dereference/arrow operators (*, ->)
- [ ] Increment/decrement (++, --)
- [ ] Stream operators (<<, >>)

#### A.5: References & Const
- [ ] Lvalue references (T&)
- [ ] Const references (const T&)
- [ ] Rvalue references (T&&)
- [ ] std::move semantics
- [ ] std::forward (perfect forwarding)
- [ ] Const member functions
- [ ] Mutable keyword

---

### Phase B: Templates (Iterations 6-10)

#### B.1: Function Templates
- [ ] Basic function templates
- [ ] Template argument deduction
- [ ] Explicit template instantiation
- [ ] Template specialization
- [ ] Variadic templates

#### B.2: Class Templates
- [ ] Basic class templates
- [ ] Template member functions
- [ ] Partial specialization
- [ ] Full specialization
- [ ] Nested templates

#### B.3: SFINAE & Type Traits
- [ ] std::enable_if
- [ ] std::is_same, std::is_base_of
- [ ] std::is_trivially_copyable
- [ ] std::conditional
- [ ] decltype and declval

#### B.4: C++20 Concepts (Basic)
- [ ] `requires` clauses on functions
- [ ] `requires` expressions
- [ ] Standard concepts (std::integral, etc.)
- [ ] Custom concept definitions

#### B.5: Template Metaprogramming
- [ ] Compile-time computation
- [ ] Type lists
- [ ] constexpr functions
- [ ] consteval (C++20)
- [ ] constinit (C++20)

---

### Phase C: Standard Library (Iterations 11-18)

#### C.1: Containers
- [ ] std::vector
- [ ] std::string
- [ ] std::map / std::unordered_map
- [ ] std::set / std::unordered_set
- [ ] std::list
- [ ] std::array
- [ ] std::pair / std::tuple
- [ ] std::optional
- [ ] std::variant

#### C.2: Smart Pointers
- [ ] std::unique_ptr
- [ ] std::shared_ptr
- [ ] std::weak_ptr
- [ ] std::make_unique / std::make_shared
- [ ] Custom deleters

#### C.3: Algorithms & Utilities
- [ ] std::move / std::forward
- [ ] std::min / std::max
- [ ] std::sort / std::find
- [ ] std::function
- [ ] std::bind
- [ ] Range-based for loops

#### C.4: Concurrency
- [ ] std::thread
- [ ] std::mutex / std::lock_guard / std::unique_lock
- [ ] std::condition_variable
- [ ] std::atomic
- [ ] std::future / std::promise
- [ ] std::async

#### C.5: I/O Streams
- [ ] std::cout / std::cerr / std::cin
- [ ] std::stringstream
- [ ] std::fstream
- [ ] Stream operators (<<, >>)

#### C.6: Chrono
- [ ] std::chrono::duration
- [ ] std::chrono::time_point
- [ ] std::chrono::steady_clock / system_clock
- [ ] Duration literals (1s, 1ms, 1us)

#### C.7: Type Support
- [ ] std::type_info
- [ ] typeid operator
- [ ] dynamic_cast
- [ ] static_cast / reinterpret_cast / const_cast

#### C.8: Memory
- [ ] std::allocator
- [ ] Placement new
- [ ] std::align
- [ ] Memory ordering (std::memory_order)

---

### Phase D: C++20 Coroutines (Iterations 19-23)

#### D.1: Coroutine Infrastructure
- [ ] `<coroutine>` header support
- [ ] std::coroutine_handle
- [ ] std::coroutine_traits
- [ ] std::suspend_always / std::suspend_never

#### D.2: Promise Types
- [ ] promise_type definition
- [ ] get_return_object()
- [ ] initial_suspend() / final_suspend()
- [ ] return_void() / return_value()
- [ ] unhandled_exception()
- [ ] yield_value()

#### D.3: Awaitables
- [ ] await_ready()
- [ ] await_suspend()
- [ ] await_resume()
- [ ] co_await expression
- [ ] Awaitable concept

#### D.4: Generators
- [ ] co_yield expression
- [ ] Generator pattern
- [ ] std::generator (C++23)

#### D.5: Async Patterns
- [ ] Task type pattern
- [ ] Lazy coroutines
- [ ] Eager coroutines
- [ ] Coroutine schedulers

---

### Phase E: Advanced Features (Iterations 24-28)

#### E.1: Exceptions
- [ ] try/catch/throw
- [ ] Exception specifications (noexcept)
- [ ] std::exception hierarchy
- [ ] Stack unwinding
- [ ] Exception safety

#### E.2: RTTI
- [ ] typeid operator
- [ ] std::type_info
- [ ] dynamic_cast
- [ ] Type name demangling

#### E.3: Lambdas
- [ ] Basic lambdas
- [ ] Capture by value/reference
- [ ] Capture with initializer
- [ ] Generic lambdas (auto parameters)
- [ ] Mutable lambdas
- [ ] Lambda return type

#### E.4: Attributes
- [ ] [[nodiscard]]
- [ ] [[maybe_unused]]
- [ ] [[deprecated]]
- [ ] [[fallthrough]]
- [ ] [[likely]] / [[unlikely]]
- [ ] Custom attributes

#### E.5: Miscellaneous
- [ ] static_assert
- [ ] alignas / alignof
- [ ] Bit fields
- [ ] Union types
- [ ] Designated initializers
- [ ] Aggregate initialization

---

### Phase F: Mako-Specific Integration (Iterations 29-32)

#### F.1: RustyCpp Types
- [ ] rusty::Box<T> → Fragile Box equivalent
- [ ] rusty::Arc<T> → Fragile Arc equivalent
- [ ] rusty::Cell<T> → Fragile Cell equivalent
- [ ] rusty::Option<T> → Fragile Option equivalent
- [ ] rusty::Mutex<T> → Fragile Mutex equivalent

#### F.2: External Dependencies
- [ ] Boost headers (system, filesystem, thread, coroutine)
- [ ] RocksDB integration
- [ ] DPDK headers (optional)
- [ ] eRPC headers (optional)

#### F.3: Build System
- [ ] CMake integration
- [ ] Compile flags support (-std=c++23, -O2, etc.)
- [ ] Include path handling
- [ ] Library linking

#### F.4: Testing & Validation
- [ ] Compile Mako source files individually
- [ ] Link Mako components
- [ ] Run Mako tests
- [ ] Benchmark comparison

---

## Priority Order for Mako

Based on Mako's actual usage, prioritize:

### Critical Path (Must Have)
1. **Classes + Inheritance + Virtual** - Core OOP
2. **Templates** - Used everywhere
3. **STL Containers** - vector, map, string
4. **Smart Pointers** - unique_ptr, shared_ptr
5. **Concurrency** - thread, mutex, atomic
6. **Coroutines** - Used for async operations

### Important (Should Have)
7. **Namespaces** - Project organization
8. **Lambdas** - Callbacks everywhere
9. **RAII/Destructors** - Memory management
10. **Type Traits** - Template metaprogramming

### Nice to Have
11. **Concepts** - Some usage
12. **Exceptions** - Error handling
13. **RTTI** - Dynamic dispatch

---

## Testing Strategy

### Unit Tests
For each feature, create minimal test cases:

```cpp
// tests/cpp20/coroutine_basic.cpp
#include <coroutine>

struct Task {
    struct promise_type {
        Task get_return_object() { return {}; }
        std::suspend_never initial_suspend() { return {}; }
        std::suspend_never final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
    };
};

Task simple_task() {
    co_return;
}

int main() {
    simple_task();
    return 0;
}
```

### Integration Tests
Compile actual Mako source files:

```bash
# Phase 1: Single file compilation
fragile compile vendor/mako/src/rrr/misc/rand.cpp

# Phase 2: Multiple files
fragile compile vendor/mako/src/rrr/misc/*.cpp

# Phase 3: Full module
fragile compile vendor/mako/src/rrr/**/*.cpp

# Phase 4: Full project
fragile build vendor/mako
```

### Compatibility Matrix

| Mako File | Dependencies | Priority |
|-----------|--------------|----------|
| `src/rrr/misc/rand.cpp` | Minimal | 1 - Start here |
| `src/rrr/misc/marshal.cpp` | Templates, STL | 2 |
| `src/rrr/rpc/server.cpp` | Networking, threads | 3 |
| `src/mako/vec/coroutine.cpp` | Coroutines | 4 |
| Full project | Everything | Final |

---

## Success Metrics

1. **Milestone 1**: Compile `rand.cpp` (minimal deps)
2. **Milestone 2**: Compile `rrr/misc/*.cpp` (templates, STL)
3. **Milestone 3**: Compile `rrr/rpc/*.cpp` (OOP, threads)
4. **Milestone 4**: Compile `mako/vec/*.cpp` (coroutines)
5. **Milestone 5**: Full Mako build
6. **Milestone 6**: Mako tests pass

---

## Estimated Effort

| Phase | Iterations | Description |
|-------|------------|-------------|
| A | 5 | Core C++ Infrastructure |
| B | 5 | Templates |
| C | 8 | Standard Library |
| D | 5 | C++20 Coroutines |
| E | 5 | Advanced Features |
| F | 4 | Mako Integration |
| **Total** | **32** | |

---

## Next Steps

1. Start with Phase A.1 (Namespaces) - Mako uses namespaces extensively
2. Create test files for each C++ feature
3. Implement features incrementally, testing against Mako files
4. Track progress using the compatibility matrix
