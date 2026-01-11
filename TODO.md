# Fragile TODO

## Phase 1: Foundation (Completed)
- [x] Project structure with cargo workspaces
- [x] Tree-sitter integration for Rust, C++, Go
- [x] Unified HIR definitions
- [x] Frontend lowering for all three languages
- [x] Basic LLVM codegen
- [x] CLI interface

## Phase 2: Specification & Documentation
- [ ] Write fragile-book (language specification)
  - [ ] Cross-language calling conventions
  - [ ] Type mapping between languages
  - [ ] Memory management interop (Rust ownership, Go GC, C++ RAII)
  - [ ] ABI specification
  - [ ] Error handling across language boundaries

## Phase 3: Type System
- [ ] Implement type inference
- [ ] Add type checking pass
- [ ] Unified type representation
- [ ] Type coercion rules

## Phase 4: Interoperability
- [ ] Cross-language function resolution
- [ ] Name mangling scheme
- [ ] Struct/class interop
- [ ] Interface/trait unification
- [ ] Memory management bridges

## Phase 5: Advanced Features
- [ ] Generics/templates
- [ ] Pattern matching
- [ ] Async/goroutines
- [ ] Error handling (Result, exceptions, error returns)

## Phase 6: Standard Library
- [ ] libc bindings
- [ ] Minimal runtime
- [ ] Cross-language standard types

## Open Design Questions
1. How should C++ exceptions interact with Rust's Result/panic?
2. Should Go goroutines be supported, or simplified to threads?
3. How to handle C++ templates vs Rust generics vs Go generics?
4. What's the memory model for mixed-language objects?
