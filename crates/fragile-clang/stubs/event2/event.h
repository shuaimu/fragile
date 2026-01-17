// Stub header for event2/event.h (libevent)
// Provides minimal type declarations for C++ parsing

#ifndef EVENT2_EVENT_H_
#define EVENT2_EVENT_H_

#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// Forward declarations
struct event_base;
struct event;
struct timeval;

// evutil_socket_t is a typedef, not a struct
typedef int evutil_socket_t;

// Event flags
#define EV_TIMEOUT      0x01
#define EV_READ         0x02
#define EV_WRITE        0x04
#define EV_SIGNAL       0x08
#define EV_PERSIST      0x10
#define EV_ET           0x20
#define EV_FINALIZE     0x40
#define EV_CLOSED       0x80

// Event loop flags
#define EVLOOP_ONCE             0x01
#define EVLOOP_NONBLOCK         0x02
#define EVLOOP_NO_EXIT_ON_EMPTY 0x04

// Event base loop return values
#define EVLOOP_RUNNING 0
#define EVLOOP_DONE    1
#define EVLOOP_EXIT    2

// Event callback type
typedef void (*event_callback_fn)(evutil_socket_t, short, void*);

// Event base functions
struct event_base* event_base_new(void);
void event_base_free(struct event_base* base);
int event_base_dispatch(struct event_base* base);
int event_base_loop(struct event_base* base, int flags);
int event_base_loopbreak(struct event_base* base);
int event_base_loopexit(struct event_base* base, const struct timeval* tv);
int event_base_got_exit(struct event_base* base);
int event_base_got_break(struct event_base* base);

// Event functions
struct event* event_new(struct event_base* base, evutil_socket_t fd, short events,
                        event_callback_fn callback, void* arg);
void event_free(struct event* ev);
int event_add(struct event* ev, const struct timeval* timeout);
int event_del(struct event* ev);
int event_pending(const struct event* ev, short events, struct timeval* tv);
int event_priority_set(struct event* ev, int priority);
void event_active(struct event* ev, int res, short ncalls);
int event_assign(struct event* ev, struct event_base* base, evutil_socket_t fd,
                 short events, event_callback_fn callback, void* arg);
struct event_base* event_get_base(const struct event* ev);
evutil_socket_t event_get_fd(const struct event* ev);
short event_get_events(const struct event* ev);
event_callback_fn event_get_callback(const struct event* ev);
void* event_get_callback_arg(const struct event* ev);

// Timer functions (compatibility macros)
#define evtimer_new(base, callback, arg) \
    event_new((base), -1, 0, (callback), (arg))
#define evtimer_add(ev, tv) event_add((ev), (tv))
#define evtimer_del(ev) event_del(ev)
#define evtimer_pending(ev, tv) event_pending((ev), EV_TIMEOUT, (tv))

// Signal functions (compatibility macros)
#define evsignal_new(base, signum, callback, arg) \
    event_new((base), (signum), EV_SIGNAL|EV_PERSIST, (callback), (arg))
#define evsignal_add(ev, tv) event_add((ev), (tv))
#define evsignal_del(ev) event_del(ev)
#define evsignal_pending(ev, tv) event_pending((ev), EV_SIGNAL, (tv))

#ifdef __cplusplus
}
#endif

#endif // EVENT2_EVENT_H_
