// Minimal numa.h stub for fragile parsing
// NUMA (Non-Uniform Memory Access) library stubs

#ifndef _NUMA_H
#define _NUMA_H

#include "cstddef"
#include "cstdint"

// NUMA node type
typedef int nodemask_t;

// NUMA configuration queries
int numa_available(void);
int numa_max_node(void);
int numa_num_configured_nodes(void);
int numa_num_configured_cpus(void);
int numa_preferred(void);
void numa_set_localalloc(void);
void numa_set_preferred(int node);

// NUMA node set operations
struct bitmask;
struct bitmask* numa_bitmask_alloc(unsigned int n);
struct bitmask* numa_allocate_cpumask(void);
struct bitmask* numa_allocate_nodemask(void);
void numa_bitmask_free(struct bitmask* bmp);
void numa_free_nodemask(struct bitmask* bmp);
struct bitmask* numa_bitmask_setbit(struct bitmask* bmp, unsigned int n);
struct bitmask* numa_bitmask_clearbit(struct bitmask* bmp, unsigned int n);
int numa_bitmask_isbitset(const struct bitmask* bmp, unsigned int n);
void numa_bitmask_setall(struct bitmask* bmp);
void numa_bitmask_clearall(struct bitmask* bmp);
extern struct bitmask* numa_all_nodes_ptr;
extern struct bitmask* numa_no_nodes_ptr;

// Memory allocation
void* numa_alloc(size_t size);
void* numa_alloc_local(size_t size);
void* numa_alloc_onnode(size_t size, int node);
void* numa_alloc_interleaved(size_t size);
void* numa_alloc_interleaved_subset(size_t size, struct bitmask* nodemask);
void numa_free(void* start, size_t size);

// Memory binding
int numa_run_on_node(int node);
int numa_run_on_node_mask(struct bitmask* nodemask);
int numa_run_on_node_mask_all(struct bitmask* nodemask);
int numa_get_run_node_mask(struct bitmask* nodemask);
void numa_bind(struct bitmask* nodemask);
void numa_set_bind_policy(int strict);
void numa_set_strict(int strict);
void numa_set_membind(struct bitmask* nodemask);
void numa_set_interleave_mask(struct bitmask* nodemask);
struct bitmask* numa_get_membind(void);
struct bitmask* numa_get_interleave_mask(void);

// Memory policy
void numa_interleave_memory(void* mem, size_t size, struct bitmask* nodemask);
void numa_tonode_memory(void* mem, size_t size, int node);
void numa_tonodemask_memory(void* mem, size_t size, struct bitmask* nodemask);
void numa_setlocal_memory(void* mem, size_t size);
void numa_police_memory(void* mem, size_t size);
int numa_move_pages(int pid, unsigned long count, void** pages,
                    const int* nodes, int* status, int flags);
int numa_migrate_pages(int pid, struct bitmask* fromnodes,
                       struct bitmask* tonodes);

// Distance
int numa_distance(int node1, int node2);

// CPU/node affinity
int numa_node_of_cpu(int cpu);
long long numa_node_size(int node, long long* freep);
long long numa_node_size64(int node, long long* freep);

// Parse functions
int numa_parse_nodestring(const char* string);
int numa_parse_cpustring(const char* string);
struct bitmask* numa_parse_nodestring_all(const char* string);
struct bitmask* numa_parse_cpustring_all(const char* string);

// Error handling
void numa_error(char* where);
void numa_warn(int number, char* where, ...);

// NUMA API version
int numa_pagesize(void);

#endif // _NUMA_H
