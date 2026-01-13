# Fragile TODO

## Phase 1: Foundation (Completed)
- [x] Project structure with cargo workspaces
- [x] Tree-sitter integration for Rust, C++, Go
- [x] Unified HIR definitions
- [x] Frontend lowering for all three languages
- [x] Basic LLVM codegen
- [x] CLI interface

## Phase 2: Core Language Features (Mostly Complete)
- [x] Structs (Rust and C++)
- [x] Enums with data variants (Option, Result style)
- [x] Generic functions with monomorphization
- [x] Generic enums with monomorphization
- [x] Pattern matching (match expressions, destructuring)
- [x] Impl blocks and methods
- [x] Closures (without captures)
- [x] Module system (mod, use, file resolution)
- [x] Const and static items
- [x] Type aliases
- [x] Arrays and indexing
- [x] Raw pointers and references
- [x] Loops with break values
- [x] Tuples
- [ ] Closure captures (variables from enclosing scope)
- [ ] Trait implementations (impl Trait for Type)
- [ ] Associated functions (Type::new())
- [ ] String/str types

## Phase 3: Type System
- [x] Unified type representation (HIR Type enum)
- [x] Basic type inference for generics
- [x] Integer type coercion (i32/i64)
- [ ] Full type inference
- [ ] Type checking pass with error reporting
- [ ] Lifetime inference (basic)

## Phase 4: Interoperability
- [x] extern "C" blocks
- [x] Basic name mangling (Type_method)
- [ ] Cross-language function calls
- [ ] Struct layout compatibility
- [ ] Interface/trait unification
- [ ] C++ class support
- [ ] Go interface support

## Phase 5: Advanced Features
- [x] Generics/templates (Rust)
- [x] Pattern matching
- [x] Error handling (Result<T,E>)
- [ ] Async/await
- [ ] Goroutine support
- [ ] C++ exceptions interop
- [ ] Operator overloading

## Phase 6: Standard Library
- [ ] libc bindings
- [ ] Minimal runtime
- [ ] String implementation
- [ ] Vec/slice utilities
- [ ] I/O primitives

## Current Test Coverage
35 test files covering:
- Primitives, structs, enums
- Generics and monomorphization
- Pattern matching and destructuring
- Modules and visibility
- Closures, const/static
- References and pointers
- Loops and control flow

---

## Next Steps (Priority Order)

### Immediate (High Impact)
1. **Trait implementations** - Enable `impl Trait for Type` to work
   - Parse impl blocks with trait names
   - Generate trait method dispatch
   - Required for idiomatic Rust patterns

2. **Associated functions** - Support `Type::new()` static methods
   - Detect functions without self parameter
   - Generate as regular functions with mangled names

3. **Closure captures** - Allow closures to reference outer variables
   - Track captured variables during lowering
   - Generate closure struct with captured values

### Short Term
4. **String/str types** - Basic string support
   - Add `&str` as pointer + length
   - String literals as static data

5. **Type checking pass** - Add validation before codegen
   - Type mismatch errors
   - Undefined variable/function errors
   - Better error messages with spans

### Medium Term
6. **C++ templates** - Extend monomorphization to C++
7. **Cross-language calls** - Rust calling C++, Go calling Rust
8. **Vec type** - Dynamic arrays with heap allocation

---

## Open Design Questions
1. How should C++ exceptions interact with Rust's Result/panic?
2. Should Go goroutines be supported, or simplified to threads?
3. How to handle C++ templates vs Rust generics vs Go generics?
4. What's the memory model for mixed-language objects?
