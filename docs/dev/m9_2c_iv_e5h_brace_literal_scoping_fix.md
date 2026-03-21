# M9.2.c.iv.e.5.h: Fix brace-literal scoping in function-static normalizer

## Date: 2026-03-21

## Problem

The `normalize_unprefixed_function_static_symbol_refs` normalizer pass uses brace depth tracking to scope function-static aliases per function body. The brace counter naively counted ALL `{` and `}` characters, including those inside:

- **Char literals**: `'{' | '[' => { ... }` counted 3 `{` instead of 1
- **String literals**: `"{} items"` counted 1 `{` and 1 `}`
- **`Some('{')` patterns**: counted the char literal brace

This caused functions containing char/string literal braces (e.g., `fragile_rapidjson_minify_json`) to never "close" in the brace tracker. All subsequent lines in the file were scoped to that function, causing:

1. The `__x -> __fsv___func___x_0` alias from `__seed()` at line ~30428 was placed under fn_key of the minify function (since the static appeared "inside" it from the tracker's perspective)
2. All other functions (like `trunc_(__x)`) were also assigned the same fn_key
3. The `__x` parameter in `trunc_` was rewritten to `unsafe { __fsv___func___x_0 }`, producing 186 E0425 errors

## Root Cause

Two bugs:

1. **Brace-in-literal counting**: The first pass of `normalize_unprefixed_function_static_symbol_refs` used `for ch in trimmed.chars()` to count `{`/`}`, without skipping char literals (`'{'`) or string literals (`"{}"`).

2. **function_static_var_mapping leakage** (secondary): `generate_method` and `generate_fn_template_instance` did not save/restore `function_static_var_mapping`, allowing function-static variable mappings to leak between methods and template instances during codegen.

## Fix

1. Replaced naive char-by-char brace counting with a byte-level scanner that:
   - Skips `'X'` and `'\X'` char literals (including `'{'` and `'}'`)
   - Skips `"..."` string literals (including escaped chars)
   - Only counts `{`/`}` outside literals

2. Added save/restore of `function_static_var_mapping` and `function_static_counter` in:
   - `generate_fn_template_instance` (template function codegen)
   - `generate_method` CXXMethodDecl path
   - `generate_method` ConstructorDecl path

## Impact

- **E0425 errors**: 194 -> 8 per file (186 `__fsv___func___x_0` eliminated, 100%)
- **Total errors**: 288 -> 100 per file (65% reduction)
- debugging.cpp: 288 -> 100
- misc.cpp: 289 -> 101

## Remaining E0425 (8 per file)

These are legitimate unresolved symbols, not alias leakage:
- `__c` (2): allocator parameter
- `__imp` (2): DLL import type
- `_Schrage`, `_Part`, `_Full` (3): template constants
- `__make_unsigned_type_parameter_0_0_` (1): unresolved template type

## Test Coverage

- 3 unit tests in `ast_codegen.rs`:
  - `test_normalize_function_static_symbol_refs_char_literal_braces_do_not_break_scoping`
  - `test_normalize_function_static_symbol_refs_string_literal_braces_do_not_break_scoping`
  - `test_normalize_function_static_symbol_refs_match_arm_some_brace`
- 4 M9 closure tests in `m9_rpc_closure_tests.rs`:
  - `m9_2c_iv_e5h_task_documented_in_todo`
  - `m9_2c_iv_e5h_brace_tracking_skips_char_literals`
  - `m9_2c_iv_e5h_inventory_document_exists`
  - `m9_2c_iv_e5h_function_static_mapping_isolated_in_generate_method`
