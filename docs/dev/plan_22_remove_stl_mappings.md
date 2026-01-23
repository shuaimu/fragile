# Plan: Section 22 - Remove STL Type Mappings

## Overview

This plan covers removing special-case STL type mappings from `types.rs`. The goal is to treat STL types like any other C++ code - they should be transpiled from their actual implementation, not mapped to Rust equivalents.

## Current State

The file `crates/fragile-clang/src/types.rs` contains mappings at lines ~255-500:
- `std::string` → `String`
- `std::vector<T>` → `Vec<T>`
- `std::optional<T>` → `Option<T>`
- `std::array<T, N>` → `[T; N]`
- `std::span<T>` → `&[T]` / `&mut [T]`
- `std::map<K,V>` → `BTreeMap<K,V>`
- `std::unordered_map<K,V>` → `HashMap<K,V>`
- `std::unique_ptr<T>` → `Box<T>`
- `std::shared_ptr<T>` → `Arc<T>`
- `std::weak_ptr<T>` → `Weak<T>`
- `std::variant<...>` → synthetic enum
- I/O stream types → `std::io` equivalents

## Strategy

### Option A: Remove mappings, let types pass through
- Remove all STL-specific handling
- `std::vector<int>` stays as `std::vector<int>` in generated Rust
- Requires transpiling actual STL implementation (complex)

### Option B: Remove mappings, but keep stubs (Recommended for now)
- Remove mappings from `types.rs`
- Generate Rust module `std` with stub structs/traits
- `std::vector<T>` → `crate::std::vector<T>` (stub struct)
- Stubs provide minimal API compatibility

### Option C: Gradual migration
- Remove one type at a time
- Test thoroughly after each removal
- Allows controlled rollout

## Implementation Plan (Option C)

### Phase 1: Prepare infrastructure
1. Create stub module structure for STL types
2. Add feature flag to toggle old vs new behavior
3. Update tests to use feature flag

### Phase 2: Remove mappings one by one
Each removal follows the pattern:
1. Remove the mapping code from `types.rs`
2. Add corresponding stub type if needed
3. Update affected tests
4. Verify all tests pass

### Phase 3: Clean up
1. Remove feature flag
2. Remove any unused stub types
3. Update documentation

## Risk Assessment

**High Risk**:
- Tests expecting `Vec<T>` output will fail
- Code relying on Rust std lib methods won't compile
- Breaking change for existing users

**Mitigation**:
- Run full test suite after each change
- Create comprehensive stub types
- Provide migration guide

## Estimated Effort

- Phase 1: 2-3 hours (stub infrastructure)
- Phase 2: 1-2 hours per type category (8 categories = ~12 hours)
- Phase 3: 1 hour (cleanup)

Total: ~15-18 hours of work
