// Test 02: Extern "C" block
// Needed for libc interop

extern "C" {
    int puts(const char* s);
}

int main() {
    return 0;
}
