// Minimal numa.h stub for fragile parsing (libnuma)
#ifndef _FRAGILE_NUMA_H_
#define _FRAGILE_NUMA_H_

#include "cstdint"
#include "cstddef"

// NUMA bitmask structure
struct bitmask {
    unsigned long size;
    unsigned long* maskp;
};

extern "C" {

// Initialization/availability
int numa_available(void);

// Node information
int numa_max_node(void);
int numa_num_configured_nodes(void);
int numa_num_possible_nodes(void);
int numa_num_configured_cpus(void);

// Memory allocation on specific node
void* numa_alloc(size_t size);
void* numa_alloc_onnode(size_t size, int node);
void* numa_alloc_local(size_t size);
void* numa_alloc_interleaved(size_t size);
void numa_free(void* start, size_t size);

// Memory binding
int numa_run_on_node(int node);
int numa_run_on_node_mask(struct bitmask* mask);
void numa_set_bind_policy(int strict);
void numa_set_strict(int strict);
int numa_set_preferred(int node);
void numa_set_localalloc(void);
void numa_set_membind(struct bitmask* mask);
void numa_set_interleave_mask(struct bitmask* mask);

// Get binding
struct bitmask* numa_get_run_node_mask(void);
struct bitmask* numa_get_membind(void);
struct bitmask* numa_get_interleave_mask(void);

// Node of address/cpu
int numa_node_of_cpu(int cpu);
int numa_preferred(void);
long numa_node_size64(int node, long* freep);

// Bitmask operations
struct bitmask* numa_allocate_cpumask(void);
struct bitmask* numa_allocate_nodemask(void);
struct bitmask* numa_bitmask_alloc(unsigned int n);
void numa_bitmask_free(struct bitmask* bmp);
struct bitmask* numa_bitmask_setbit(struct bitmask* bmp, unsigned int n);
struct bitmask* numa_bitmask_clearbit(struct bitmask* bmp, unsigned int n);
int numa_bitmask_isbitset(const struct bitmask* bmp, unsigned int n);
struct bitmask* numa_bitmask_setall(struct bitmask* bmp);
struct bitmask* numa_bitmask_clearall(struct bitmask* bmp);
int numa_bitmask_equal(const struct bitmask* bmp1, const struct bitmask* bmp2);

// Parsing
struct bitmask* numa_parse_nodestring(const char* string);
struct bitmask* numa_parse_cpustring(const char* string);

// Move pages
int numa_move_pages(int pid, unsigned long count, void** pages,
                    const int* nodes, int* status, int flags);
int numa_tonode_memory(void* start, size_t size, int node);
int numa_migrate_pages(int pid, struct bitmask* from, struct bitmask* to);

// Distance
int numa_distance(int node1, int node2);

// Error handling
void numa_error(const char* where);
void numa_warn(int number, const char* format, ...);

}

#endif // _FRAGILE_NUMA_H_
