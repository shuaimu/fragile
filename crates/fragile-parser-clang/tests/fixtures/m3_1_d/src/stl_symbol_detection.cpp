namespace std {
template <typename T>
struct vector {};

template <typename K, typename V>
struct map {};

template <typename K, typename V>
struct unordered_map {};

struct string {};

template <typename T>
struct optional {};

template <typename T, typename U>
struct variant {};

template <typename T, typename U>
struct tuple {};

template <typename T>
struct shared_ptr {};

template <typename T>
struct unique_ptr {};
} // namespace std

namespace direct {
using DirectVec = std::vector<int>;
using DirectMap = std::map<int, int>;
using DirectUnorderedMap = std::unordered_map<int, int>;
using DirectString = std::string;
using DirectOpt = std::optional<int>;
using DirectVariant = std::variant<int, double>;
using DirectTuple = std::tuple<int, double>;
using DirectShared = std::shared_ptr<int>;
using DirectUnique = std::unique_ptr<int>;
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
using std::unordered_map;
using ImportedMap = map<int, int>;
using ImportedUnorderedMap = unordered_map<int, int>;
using ImportedVec = Final;
using ImportedString = DirectString;
using ImportedOpt = DirectOpt;
using ImportedVariant = DirectVariant;
using ImportedTuple = DirectTuple;
using ImportedShared = DirectShared;
using ImportedUnique = DirectUnique;
} // namespace using_chain

namespace transit {
using namespace using_chain;
using TransitVec = ImportedVec;
using TransitMap = ImportedMap;
using TransitUnorderedMap = ImportedUnorderedMap;
using TransitString = ImportedString;
using TransitOpt = ImportedOpt;
using TransitVariant = ImportedVariant;
using TransitTuple = ImportedTuple;
using TransitShared = ImportedShared;
using TransitUnique = ImportedUnique;
} // namespace transit

void consume_symbols() {
    std::vector<int> direct_vec;
    std::vector<int> direct_vec_init = std::vector<int>();
    std::map<int, int> direct_map;
    std::unordered_map<int, int> direct_unordered_map;
    std::string direct_string;
    std::optional<int> direct_opt;
    std::variant<int, double> direct_variant;
    std::tuple<int, double> direct_tuple;
    std::shared_ptr<int> direct_shared;
    std::unique_ptr<int> direct_unique;
    using_chain::ImportedVec imported_vec;
    using_chain::ImportedVec imported_vec_init = using_chain::ImportedVec();
    using_chain::ImportedMap imported_map;
    using_chain::ImportedUnorderedMap imported_unordered_map;
    using_chain::ImportedString imported_string;
    using_chain::ImportedOpt imported_opt;
    using_chain::ImportedVariant imported_variant;
    using_chain::ImportedTuple imported_tuple;
    using_chain::ImportedShared imported_shared;
    using_chain::ImportedUnique imported_unique;
    transit::TransitVec transit_vec;
    transit::TransitMap transit_map;
    transit::TransitUnorderedMap transit_unordered_map;
    transit::TransitString transit_string;
    transit::TransitOpt transit_opt;
    transit::TransitVariant transit_variant;
    transit::TransitTuple transit_tuple;
    transit::TransitShared transit_shared;
    transit::TransitUnique transit_unique;
}
