# Things That Are WRONG - Do Not Do These

This document lists anti-patterns and shortcuts that are **strictly forbidden** in the Fragile transpiler. Every commit should be checked against this list.

---

## 1. Rollback Patterns (FORBIDDEN)

### What it is
"Rollback patterns" delete generated code when it contains certain string patterns that would cause compilation errors.

```rust
// WRONG - This is in ast_codegen.rs and must be removed
let generated = &self.output[method_output_start..];
if generated.contains("._M_current")
    || generated.contains("._M_t")
    || generated.contains("+ c_void")
    // ... 200+ more patterns
{
    self.output.truncate(method_output_start);  // DELETE THE METHOD
}
```

### Why it's wrong
1. **Hides real bugs**: Instead of fixing why `._M_current` doesn't work, we just delete the method
2. **Incomplete API**: Users get a `std_map_int__int` with 3 methods instead of 50+
3. **Silent failures**: No error message - the method just disappears
4. **False success**: "0 compilation errors" means nothing if we deleted all the broken code

### What to do instead
Fix the actual transpilation issue:
- If `._M_current` doesn't exist, find out why the field wasn't generated
- If types don't match, fix the type inference
- If variables are undeclared, fix the method body extraction

---

## 2. Stub Method Injection (FORBIDDEN)

### What it is
Adding fake method implementations that do nothing:

```rust
// WRONG - This is in ast_codegen.rs and must be removed
if rust_name.starts_with("std_map_") {
    self.writeln("pub fn size(&self) -> usize { 0 }");  // FAKE!
    self.writeln("pub fn op_index(&mut self, _key: i32) -> *mut i32 {");
    self.writeln("    std::ptr::null_mut()");  // FAKE!
    self.writeln("}");
}
```

### Why it's wrong
1. **Not transpilation**: We're writing Rust code, not transpiling C++ code
2. **Wrong behavior**: `size()` returning 0 for a non-empty map is a bug
3. **Defeats the purpose**: We want libc++ semantics, not fake stubs
4. **Masks problems**: Makes tests "pass" when they should fail

### What to do instead
Transpile the actual libc++ method:
- `std::map::size()` calls `__tree_.size()` - transpile that
- `std::map::operator[]` does tree lookup - transpile that

---

## 3. Semantic Type Mapping (FORBIDDEN)

### What it is
Mapping C++ types to Rust standard library equivalents:

```rust
// WRONG - Must never do this
"std::map" => "BTreeMap"
"std::vector" => "Vec"
"std::string" => "String"
```

### Why it's wrong
1. **Different semantics**: BTreeMap is not std::map (different iterator invalidation, no allocator)
2. **Different memory layout**: Can't do FFI with semantic mapping
3. **Incomplete**: Not all std::map methods exist on BTreeMap
4. **Wrong approach**: We transpile code, not map types

### What to do instead
Transpile the actual libc++ implementation as a Rust struct with the same fields and methods.

---

## 4. `todo!()` Method Bodies (FORBIDDEN for shipped code)

### What it is
```rust
pub fn contains(&mut self, __k: i32) -> bool {
    todo!("Template method body")  // WRONG
}
```

### Why it's wrong
1. **Not a solution**: The method exists but doesn't work
2. **Runtime panic**: Will crash if called
3. **Hides the real problem**: Why didn't LibTooling extract this body?

### What to do instead
- Debug why LibTooling didn't extract the method body
- Fix the body extraction in `fragile-ast-exporter`
- If truly impossible, document WHY and create a tracking issue

---

## 5. Skipping Types/Methods Without Tracking (FORBIDDEN)

### What it is
```rust
// Skip problematic types
if name.contains("__wrap_iter") { return; }  // WRONG without tracking
```

### Why it's wrong
Silently skipping types means we don't know what's missing.

### What to do instead
If something must be skipped temporarily:
1. Log a warning
2. Add to a "known issues" tracking list
3. Create a TODO item to fix it
4. Never consider the feature "complete" while skips exist

---

## Commit Checklist

Before every commit, verify:

- [ ] No new rollback patterns added to `ast_codegen.rs`
- [ ] No new stub method injections (methods that return hardcoded values)
- [ ] No semantic type mappings (std::X → Rust std::X)
- [ ] No `todo!()` bodies without tracking issue
- [ ] No silent skips without logging/tracking
- [ ] Existing rollback count has not increased (currently ~204 patterns - must decrease)

---

## Current Technical Debt

### Rollback patterns to remove (in ast_codegen.rs)
Location: Lines ~2921-3100+ and ~3457-3600+

These patterns exist because the underlying transpilation has bugs. Each pattern represents a bug to fix:

1. `._M_current` - Iterator field not being generated
2. `._M_t` - Map internal tree field access broken
3. `+ c_void` - Type resolution failing for template parameters
4. `._M_impl` - Allocator implementation field missing
5. ... (200+ more)

### Stub injections to remove (in ast_codegen.rs)
Location: Lines ~3300-3350

```rust
// These must be replaced with actual transpiled code:
"pub fn size(&self) -> usize { 0 }"
"pub fn op_index(&mut self, _key: i32) -> *mut i32 { std::ptr::null_mut() }"
"pub fn push_back(&mut self, _val: i32) { }"
"pub fn new_0() -> Self { unsafe { std::mem::zeroed() } }"
```

---

## The Right Way

The correct approach is always:

1. **Transpile the actual C++ code** - Every line of libc++ becomes a line of Rust
2. **Fix type resolution** - If types don't match, fix the type system
3. **Fix field generation** - If fields are missing, fix struct generation
4. **Fix method body extraction** - If bodies are wrong, fix LibTooling integration
5. **Test runtime behavior** - Compilation is not enough; the code must actually work

Remember: **The goal is not "0 compilation errors". The goal is "correct transpilation of libc++ to Rust".**
