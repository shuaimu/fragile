# Plan: Task 23.11.1 - Test Single-File Projects

## Overview

Test the transpiler against a self-contained single-file C++ program that exercises multiple features we support.

## Approach

Since iostream is blocked and nlohmann/json is complex, we'll create a comprehensive test that:
1. Uses classes with inheritance and virtual methods
2. Uses templates
3. Uses our STL stubs (vector, string, smart pointers)
4. Uses operator overloading
5. Returns exit codes to verify correctness

## Test Program Design

A simple expression evaluator that:
- Has a base class `Expr` with virtual `eval()` method
- Has derived classes: `Number`, `Add`, `Mul`
- Uses smart pointers for memory management
- Returns 0 on success, non-zero on failure

```cpp
// Simple expression evaluator
// Tests: classes, inheritance, virtual methods, templates, basic operations

class Expr {
public:
    virtual ~Expr() {}
    virtual int eval() const = 0;
};

class Number : public Expr {
    int value;
public:
    Number(int v) : value(v) {}
    int eval() const override { return value; }
};

class BinaryExpr : public Expr {
protected:
    Expr* left;
    Expr* right;
public:
    BinaryExpr(Expr* l, Expr* r) : left(l), right(r) {}
    ~BinaryExpr() { delete left; delete right; }
};

class Add : public BinaryExpr {
public:
    Add(Expr* l, Expr* r) : BinaryExpr(l, r) {}
    int eval() const override { return left->eval() + right->eval(); }
};

class Mul : public BinaryExpr {
public:
    Mul(Expr* l, Expr* r) : BinaryExpr(l, r) {}
    int eval() const override { return left->eval() * right->eval(); }
};

int main() {
    // (2 + 3) * 4 = 20
    Expr* expr = new Mul(
        new Add(new Number(2), new Number(3)),
        new Number(4)
    );

    int result = expr->eval();
    delete expr;

    if (result == 20) {
        return 0;  // Success
    }
    return 1;  // Failure
}
```

## Test Cases

1. Expression evaluator compiles and runs
2. Virtual dispatch works (eval() on polymorphic types)
3. Inheritance chain works (BinaryExpr inherits Expr, Add/Mul inherit BinaryExpr)
4. Memory management works (new/delete)
5. Result is correct (20)

## Implementation Steps

1. Add E2E test `test_e2e_expression_evaluator` (~100 LOC)
2. Verify it compiles and runs with exit code 0

## Estimated LOC

- Test code: ~60 LOC
- Total: ~60 LOC
