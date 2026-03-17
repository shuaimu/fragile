#include "corpus_algo.hpp"

namespace corpus {

int classify_status(Status status) {
    switch (status) {
    case Status::Ok:
        return 1;
    case Status::Retry:
        return 2;
    default:
        return -1;
    }
}

int fold_packets(const Packet* packets, int len) {
    int total = 0;
    for (int i = 0; i < len; ++i) {
        total += packets[i].id;
    }
    return total;
}

int run_pipeline(int seed) {
#ifndef CORPUS_SCALE
#error CORPUS_SCALE must be defined
#endif
    Box<int> box(seed);
    auto reducer = [](int acc, int idx) { return acc + idx + CORPUS_SCALE; };

    Packet packets[3] = {
        Packet(1, "alpha"),
        Packet(2, "beta"),
        Packet(3, "gamma"),
    };

    return apply_n(box.get(), 3, reducer) + fold_packets(packets, 3);
}

}  // namespace corpus
