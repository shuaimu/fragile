# Plan: Task 23.11.3 - Test Medium Projects (5K-50K LOC)

## Goal

Test the transpiler against actual open-source C++ projects in the 5K-50K LOC range.
Accept partial success - some tests may fail, but core functionality should work.

## Current Blockers (Inherited from 23.11.2)

1. **iostream** - BLOCKED (static initialization issues)
2. **Threading** - BLOCKED (libc++ thread support incomplete)
3. **Complex STL** - Partial (vector works, map/set may need more work)

## Strategy

Focus on projects that:
1. Are primarily algorithmic/computational
2. Minimize iostream usage (or can be configured without)
3. Have good test coverage we can run
4. Use C++11/14/17 features (not heavy C++20)

## Candidate Projects

### Option 1: Header-Only Libraries

**nlohmann/json** (~14K LOC)
- Pros: Header-only, widely used, good tests
- Cons: Heavy template usage, may stress transpiler

**catch2** (~15K LOC)
- Pros: Unit testing framework, header-only
- Cons: Heavy macro usage, stream operators

### Option 2: Algorithm Libraries

**cpp-sort** (~8K LOC)
- Pros: Sorting algorithms, well-tested
- Cons: Template-heavy

**ETL (Embedded Template Library)** (~30K LOC)
- Pros: Designed for embedded, minimal dependencies
- Cons: Still uses some STL patterns

### Option 3: Self-Contained Utilities

**argparse** (~3K LOC)
- Pros: Argument parsing, minimal deps
- Cons: Uses streams for help output

**fmt** (format library, ~10K LOC core)
- Pros: String formatting, well-tested
- Cons: Complex template machinery

### Option 4: Data Structure Libraries

**robin-hood-hashing** (~4K LOC)
- Pros: Hash table implementation, focused
- Cons: Modern C++ features

## Recommended Approach

Given the blockers, start with projects that:
1. Compile as single translation unit (easier to test)
2. Have self-contained test files
3. Don't require iostream for core functionality

### Phase 1: Prepare Test Infrastructure
1. Create test harness for multi-file projects
2. Add compile_commands.json support for include paths
3. Set up test result tracking

### Phase 2: Test Small Header-Only Libraries
1. Start with robin-hood-hashing (smallest, focused)
2. Try argparse (if we stub iostream)
3. Attempt fmt core functions

### Phase 3: Progress to Larger Projects
1. nlohmann/json (if templates work well)
2. ETL subset (embedded focus may be simpler)

## LOC Estimate

- Test infrastructure: ~200 LOC
- Project-specific test harnesses: ~100 LOC each
- Total: ~500 LOC for initial setup

## Next Steps

1. Clone and analyze robin-hood-hashing
2. Identify minimal test case to transpile
3. Fix any transpiler issues found
4. Document results and iterate

## Success Metrics

- At least one 5K+ LOC project compiles
- Some tests from that project pass
- Document any new limitations discovered
