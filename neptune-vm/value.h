#pragma once
#include "util.h"
#include <cstdint>
#include <ostream>

#if (defined(__x86_64__) || defined(_M_X64) || defined(__aarch64__) ||         \
     defined(_M_ARM64))
#define NANBOX
#endif

namespace neptune_vm {
class Object;

class Value {
#ifdef NANBOX
  uint64_t inner;
  static constexpr uint64_t VALUE_NULL = 1;
  static constexpr uint64_t VALUE_TRUE = 2;
  static constexpr uint64_t VALUE_FALSE = 3;
  ALWAYS_INLINE explicit Value(uint64_t u) { inner = u; }

public:
  ALWAYS_INLINE explicit Value() { inner = 0; }
  ALWAYS_INLINE static Value new_true() { return Value(VALUE_TRUE); }

  ALWAYS_INLINE static Value new_false() { return Value(VALUE_FALSE); }

  ALWAYS_INLINE static Value null() { return Value(VALUE_NULL); }

  ALWAYS_INLINE static Value empty() { return Value(static_cast<uint64_t>(0)); }

#else
  enum class Tag : uint8_t {
    Empty,
    Int,
    Float,
    Object,
    True,
    False,
    Null,
  };

  Tag tag;
  union {
    int32_t as_int;
    double as_float;
    Object *as_object;
  } value;

  ALWAYS_INLINE explicit Value(Tag t) { tag = t; }

public:
  ALWAYS_INLINE static Value new_true() { return Value(Tag::True); }

  ALWAYS_INLINE static Value new_false() { return Value(Tag::False); }

  ALWAYS_INLINE static Value null() { return Value(Tag::Null); }

  ALWAYS_INLINE static Value empty() { return Value(Tag::Empty); }

  ALWAYS_INLINE explicit Value() { tag = Tag::Empty; }

#endif
public:
  ALWAYS_INLINE explicit Value(int32_t i);
  ALWAYS_INLINE explicit Value(double d);
  ALWAYS_INLINE explicit Value(Object *o);
  ALWAYS_INLINE explicit Value(bool b);
  ALWAYS_INLINE bool is_int() const;
  ALWAYS_INLINE int32_t as_int() const;
  ALWAYS_INLINE bool is_float() const;
  ALWAYS_INLINE double as_float() const;
  ALWAYS_INLINE bool is_null_or_false() const;
  ALWAYS_INLINE bool is_object() const;
  ALWAYS_INLINE Object *as_object() const;
  ALWAYS_INLINE bool is_null() const;
  ALWAYS_INLINE bool is_empty() const;
  ALWAYS_INLINE bool is_bool() const;
  ALWAYS_INLINE bool is_true() const;
  ALWAYS_INLINE bool is_false() const;
  ALWAYS_INLINE bool operator==(Value rhs) const;
  ALWAYS_INLINE const char *type_string() const;
  ALWAYS_INLINE void inc();
  friend std::ostream &operator<<(std::ostream &os, const Value v);
  friend struct ValueHasher;
  friend struct ValueStrictEquality;
};

struct ValueHasher {
  uint32_t operator()(Value v) const;
};

struct ValueStrictEquality {
  bool operator()(Value a, Value b) const;
};

class ValueFormatter {
  std::ostream &os;
  uint32_t depth;

public:
  ValueFormatter(std::ostream &os, uint32_t depth) : os(os), depth(depth) {}
  explicit ValueFormatter(std::ostream &os) : os(os), depth(0) {}
  friend void operator<<(ValueFormatter vf, Object *obj);
  friend void operator<<(ValueFormatter vf, Value v);
  ValueFormatter inc_depth() { return ValueFormatter(os, depth + 1); }
};
std::ostream &operator<<(std::ostream &os, Value v);
std::ostream &operator<<(std::ostream &os, Object &o);
} // namespace neptune_vm
