#pragma once

#include "corpus_common.hpp"

namespace corpus {

template <typename Fn>
int apply_n(int seed, int count, Fn fn) {
    int out = seed;
    for (int i = 0; i < count; ++i) {
        out = fn(out, i);
    }
    return out;
}

int fold_packets(const Packet* packets, int len);

}  // namespace corpus
