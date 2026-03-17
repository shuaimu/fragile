#pragma once

namespace corpus {

enum class Status : int {
    Ok = 0,
    Retry = 1,
    Fail = 2,
};

template <typename T>
struct Box {
    T value;

    Box() : value() {}
    explicit Box(T v) : value(v) {}

    T get() const {
        return value;
    }
};

struct Packet {
    int id;
    const char* tag;

    Packet(int packet_id, const char* packet_tag)
        : id(packet_id), tag(packet_tag) {}
};

using PacketBox = Box<Packet>;

int classify_status(Status status);

}  // namespace corpus
