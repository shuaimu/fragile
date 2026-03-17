typedef struct MetricSample {
    int weight;
    int value;
} MetricSample;

static int clamp_non_negative(int value) {
    if (value < 0) {
        return 0;
    }
    return value;
}

int accumulate_weighted(const MetricSample* samples, int len) {
    int total = 0;
    for (int i = 0; i < len; ++i) {
        total += samples[i].weight * clamp_non_negative(samples[i].value);
    }
    return total;
}

int stable_partition_score(int seed) {
#ifndef CORPUS_C_SHIFT
#error CORPUS_C_SHIFT must be defined
#endif
    MetricSample local[3] = {
        {1, seed},
        {2, seed + CORPUS_C_SHIFT},
        {3, seed - 1},
    };
    return accumulate_weighted(local, 3);
}
