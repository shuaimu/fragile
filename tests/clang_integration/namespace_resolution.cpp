// Test file for namespace name resolution
// Tests A.1.4: Name resolution

// Test 1: Same namespace lookup
namespace foo {
    int helper() {
        return 42;
    }

    int test_same_namespace() {
        return helper();  // Should resolve to foo::helper
    }
}

// Test 2: Using namespace directive
namespace bar {
    int bar_helper() {
        return 100;
    }
}

using namespace bar;

int test_using_namespace() {
    return bar_helper();  // Should resolve to bar::bar_helper
}

// Test 3: Using declaration
namespace baz {
    int baz_func() {
        return 200;
    }
}

using baz::baz_func;

int test_using_declaration() {
    return baz_func();  // Should resolve to baz::baz_func
}

// Test 4: Nested namespace lookup
namespace outer {
    namespace inner {
        int nested_func() {
            return 300;
        }
    }

    int test_nested() {
        return inner::nested_func();  // Should resolve to outer::inner::nested_func
    }
}

// Test 5: Global function from namespace
int global_func() {
    return 500;
}

namespace ns {
    int test_global() {
        return global_func();  // Should resolve to global_func (global scope)
    }
}
