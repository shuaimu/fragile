#include "corpus_common.hpp"

namespace corpus {

typedef int (*HandlerFn)(int);

int dispatch_status(Status status, HandlerFn on_ok, HandlerFn on_retry, HandlerFn on_fail) {
    switch (status) {
    case Status::Ok:
        return on_ok(1);
    case Status::Retry:
        return on_retry(2);
    default:
        return on_fail(3);
    }
}

int wrap_dispatch(HandlerFn handler) {
    PacketBox boxed(Packet(9, "delta"));
    return dispatch_status(Status::Retry, handler, handler, handler) + boxed.get().id;
}

}  // namespace corpus
