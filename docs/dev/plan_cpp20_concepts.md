# Plan: C++20 Concepts Support

## Task
Implement C++20 Concepts support including concept definitions, requires clauses, and requires expressions.

## Background
C++20 Concepts provide a way to constrain templates with predicates, improving error messages and allowing more expressive template constraints. Key features:

```cpp
// Concept definition
template<typename T>
concept Integral = std::is_integral_v<T>;

// Requires clause on function template
template<typename T>
    requires Integral<T>
T twice(T x) { return x * 2; }

// Abbreviated template syntax
template<Integral T>
T twice(T x) { return x * 2; }

// Requires expression
template<typename T>
concept Addable = requires(T a, T b) {
    a + b;                           // Simple requirement
    { a + b } -> std::same_as<T>;    // Compound requirement
    typename T::value_type;          // Type requirement
};
```

## Complexity Assessment
This is a **HIGH** complexity feature requiring 5-6 iterations (~500-800 lines total).

## Scope
Focus on concepts features used in Mako:
- Basic concept definitions
- Requires clauses on templates
- Simple requires expressions
- Compound requirements (return type checking)

**Deferred:**
- Nested requirements
- Constraint subsumption for overload resolution
- Standard concepts library (std::integral, etc.)

## Implementation Plan

### B.4.1 AST Representation (~100 lines)

Add to `ClangNodeKind` in `ast.rs`:

```rust
/// Concept definition (e.g., template<typename T> concept Integral = ...)
ConceptDecl {
    name: String,
    /// Template parameters for the concept
    template_params: Vec<String>,
    /// The constraint expression (stored as text for now)
    constraint_expr: String,
}

/// Requires clause attached to a template (e.g., requires Integral<T>)
RequiresClause {
    /// The constraint expression text
    constraint_expr: String,
}

/// Requires expression (e.g., requires { expr; })
RequiresExpr {
    /// Parameter list for the requires expression
    params: Vec<(String, CppType)>,
    /// Requirements inside the requires expression
    requirements: Vec<Requirement>,
}
```

Add new types:

```rust
/// A single requirement inside a requires expression
#[derive(Debug, Clone)]
pub enum Requirement {
    /// Simple requirement: expression must be valid (e.g., `a + b;`)
    Simple { expr: String },
    /// Type requirement: type must exist (e.g., `typename T::value_type;`)
    Type { type_name: String },
    /// Compound requirement: expr with optional noexcept and return type
    /// (e.g., `{ a + b } -> std::same_as<T>;`)
    Compound {
        expr: String,
        is_noexcept: bool,
        return_constraint: Option<String>,
    },
    /// Nested requirement: requires clause inside requires
    Nested { constraint: String },
}
```

Update `FunctionTemplateDecl` and `ClassTemplateDecl` to include optional requires clause:

```rust
FunctionTemplateDecl {
    // ... existing fields ...
    /// Optional requires clause
    requires_clause: Option<String>,
}

ClassTemplateDecl {
    // ... existing fields ...
    /// Optional requires clause
    requires_clause: Option<String>,
}
```

### B.4.2 Parser Support (~200 lines)

Add to `parse.rs`:

1. **Handle ConceptDecl cursor** (libclang `CXCursor_ConceptDecl` = 604):
   ```rust
   604 => {
       let name = cursor_spelling(cursor);
       let template_params = self.get_template_type_params(cursor);
       let constraint_expr = self.get_concept_constraint(cursor);
       ClangNodeKind::ConceptDecl {
           name,
           template_params,
           constraint_expr,
       }
   }
   ```

2. **Extract requires clause from function/class templates**:
   - Check for `CXCursor_RequiresExpr` child (cursor kind 279)
   - Extract the text of the constraint

3. **Handle RequiresExpr cursor**:
   ```rust
   // CXCursor_RequiresExpr = 279
   279 => {
       let params = self.get_requires_params(cursor);
       let requirements = self.get_requirements(cursor);
       ClangNodeKind::RequiresExpr { params, requirements }
   }
   ```

4. **Helper functions**:
   - `get_concept_constraint()` - extract constraint expression text
   - `get_requires_params()` - extract parameters from requires expression
   - `get_requirements()` - extract requirement list

### B.4.3 Concept Definitions (~100 lines)

Update `lib.rs` to add concept tracking in `CppModule`:

```rust
pub struct CppModule {
    // ... existing fields ...
    /// Concept definitions in this module
    pub concepts: Vec<CppConceptDecl>,
}

/// A C++ concept definition
#[derive(Debug, Clone)]
pub struct CppConceptDecl {
    pub name: String,
    pub template_params: Vec<String>,
    pub constraint_expr: String,
    /// Parsed constraint for evaluation (if supported)
    pub constraint: Option<ConceptConstraint>,
}

/// Parsed constraint for evaluation
#[derive(Debug, Clone)]
pub enum ConceptConstraint {
    /// Type trait check (e.g., std::is_integral_v<T>)
    TypeTrait { trait_kind: TypeTraitKind, type_arg: CppType },
    /// Conjunction of constraints
    And(Box<ConceptConstraint>, Box<ConceptConstraint>),
    /// Disjunction of constraints
    Or(Box<ConceptConstraint>, Box<ConceptConstraint>),
    /// Negation
    Not(Box<ConceptConstraint>),
    /// Another concept
    ConceptRef { name: String, args: Vec<CppType> },
    /// Unparsed constraint (fallback)
    Unparsed(String),
}
```

### B.4.4 Requires Clauses on Templates (~100 lines)

Update `convert.rs` to:
1. Pass through requires clause information when converting templates
2. Store requires clause in `CppFunctionTemplate` and `CppClassTemplate`
3. Evaluate requires clause during template instantiation (basic support)

### B.4.5 Requires Expressions (~150 lines)

Update handling in `convert.rs`:
1. Convert `RequiresExpr` AST to a boolean constraint check
2. Support simple requirements (expression validity)
3. Support type requirements
4. Support compound requirements with return type constraints

### B.4.6 Standard Concepts (Deferred)

This is deferred to Phase C (Standard Library) as it requires:
- std::same_as
- std::derived_from
- std::convertible_to
- std::integral, std::floating_point, etc.

For now, map standard concepts to type traits where possible:
- `std::integral<T>` → `TypeTraitKind::IsIntegral`
- `std::same_as<T, U>` → `TypeTraitKind::IsSame`

## Files to Modify

1. `crates/fragile-clang/src/ast.rs` - Add concept AST nodes (~80 lines)
2. `crates/fragile-clang/src/parse.rs` - Parse concept cursors (~150 lines)
3. `crates/fragile-clang/src/lib.rs` - Add CppConceptDecl, update CppModule (~50 lines)
4. `crates/fragile-clang/src/convert.rs` - Convert concept AST to module (~100 lines)
5. `crates/fragile-clang/tests/integration_test.rs` - Add concept tests (~100 lines)

## Test Cases

### B.4.1 AST Tests
```cpp
// Test concept definition
template<typename T>
concept Integral = __is_integral(T);
```

### B.4.2 Parser Tests
```cpp
// Test requires clause parsing
template<typename T>
    requires __is_integral(T)
T square(T x) { return x * x; }
```

### B.4.3-B.4.5 Integration Tests
```cpp
// Test concept with function
template<typename T>
concept Numeric = __is_arithmetic(T);

template<Numeric T>
T add(T a, T b) { return a + b; }

// Test requires expression
template<typename T>
concept Addable = requires(T a, T b) {
    a + b;
};
```

## Estimated Size
~500-600 lines of code total across all files.

## Dependencies
- [x] Function templates (B.1)
- [x] Class templates (B.2)
- [x] Type traits (B.3)

## Risk Assessment

**Low Risk:**
- AST representation - straightforward extension
- Parsing with libclang - well-supported

**Medium Risk:**
- Constraint evaluation - complex for full support
- Integration with template instantiation

**Mitigation:**
- Start with unparsed string storage for constraints
- Evaluate type trait-based constraints only
- Defer complex constraint satisfaction to later iteration

## Implementation Order

1. **B.4.1** - Add AST nodes (ConceptDecl, RequiresExpr, Requirement)
2. **B.4.2** - Add parser support for concept cursors
3. **B.4.3** - Add CppConceptDecl to CppModule, basic conversion
4. **B.4.4** - Update FunctionTemplateDecl/ClassTemplateDecl for requires
5. **B.4.5** - Handle RequiresExpr in conversion
6. **B.4.6** - (Deferred) Standard concepts mapping
