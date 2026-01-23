# Plan: Task 22.5 - Categorize libc++ Components

## Objective

Categorize libc++ components by transpilation complexity to prioritize implementation effort.

## Categories

### 22.5.1 Header-only (Easy) - Templates, inline functions only

These headers are primarily templates that get instantiated at compile time.
No corresponding `.cpp` file needed.

| Header | Description | Priority |
|--------|-------------|----------|
| `<vector>` | Dynamic array | High |
| `<array>` | Fixed-size array | High |
| `<map>` | Ordered associative container | Medium |
| `<set>` | Ordered set | Medium |
| `<unordered_map>` | Hash map | High |
| `<unordered_set>` | Hash set | Medium |
| `<deque>` | Double-ended queue | Low |
| `<list>` | Doubly-linked list | Low |
| `<forward_list>` | Singly-linked list | Low |
| `<queue>` | Queue adapter | Low |
| `<stack>` | Stack adapter | Low |
| `<span>` | Non-owning view | Medium |
| `<algorithm>` | STL algorithms | High |
| `<numeric>` | Numeric algorithms | Medium |
| `<iterator>` | Iterator utilities | High |
| `<ranges>` | C++20 ranges | Medium |
| `<memory>` | Smart pointers (partial) | High |
| `<utility>` | Pair, move, forward | High |
| `<tuple>` | Tuple type | Medium |
| `<optional>` | Optional wrapper | High |
| `<variant>` | Type-safe union | Medium |
| `<any>` | Type-erased container | Low |
| `<expected>` | C++23 expected | Low |
| `<functional>` | Function objects | Medium |
| `<type_traits>` | Compile-time type info | High |
| `<concepts>` | C++20 concepts | Low |
| `<ratio>` | Compile-time rationals | Low |
| `<limits>` | Numeric limits | Medium |
| `<initializer_list>` | Initializer lists | High |
| `<compare>` | Three-way comparison | Low |
| `<bitset>` | Fixed-size bit array | Low |
| `<mdspan>` | Multidimensional span | Low |
| `<numbers>` | Math constants | Low |
| `<source_location>` | Source location | Low |
| `<flat_map>` | C++23 flat map | Low |
| `<flat_set>` | C++23 flat set | Low |
| `<latch>` | Synchronization latch | Low |
| `<barrier>` | Synchronization barrier | Low |
| `<semaphore>` | Semaphore | Low |
| `<stop_token>` | Cooperative cancellation | Low |

### 22.5.2 Partial src (Medium) - Some compiled components

These headers have both template code (in headers) and non-template code
(in `src/*.cpp`). Need to transpile both.

| Header | Src File | Description | Priority |
|--------|----------|-------------|----------|
| `<string>` | `string.cpp` | String class | High |
| `<locale>` | `locale.cpp` | Localization | Low |
| `<regex>` | `regex.cpp` | Regular expressions | Low |
| `<chrono>` | `chrono.cpp` | Time utilities | Medium |
| `<exception>` | `exception.cpp` | Exception handling | Medium |
| `<new>` | `new.cpp`, `new_handler.cpp`, `new_helpers.cpp` | Memory allocation | High |
| `<typeinfo>` | `typeinfo.cpp` | RTTI support | Low |
| `<stdexcept>` | `stdexcept.cpp` | Standard exceptions | Medium |
| `<system_error>` | `system_error.cpp` | Error handling | Medium |
| `<future>` | `future.cpp` | Async support | Low |
| `<memory_resource>` | `memory_resource.cpp` | PMR allocators | Low |
| `<random>` | `random.cpp` | Random numbers | Low |
| `<valarray>` | `valarray.cpp` | Numeric arrays | Low |
| `<variant>` | `variant.cpp` | Type-safe union impl | Medium |
| `<optional>` | `optional.cpp` | Optional impl | Medium |
| `<any>` | `any.cpp` | Type-erased impl | Low |
| `<charconv>` | `charconv.cpp` | Number conversion | Medium |
| `<format>` | N/A | Formatting | Low |
| `<print>` | `print.cpp` | C++23 print | Low |

### 22.5.3 OS Interface (Hard) - System dependencies

These components interact with the operating system and require Rust
equivalents or FFI wrappers.

| Header | Src Files | Description | Priority | Rust Equivalent |
|--------|-----------|-------------|----------|-----------------|
| `<iostream>` | `iostream.cpp`, `ios.cpp`, `ostream.cpp` | Console I/O | High | `std::io` |
| `<fstream>` | `fstream.cpp` | File I/O | High | `std::fs::File` |
| `<sstream>` | N/A (template) | String streams | Medium | `Cursor<Vec<u8>>` |
| `<istream>` | N/A | Input stream | High | `Read` trait |
| `<ostream>` | `ostream.cpp` | Output stream | High | `Write` trait |
| `<streambuf>` | N/A | Stream buffers | Medium | N/A |
| `<iomanip>` | N/A | Stream manipulators | Medium | N/A |
| `<syncstream>` | N/A | Synchronized streams | Low | N/A |
| `<thread>` | `thread.cpp` | Threading | Medium | `std::thread` |
| `<mutex>` | `mutex.cpp`, `mutex_destructor.cpp` | Mutexes | Medium | `std::sync::Mutex` |
| `<shared_mutex>` | `shared_mutex.cpp` | RW locks | Medium | `std::sync::RwLock` |
| `<condition_variable>` | `condition_variable.cpp` | Condvars | Medium | `std::sync::Condvar` |
| `<atomic>` | `atomic.cpp` | Atomics | Medium | `std::sync::atomic` |
| `<filesystem>` | `src/filesystem/*` | File system ops | Low | `std::fs`, `std::path` |

### 22.5.4 Priority List (by common usage)

**Tier 1 - Most commonly used (implement first):**
1. `<vector>` - Dynamic arrays
2. `<string>` - Strings
3. `<algorithm>` - STL algorithms
4. `<memory>` - Smart pointers
5. `<iostream>` - Console I/O

**Tier 2 - Frequently used:**
6. `<unordered_map>` - Hash maps
7. `<map>` - Ordered maps
8. `<optional>` - Optional values
9. `<fstream>` - File I/O
10. `<chrono>` - Time handling

**Tier 3 - Moderately used:**
11. `<functional>` - Function objects
12. `<tuple>` - Tuples
13. `<set>` / `<unordered_set>` - Sets
14. `<variant>` - Tagged unions
15. `<thread>` / `<mutex>` - Threading

**Tier 4 - Less common:**
- Everything else

## Implementation Strategy

1. **Start with header-only containers** - `<vector>`, `<array>`, `<algorithm>`
   - These just need template instantiation to work
   - Test with vendored libc++ headers

2. **Add memory/allocation support** - `<memory>`, `<new>`
   - Critical for smart pointers
   - May need custom allocation wrappers

3. **Add I/O support** - `<iostream>`, `<fstream>`
   - Map to Rust `std::io` traits
   - May need adapters

4. **Add threading support** - `<thread>`, `<mutex>`, `<atomic>`
   - Map to Rust `std::thread`, `std::sync`
   - Need careful handling for thread safety

## Status

Task 22.5 is primarily documentation. The actual implementation happens
in Phase 4 (22.7+) and Phase 5 (22.13).
