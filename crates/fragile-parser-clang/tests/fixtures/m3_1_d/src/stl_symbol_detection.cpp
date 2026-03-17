namespace std {
template <typename T>
struct vector {};

template <typename K, typename V>
struct map {};

template <typename T>
struct optional {};
} // namespace std

namespace direct {
using DirectVec = std::vector<int>;
using DirectMap = std::map<int, int>;
using DirectOpt = std::optional<int>;
} // namespace direct

namespace typedef_chain {
using Seed = std::vector<int>;
typedef Seed Mid;
using Final = Mid;
} // namespace typedef_chain

namespace using_chain {
using typedef_chain::Final;
using namespace direct;
using std::map;
using ImportedMap = map<int, int>;
using ImportedVec = Final;
using ImportedOpt = DirectOpt;
} // namespace using_chain

namespace transit {
using namespace using_chain;
using TransitVec = ImportedVec;
} // namespace transit
