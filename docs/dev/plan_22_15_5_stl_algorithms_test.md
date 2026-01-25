# Plan: Task 22.15.5 - Test STL Algorithms

## Overview

Add E2E tests for STL algorithm operations (`std::sort`, `std::find`, `std::copy`, etc.), following the same pattern as previous STL stub tasks.

## Approach

Since STL algorithms operate on iterator ranges, we'll implement stub functions that work with pointers (which are the C++ iterator equivalent for contiguous containers). The stubs will use Rust slices internally for the actual operations.

## Algorithm Stub Designs

### std_sort (sorting)
```rust
// std::sort(first, last) - sorts range [first, last)
pub fn std_sort_int(first: *mut i32, last: *mut i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    slice.sort();
}
```

### std_find (searching)
```rust
// std::find(first, last, value) - returns iterator to first match or last
pub fn std_find_int(first: *const i32, last: *const i32, value: i32) -> *const i32 {
    if first.is_null() || last.is_null() { return last; }
    let len = unsafe { last.offset_from(first) as usize };
    let slice = unsafe { std::slice::from_raw_parts(first, len) };
    match slice.iter().position(|&x| x == value) {
        Some(idx) => unsafe { first.add(idx) },
        None => last,
    }
}
```

### std_count (counting)
```rust
// std::count(first, last, value) - counts occurrences of value
pub fn std_count_int(first: *const i32, last: *const i32, value: i32) -> usize {
    if first.is_null() || last.is_null() { return 0; }
    let len = unsafe { last.offset_from(first) as usize };
    let slice = unsafe { std::slice::from_raw_parts(first, len) };
    slice.iter().filter(|&&x| x == value).count()
}
```

### std_copy (copying)
```rust
// std::copy(first, last, dest) - copies range to dest, returns end of dest
pub fn std_copy_int(first: *const i32, last: *const i32, dest: *mut i32) -> *mut i32 {
    if first.is_null() || last.is_null() || dest.is_null() { return dest; }
    let len = unsafe { last.offset_from(first) as usize };
    unsafe { std::ptr::copy_nonoverlapping(first, dest, len); }
    unsafe { dest.add(len) }
}
```

### std_fill (filling)
```rust
// std::fill(first, last, value) - fills range with value
pub fn std_fill_int(first: *mut i32, last: *mut i32, value: i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    for elem in slice.iter_mut() {
        *elem = value;
    }
}
```

### std_reverse (reversing)
```rust
// std::reverse(first, last) - reverses range in place
pub fn std_reverse_int(first: *mut i32, last: *mut i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    slice.reverse();
}
```

## Test Cases

### std_sort tests:
- Sort empty range (no-op)
- Sort single element (no-op)
- Sort already sorted array
- Sort reverse-sorted array
- Sort random order array

### std_find tests:
- Find in empty range (returns end)
- Find existing element (returns pointer to it)
- Find non-existing element (returns end)
- Find first of duplicates

### std_count tests:
- Count in empty range (returns 0)
- Count non-existing value (returns 0)
- Count single occurrence
- Count multiple occurrences

### std_copy tests:
- Copy empty range
- Copy to separate buffer
- Verify original unchanged

### std_fill tests:
- Fill empty range (no-op)
- Fill with value
- Fill with zero

### std_reverse tests:
- Reverse empty range (no-op)
- Reverse single element (no-op)
- Reverse even length array
- Reverse odd length array

## Implementation Steps

1. Add algorithm stub functions in preamble (~100 LOC)
2. Add E2E test validating all algorithms (~120 LOC)

## Estimated LOC

- Algorithm stubs: ~100 LOC
- E2E tests: ~120 LOC
- Total: ~220 LOC (< 500 LOC threshold)
