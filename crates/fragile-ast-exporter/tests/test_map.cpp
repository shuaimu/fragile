// Test file for std::map template instantiation export

#include <map>
#include <string>

void test_map_basic() {
    std::map<int, int> m;
    m[1] = 10;
    m[2] = 20;

    int val = m[1];

    auto it = m.find(2);
    if (it != m.end()) {
        int found = it->second;
    }
}

void test_map_string() {
    std::map<std::string, int> m;
    m["hello"] = 1;
    m["world"] = 2;

    auto size = m.size();
    bool empty = m.empty();

    m.clear();
}

int main() {
    test_map_basic();
    test_map_string();
    return 0;
}
