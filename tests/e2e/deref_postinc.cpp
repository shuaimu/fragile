// Test for dereferencing a post-incremented pointer
// This pattern *ptr++ should:
// 1. Return the value at the original pointer location
// 2. Then increment the pointer

char get_and_advance(const char** ptr_ptr) {
    const char* ptr = *ptr_ptr;
    char result = *ptr++;  // Deref then post-increment
    *ptr_ptr = ptr;
    return result;
}

int main() {
    const char* hello = "hello";
    const char** pp = &hello;

    // Get first char
    char c1 = get_and_advance(pp);
    if (c1 != 'h') return 1;

    // Get second char
    char c2 = get_and_advance(pp);
    if (c2 != 'e') return 2;

    // Get third char
    char c3 = get_and_advance(pp);
    if (c3 != 'l') return 3;

    return 0;  // Success
}
