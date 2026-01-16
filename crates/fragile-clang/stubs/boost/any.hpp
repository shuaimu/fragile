// Stub header for boost/any.hpp
#pragma once

#include <typeinfo>
#include <stdexcept>

namespace boost {

class bad_any_cast : public std::bad_cast {
public:
    const char* what() const noexcept override {
        return "boost::bad_any_cast";
    }
};

class any {
public:
    any() noexcept : content_(nullptr) {}

    template<typename T>
    any(const T& value) : content_(new holder<T>(value)) {}

    any(const any& other) : content_(other.content_ ? other.content_->clone() : nullptr) {}

    any(any&& other) noexcept : content_(other.content_) {
        other.content_ = nullptr;
    }

    ~any() { delete content_; }

    any& operator=(const any& other) {
        any(other).swap(*this);
        return *this;
    }

    any& operator=(any&& other) noexcept {
        other.swap(*this);
        any().swap(other);
        return *this;
    }

    template<typename T>
    any& operator=(const T& value) {
        any(value).swap(*this);
        return *this;
    }

    bool empty() const noexcept {
        return !content_;
    }

    void clear() noexcept {
        any().swap(*this);
    }

    void swap(any& other) noexcept {
        placeholder* tmp = content_;
        content_ = other.content_;
        other.content_ = tmp;
    }

    const std::type_info& type() const noexcept {
        return content_ ? content_->type() : typeid(void);
    }

private:
    class placeholder {
    public:
        virtual ~placeholder() {}
        virtual const std::type_info& type() const noexcept = 0;
        virtual placeholder* clone() const = 0;
    };

    template<typename T>
    class holder : public placeholder {
    public:
        holder(const T& value) : held_(value) {}
        const std::type_info& type() const noexcept override { return typeid(T); }
        placeholder* clone() const override { return new holder(held_); }
        T held_;
    };

    template<typename T>
    friend T* any_cast(any*) noexcept;

    template<typename T>
    friend const T* any_cast(const any*) noexcept;

    placeholder* content_;
};

template<typename T>
T* any_cast(any* operand) noexcept {
    if (operand && operand->type() == typeid(T)) {
        return &static_cast<any::holder<T>*>(operand->content_)->held_;
    }
    return nullptr;
}

template<typename T>
const T* any_cast(const any* operand) noexcept {
    return any_cast<T>(const_cast<any*>(operand));
}

template<typename T>
T any_cast(any& operand) {
    T* result = any_cast<T>(&operand);
    if (!result) throw bad_any_cast();
    return *result;
}

template<typename T>
T any_cast(const any& operand) {
    const T* result = any_cast<T>(&operand);
    if (!result) throw bad_any_cast();
    return *result;
}

template<typename T>
T any_cast(any&& operand) {
    T* result = any_cast<T>(&operand);
    if (!result) throw bad_any_cast();
    return static_cast<T&&>(*result);
}

} // namespace boost
