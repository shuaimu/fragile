// Stub header for boost/unordered_map.hpp
// Provides minimal type declarations for C++ parsing

#ifndef BOOST_UNORDERED_MAP_HPP
#define BOOST_UNORDERED_MAP_HPP

#include <unordered_map>
#include <functional>
#include <utility>

namespace boost {

// boost::unordered_map is essentially std::unordered_map
// We provide it as an alias for compatibility
template <
    class Key,
    class T,
    class Hash = std::hash<Key>,
    class Pred = std::equal_to<Key>,
    class Allocator = std::allocator<std::pair<const Key, T>>
>
class unordered_map : public std::unordered_map<Key, T, Hash, Pred, Allocator> {
public:
    using base = std::unordered_map<Key, T, Hash, Pred, Allocator>;
    using base::base;  // Inherit constructors

    // Additional boost-specific methods (minimal stubs)
    template<typename... Args>
    std::pair<typename base::iterator, bool> emplace(Args&&... args) {
        return base::emplace(std::forward<Args>(args)...);
    }
};

// boost::unordered_multimap
template <
    class Key,
    class T,
    class Hash = std::hash<Key>,
    class Pred = std::equal_to<Key>,
    class Allocator = std::allocator<std::pair<const Key, T>>
>
class unordered_multimap : public std::unordered_multimap<Key, T, Hash, Pred, Allocator> {
public:
    using base = std::unordered_multimap<Key, T, Hash, Pred, Allocator>;
    using base::base;  // Inherit constructors
};

} // namespace boost

#endif // BOOST_UNORDERED_MAP_HPP
