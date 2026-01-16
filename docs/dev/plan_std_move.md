# Plan: std::move and std::forward Support (A.5.4, A.5.5)

**Status:** Completed [26:01:16, 00:58]

## Design Rationale

### Problem
C++ `std::move` and `std::forward` are essential for move semantics and perfect forwarding. They enable efficient transfer of resources by converting lvalues to rvalue references.

### What std::move Actually Does
```cpp
template<typename T>
constexpr remove_reference_t<T>&& move(T&& t) noexcept {
    return static_cast<remove_reference_t<T>&&>(t);
}
```

It's NOT a special operation - it's just a cast that:
1. Signals intent to move (not copy) the value
2. Enables function overload resolution to prefer move constructors/assignment

### Solution
Since `std::move` and `std::forward` are just casts to rvalue reference, we:
1. Recognize these function calls in the AST
2. Convert the argument to MIR
3. Wrap the result in `MirOperand::Move` (vs `MirOperand::Copy`)

### Design Decisions
1. **Builtin recognition**: Treat std::move/forward as compiler builtins rather than parsing their template definitions
2. **Name matching**: Match both qualified (`std::move`) and unqualified (`move`) forms
3. **Simple conversion**: `std::move(x)` → `MirOperand::Move(place_of_x)`
4. **Forward simplification**: For now, treat `std::forward` like `std::move` (full reference collapsing not yet implemented)

## Implementation Summary

### Files Changed

1. **crates/fragile-clang/src/convert.rs** (~40 lines)
   - Added std::move/forward recognition in `CallExpr` handling
   - Added `is_std_move()` and `is_std_forward()` helper functions
   - Added `Unknown(_)` handling to unwrap UnexposedExpr nodes

2. **crates/fragile-clang/tests/integration_test.rs** (~100 lines)
   - `test_std_move_basic`: Basic std::move generates Move operand
   - `test_std_forward_basic`: std::forward parsing works
   - `test_std_move_in_call`: Move operand passed to function

3. **docs/dev/plan_std_move.md** (this file)

### Test Coverage
- 3 new integration tests for std::move/forward
- All 86 tests passing

## User Guide

### Basic Usage
std::move is automatically recognized and converted:

```cpp
#include <utility>

int test() {
    int x = 42;
    int y = std::move(x);  // Generates MirOperand::Move
    return y;
}
```

### In Function Calls
Move semantics work in function call arguments:

```cpp
void consume(int&& val);

void test() {
    int x = 10;
    consume(std::move(x));  // Argument is MirOperand::Move
}
```

### MIR Output
Before: `MirOperand::Copy(place)`
After std::move: `MirOperand::Move(place)`

The distinction allows the rustc codegen to optimize ownership transfer.

## Limitations

- **No reference collapsing**: `std::forward` is simplified to always move
- **Template-dependent**: Full perfect forwarding requires template argument tracking
- **Overload resolution**: Move vs copy constructor selection happens at rustc level
